use crate::inventory::error::ValidationError;
use crate::types::inventory::{ConnectionMethod, ParsedInventory};

pub trait InventoryValidator {
    fn validate(&self, inventory: &ParsedInventory) -> Result<(), ValidationError>;
}

pub struct ConnectivityValidator;
pub struct ArchitectureValidator;
pub struct VariableValidator;

impl InventoryValidator for ConnectivityValidator {
    fn validate(&self, inventory: &ParsedInventory) -> Result<(), ValidationError> {
        for (host_name, host) in &inventory.hosts {
            // Validate connection configuration
            match &host.connection.method {
                ConnectionMethod::Ssh => {
                    if host.connection.host.is_none() && host.address.is_none() {
                        return Err(ValidationError::InvalidConnection {
                            host: host_name.clone(),
                        });
                    }
                }
                ConnectionMethod::WinRm => {
                    if host.connection.host.is_none() && host.address.is_none() {
                        return Err(ValidationError::InvalidConnection {
                            host: host_name.clone(),
                        });
                    }
                }
                ConnectionMethod::Local => {
                    // Local connections don't need additional validation
                }
                _ => {
                    // Other connection methods might need specific validation
                }
            }
        }
        Ok(())
    }
}

impl InventoryValidator for ArchitectureValidator {
    fn validate(&self, inventory: &ParsedInventory) -> Result<(), ValidationError> {
        for (host_name, host) in &inventory.hosts {
            // Check if we have some way to determine the target architecture
            let has_target_triple = host.target_triple.is_some();
            let has_arch_info = host.architecture.is_some() && host.operating_system.is_some();
            let has_ansible_facts = host.variables.contains_key("ansible_architecture")
                && host.variables.contains_key("ansible_os_family");

            if !has_target_triple && !has_arch_info && !has_ansible_facts {
                // For local connections, we can detect automatically
                if !matches!(host.connection.method, ConnectionMethod::Local) {
                    return Err(ValidationError::InvalidConnection {
                        host: format!("{host_name} (missing architecture information)"),
                    });
                }
            }
        }
        Ok(())
    }
}

impl InventoryValidator for VariableValidator {
    fn validate(&self, inventory: &ParsedInventory) -> Result<(), ValidationError> {
        // Check for duplicate host names
        let mut seen_hosts = std::collections::HashSet::new();
        for host_name in inventory.hosts.keys() {
            if !seen_hosts.insert(host_name) {
                return Err(ValidationError::DuplicateHost {
                    host: host_name.clone(),
                });
            }
        }

        // Check for missing groups
        for (host_name, host) in &inventory.hosts {
            for group_name in &host.groups {
                if !inventory.groups.contains_key(group_name) {
                    return Err(ValidationError::MissingGroup {
                        group: format!("{group_name} (referenced by host {host_name})"),
                    });
                }
            }
        }

        // Check for circular group dependencies
        for group_name in inventory.groups.keys() {
            if Self::has_circular_dependency(
                group_name,
                group_name,
                &inventory.groups,
                &mut std::collections::HashSet::new(),
            ) {
                return Err(ValidationError::CircularGroupDependency {
                    cycle: vec![group_name.clone()],
                });
            }
        }

        Ok(())
    }
}

impl VariableValidator {
    fn has_circular_dependency(
        original_group: &str,
        current_group: &str,
        groups: &std::collections::HashMap<String, crate::types::inventory::InventoryGroup>,
        visited: &mut std::collections::HashSet<String>,
    ) -> bool {
        if visited.contains(current_group) {
            return current_group == original_group;
        }

        visited.insert(current_group.to_string());

        if let Some(group) = groups.get(current_group) {
            for parent in &group.parent_groups {
                if Self::has_circular_dependency(original_group, parent, groups, visited) {
                    return true;
                }
            }
        }

        visited.remove(current_group);
        false
    }
}

pub struct InventoryValidatorSet {
    validators: Vec<Box<dyn InventoryValidator>>,
}

impl InventoryValidatorSet {
    pub fn new() -> Self {
        Self {
            validators: vec![
                Box::new(ConnectivityValidator),
                Box::new(ArchitectureValidator),
                Box::new(VariableValidator),
            ],
        }
    }

    pub fn validate(&self, inventory: &ParsedInventory) -> Result<(), ValidationError> {
        for validator in &self.validators {
            validator.validate(inventory)?;
        }
        Ok(())
    }
}

impl Default for InventoryValidatorSet {
    fn default() -> Self {
        Self::new()
    }
}
