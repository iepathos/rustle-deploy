use crate::execution::{Host, HostGroup, InventorySpec, ValidationError};
use std::collections::HashMap;

pub struct InventoryParser;

impl InventoryParser {
    pub fn new() -> Self {
        Self
    }

    pub fn parse_inventory(
        &self,
        inventory: &InventorySpec,
    ) -> Result<ParsedInventory, ValidationError> {
        let mut parsed = ParsedInventory {
            hosts: HashMap::new(),
            groups: HashMap::new(),
        };

        // Parse hosts
        for (host_id, host) in &inventory.hosts {
            parsed.hosts.insert(host_id.clone(), host.clone());
        }

        // Parse groups
        for (group_id, group) in &inventory.groups {
            parsed.groups.insert(group_id.clone(), group.clone());
        }

        // Validate that all group children exist
        for (group_id, group) in &parsed.groups {
            for child in &group.children {
                if !parsed.groups.contains_key(child) {
                    return Err(ValidationError::InvalidInventory {
                        reason: format!(
                            "Group '{group_id}' references non-existent child group '{child}'"
                        ),
                    });
                }
            }
        }

        // Validate that all group hosts exist
        for (group_id, group) in &parsed.groups {
            for host_id in &group.hosts {
                if !parsed.hosts.contains_key(host_id) {
                    return Err(ValidationError::InvalidInventory {
                        reason: format!(
                            "Group '{group_id}' references non-existent host '{host_id}'"
                        ),
                    });
                }
            }
        }

        Ok(parsed)
    }

    pub fn resolve_host_groups(&self, inventory: &ParsedInventory, host_id: &str) -> Vec<String> {
        let mut groups = Vec::new();

        for (group_id, group) in &inventory.groups {
            if group.hosts.contains(&host_id.to_string()) {
                groups.push(group_id.clone());
            }
        }

        groups
    }

    pub fn get_host_variables(
        &self,
        inventory: &ParsedInventory,
        host_id: &str,
    ) -> HashMap<String, serde_json::Value> {
        let mut variables = HashMap::new();

        // Add global inventory variables
        variables.extend(
            inventory
                .groups
                .get("all")
                .map(|g| &g.variables)
                .unwrap_or(&HashMap::new())
                .clone(),
        );

        // Add group variables (in order of group hierarchy)
        let host_groups = self.resolve_host_groups(inventory, host_id);
        for group_id in &host_groups {
            if let Some(group) = inventory.groups.get(group_id) {
                variables.extend(group.variables.clone());
            }
        }

        // Add host-specific variables (highest priority)
        if let Some(host) = inventory.hosts.get(host_id) {
            variables.extend(host.variables.clone());
        }

        variables
    }
}

impl Default for InventoryParser {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ParsedInventory {
    pub hosts: HashMap<String, Host>,
    pub groups: HashMap<String, HostGroup>,
}
