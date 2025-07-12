use crate::inventory::error::VariableError;
use crate::types::inventory::{InventoryGroup, ParsedInventory};
use std::collections::HashMap;

pub struct VariableResolver;

impl VariableResolver {
    pub fn new() -> Self {
        Self
    }

    pub fn resolve_variables(&self, inventory: &mut ParsedInventory) -> Result<(), VariableError> {
        for host_name in inventory.hosts.keys().cloned().collect::<Vec<_>>() {
            let mut resolved_vars = inventory.global_vars.clone();

            // Collect variables from all groups (in order)
            let host =
                inventory
                    .hosts
                    .get(&host_name)
                    .ok_or_else(|| VariableError::InvalidHost {
                        host: host_name.clone(),
                    })?;
            for group_name in &host.groups {
                if let Some(group) = inventory.groups.get(group_name) {
                    // Recursively resolve parent group variables
                    Self::resolve_group_variables(group, &inventory.groups, &mut resolved_vars)?;

                    // Apply group variables
                    for (key, value) in &group.variables {
                        resolved_vars.insert(key.clone(), value.clone());
                    }
                }
            }

            // Apply host-specific variables (highest priority)
            for (key, value) in &host.variables {
                resolved_vars.insert(key.clone(), value.clone());
            }

            // Update host with resolved variables
            inventory
                .hosts
                .get_mut(&host_name)
                .ok_or_else(|| VariableError::InvalidHost {
                    host: host_name.clone(),
                })?
                .variables = resolved_vars;
        }

        Ok(())
    }

    fn resolve_group_variables(
        group: &InventoryGroup,
        all_groups: &HashMap<String, InventoryGroup>,
        vars: &mut HashMap<String, serde_json::Value>,
    ) -> Result<(), VariableError> {
        // Recursively resolve parent group variables first
        for parent_name in &group.parent_groups {
            if let Some(parent_group) = all_groups.get(parent_name) {
                Self::resolve_group_variables(parent_group, all_groups, vars)?;
                for (key, value) in &parent_group.variables {
                    vars.insert(key.clone(), value.clone());
                }
            }
        }

        Ok(())
    }

    pub fn validate_no_circular_dependencies(
        &self,
        groups: &HashMap<String, InventoryGroup>,
    ) -> Result<(), VariableError> {
        for group_name in groups.keys() {
            let mut visited = std::collections::HashSet::new();
            let mut path = Vec::new();
            Self::check_circular_group_deps(group_name, groups, &mut visited, &mut path)?;
        }
        Ok(())
    }

    fn check_circular_group_deps(
        group_name: &str,
        groups: &HashMap<String, InventoryGroup>,
        visited: &mut std::collections::HashSet<String>,
        path: &mut Vec<String>,
    ) -> Result<(), VariableError> {
        if path.contains(&group_name.to_string()) {
            let cycle_start = path.iter().position(|g| g == group_name).ok_or_else(|| {
                VariableError::InternalError {
                    message: format!(
                        "Group {group_name} not found in path during circular dependency check"
                    ),
                }
            })?;
            let cycle = path[cycle_start..].to_vec();
            return Err(VariableError::CircularDependency { cycle });
        }

        if visited.contains(group_name) {
            return Ok(());
        }

        visited.insert(group_name.to_string());
        path.push(group_name.to_string());

        if let Some(group) = groups.get(group_name) {
            for parent in &group.parent_groups {
                Self::check_circular_group_deps(parent, groups, visited, path)?;
            }
        }

        path.pop();
        Ok(())
    }
}

impl Default for VariableResolver {
    fn default() -> Self {
        Self::new()
    }
}
