use crate::execution::plan::ExecutionPlan;
use crate::inventory::error::InventoryError;
use crate::types::inventory::{
    ConnectionConfig, ConnectionMethod, InventoryFormat, InventoryGroup, InventoryHost,
    InventoryMetadata, ParsedInventory,
};
use chrono::Utc;
use std::collections::HashMap;

/// JSON inventory processor (rustle-plan output)
pub struct JsonInventoryProcessor;

impl JsonInventoryProcessor {
    pub fn new() -> Self {
        Self
    }

    pub fn process_from_plan_output(
        &self,
        plan_output: &serde_json::Value,
    ) -> Result<ParsedInventory, InventoryError> {
        // Try to parse as full ExecutionPlan first
        if let Ok(execution_plan) = serde_json::from_value::<ExecutionPlan>(plan_output.clone()) {
            return self.process_from_execution_plan(&execution_plan);
        }

        // Try to extract inventory section
        let inventory_section = self.extract_inventory_section(plan_output)?;
        self.process_inventory_json(&inventory_section)
    }

    pub fn process_from_execution_plan(
        &self,
        execution_plan: &ExecutionPlan,
    ) -> Result<ParsedInventory, InventoryError> {
        let inventory_spec = &execution_plan.inventory;

        let mut hosts = HashMap::new();
        let mut groups = HashMap::new();

        // Process hosts from inventory spec
        for (host_id, host_spec) in &inventory_spec.hosts {
            let inventory_host = InventoryHost {
                name: host_id.clone(),
                address: Some(host_spec.address.clone()),
                connection: ConnectionConfig {
                    method: match host_spec.connection.method {
                        crate::execution::plan::ConnectionMethod::Ssh => ConnectionMethod::Ssh,
                        crate::execution::plan::ConnectionMethod::WinRm => ConnectionMethod::WinRm,
                        crate::execution::plan::ConnectionMethod::Local => ConnectionMethod::Local,
                    },
                    host: Some(host_spec.address.clone()),
                    port: host_spec.connection.port,
                    username: host_spec.connection.username.clone(),
                    password: host_spec.connection.password.clone(),
                    private_key: None,
                    private_key_file: host_spec.connection.key_file.clone(),
                    timeout: host_spec.connection.timeout,
                    ssh_args: None,
                    winrm_transport: None,
                },
                variables: host_spec.variables.clone(),
                groups: Vec::new(), // Will be populated below
                target_triple: host_spec.target_triple.clone(),
                architecture: None,
                operating_system: None,
                platform: None,
            };
            hosts.insert(host_id.clone(), inventory_host);
        }

        // Process groups from inventory spec
        for (group_id, group_spec) in &inventory_spec.groups {
            let inventory_group = InventoryGroup {
                name: group_id.clone(),
                hosts: group_spec.hosts.clone(),
                children: group_spec.children.clone(),
                variables: group_spec.variables.clone(),
                parent_groups: Vec::new(), // Will be computed later
            };
            groups.insert(group_id.clone(), inventory_group);
        }

        // Update host group memberships
        for (group_id, group) in &groups {
            for host_id in &group.hosts {
                if let Some(host) = hosts.get_mut(host_id) {
                    host.groups.push(group_id.clone());
                }
            }
        }

        // Compute parent group relationships
        for (group_id, group) in &groups.clone() {
            for child_id in &group.children {
                if let Some(child_group) = groups.get_mut(child_id) {
                    child_group.parent_groups.push(group_id.clone());
                }
            }
        }

        let metadata = InventoryMetadata {
            format: InventoryFormat::Json,
            source: "rustle-plan".to_string(),
            parsed_at: Utc::now(),
            host_count: hosts.len(),
            group_count: groups.len(),
        };

        Ok(ParsedInventory {
            hosts,
            groups,
            global_vars: inventory_spec.variables.clone(),
            metadata,
        })
    }

    pub fn extract_inventory_section(
        &self,
        plan_output: &serde_json::Value,
    ) -> Result<serde_json::Value, InventoryError> {
        // Try to find inventory in common locations
        if let Some(inventory) = plan_output.get("inventory") {
            return Ok(inventory.clone());
        }

        if let Some(inventory) = plan_output.get("hosts") {
            return Ok(inventory.clone());
        }

        // If the entire object looks like inventory data, use it
        if plan_output.get("all").is_some() || plan_output.get("_meta").is_some() {
            return Ok(plan_output.clone());
        }

        Err(InventoryError::InvalidJson {
            reason: "No inventory section found in plan output".to_string(),
        })
    }

    pub fn process_inventory_json(
        &self,
        inventory_data: &serde_json::Value,
    ) -> Result<ParsedInventory, InventoryError> {
        let mut hosts = HashMap::new();
        let mut groups = HashMap::new();
        let mut global_vars = HashMap::new();

        // Handle Ansible dynamic inventory format
        if let Some(meta) = inventory_data.get("_meta") {
            if let Some(hostvars) = meta.get("hostvars") {
                if let Some(hostvars_obj) = hostvars.as_object() {
                    for (host_name, host_vars) in hostvars_obj {
                        let host = self.create_host_from_vars(host_name, host_vars)?;
                        hosts.insert(host_name.clone(), host);
                    }
                }
            }
        }

        // Process groups
        if let Some(obj) = inventory_data.as_object() {
            for (key, value) in obj {
                if key == "_meta" {
                    continue;
                }

                if key == "all" {
                    if let Some(group_data) = value.as_object() {
                        if let Some(vars) = group_data.get("vars") {
                            if let Some(vars_obj) = vars.as_object() {
                                for (var_name, var_value) in vars_obj {
                                    global_vars.insert(var_name.clone(), var_value.clone());
                                }
                            }
                        }
                    }
                    continue;
                }

                // Process as group
                let group = self.create_group_from_data(key, value)?;
                groups.insert(key.clone(), group);

                // Add hosts from group if not already present
                if let Some(group_hosts) = value.get("hosts") {
                    if let Some(hosts_array) = group_hosts.as_array() {
                        for host_name_val in hosts_array {
                            if let Some(host_name) = host_name_val.as_str() {
                                if !hosts.contains_key(host_name) {
                                    let host = self.create_default_host(host_name);
                                    hosts.insert(host_name.to_string(), host);
                                }
                                // Add group membership
                                if let Some(host) = hosts.get_mut(host_name) {
                                    if !host.groups.contains(&key.to_string()) {
                                        host.groups.push(key.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let metadata = InventoryMetadata {
            format: InventoryFormat::Json,
            source: "json".to_string(),
            parsed_at: Utc::now(),
            host_count: hosts.len(),
            group_count: groups.len(),
        };

        Ok(ParsedInventory {
            hosts,
            groups,
            global_vars,
            metadata,
        })
    }

    fn create_host_from_vars(
        &self,
        host_name: &str,
        host_vars: &serde_json::Value,
    ) -> Result<InventoryHost, InventoryError> {
        let vars = if let Some(vars_obj) = host_vars.as_object() {
            vars_obj
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        } else {
            HashMap::new()
        };

        let address = vars
            .get("ansible_host")
            .or_else(|| vars.get("ansible_ssh_host"))
            .and_then(|v| v.as_str())
            .unwrap_or(host_name)
            .to_string();

        let connection_method = vars
            .get("ansible_connection")
            .and_then(|v| v.as_str())
            .map(|s| match s {
                "ssh" => ConnectionMethod::Ssh,
                "winrm" => ConnectionMethod::WinRm,
                "local" => ConnectionMethod::Local,
                _ => ConnectionMethod::Ssh,
            })
            .unwrap_or(ConnectionMethod::Ssh);

        let port = vars
            .get("ansible_port")
            .or_else(|| vars.get("ansible_ssh_port"))
            .and_then(|v| v.as_u64())
            .map(|p| p as u16);

        let username = vars
            .get("ansible_user")
            .or_else(|| vars.get("ansible_ssh_user"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let key_file = vars
            .get("ansible_ssh_private_key_file")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let target_triple = vars
            .get("target_triple")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let architecture = vars
            .get("ansible_architecture")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let operating_system = vars
            .get("ansible_os_family")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(InventoryHost {
            name: host_name.to_string(),
            address: Some(address),
            connection: ConnectionConfig {
                method: connection_method,
                host: Some(host_name.to_string()),
                port,
                username,
                password: None, // Don't store passwords in inventory
                private_key: None,
                private_key_file: key_file,
                timeout: None,
                ssh_args: None,
                winrm_transport: None,
            },
            variables: vars,
            groups: Vec::new(),
            target_triple,
            architecture,
            operating_system,
            platform: None,
        })
    }

    fn create_group_from_data(
        &self,
        group_name: &str,
        group_data: &serde_json::Value,
    ) -> Result<InventoryGroup, InventoryError> {
        let mut hosts = Vec::new();
        let mut children = Vec::new();
        let mut variables = HashMap::new();

        if let Some(obj) = group_data.as_object() {
            if let Some(hosts_val) = obj.get("hosts") {
                if let Some(hosts_array) = hosts_val.as_array() {
                    for host_val in hosts_array {
                        if let Some(host_name) = host_val.as_str() {
                            hosts.push(host_name.to_string());
                        }
                    }
                }
            }

            if let Some(children_val) = obj.get("children") {
                if let Some(children_array) = children_val.as_array() {
                    for child_val in children_array {
                        if let Some(child_name) = child_val.as_str() {
                            children.push(child_name.to_string());
                        }
                    }
                }
            }

            if let Some(vars_val) = obj.get("vars") {
                if let Some(vars_obj) = vars_val.as_object() {
                    for (var_name, var_value) in vars_obj {
                        variables.insert(var_name.clone(), var_value.clone());
                    }
                }
            }
        }

        Ok(InventoryGroup {
            name: group_name.to_string(),
            hosts,
            children,
            variables,
            parent_groups: Vec::new(),
        })
    }

    fn create_default_host(&self, host_name: &str) -> InventoryHost {
        InventoryHost {
            name: host_name.to_string(),
            address: Some(host_name.to_string()),
            connection: ConnectionConfig {
                method: ConnectionMethod::Ssh,
                host: Some(host_name.to_string()),
                port: None,
                username: None,
                password: None,
                private_key: None,
                private_key_file: None,
                timeout: None,
                ssh_args: None,
                winrm_transport: None,
            },
            variables: HashMap::new(),
            groups: Vec::new(),
            target_triple: None,
            architecture: None,
            operating_system: None,
            platform: None,
        }
    }
}

impl Default for JsonInventoryProcessor {
    fn default() -> Self {
        Self::new()
    }
}
