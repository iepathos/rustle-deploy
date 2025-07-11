use crate::inventory::{
    ArchitectureDetector, ConversionError, DetectionError, HostInfoProber, InventoryError,
    InventoryValidatorSet, JsonInventoryProcessor, ValidationError, VariableError,
    VariableResolver,
};
use crate::types::{DeploymentMethod, DeploymentStatus, DeploymentTarget, ParsedInventory};
use std::collections::HashMap;

pub struct InventoryProcessor {
    detector: ArchitectureDetector,
    validators: InventoryValidatorSet,
    variable_resolver: VariableResolver,
    host_prober: HostInfoProber,
    json_processor: JsonInventoryProcessor,
}

impl InventoryProcessor {
    pub fn new() -> Self {
        Self {
            detector: ArchitectureDetector::new(),
            validators: InventoryValidatorSet::new(),
            variable_resolver: VariableResolver::new(),
            host_prober: HostInfoProber::new(),
            json_processor: JsonInventoryProcessor::new(),
        }
    }

    pub fn process_from_plan(
        &self,
        plan_output: &serde_json::Value,
    ) -> Result<ParsedInventory, InventoryError> {
        let mut inventory = self.json_processor.process_from_plan_output(plan_output)?;
        self.process_inventory_data(&mut inventory)?;
        Ok(inventory)
    }

    pub fn process_inventory_data(
        &self,
        inventory: &mut ParsedInventory,
    ) -> Result<(), InventoryError> {
        // Validate the inventory structure
        self.validate(inventory)
            .map_err(|e| InventoryError::VariableResolution {
                variable: format!("Validation failed: {e}"),
            })?;

        // Resolve variables and inheritance
        self.resolve_variables(inventory)
            .map_err(|e| InventoryError::VariableResolution {
                variable: format!("Variable resolution failed: {e}"),
            })?;

        // Detect architectures for hosts
        self.detect_architectures(inventory).map_err(|e| {
            InventoryError::ArchitectureDetectionFailed {
                host: format!("Architecture detection failed: {e}"),
            }
        })?;

        Ok(())
    }

    pub fn validate(&self, inventory: &ParsedInventory) -> Result<(), ValidationError> {
        self.validators.validate(inventory)
    }

    pub fn resolve_variables(&self, inventory: &mut ParsedInventory) -> Result<(), VariableError> {
        self.variable_resolver.resolve_variables(inventory)
    }

    pub fn detect_architectures(
        &self,
        inventory: &mut ParsedInventory,
    ) -> Result<(), DetectionError> {
        for (host_name, host) in inventory.hosts.iter_mut() {
            // Only detect if not already specified
            if host.target_triple.is_none() {
                if let Some(triple) = self.detector.detect_target_triple(host) {
                    host.target_triple = Some(triple);
                } else {
                    return Err(DetectionError::DetectionFailed {
                        reason: format!("Could not detect target triple for host: {host_name}"),
                    });
                }
            }

            // Try to fill in architecture/os info if missing
            if host.architecture.is_none() || host.operating_system.is_none() {
                if let Ok(host_info) = self.host_prober.probe_host_info(host) {
                    if host.architecture.is_none() {
                        host.architecture = Some(host_info.architecture);
                    }
                    if host.operating_system.is_none() {
                        host.operating_system = Some(host_info.operating_system);
                    }
                    if host.platform.is_none() {
                        host.platform = Some(host_info.platform);
                    }
                }
            }
        }

        Ok(())
    }

    pub fn to_deployment_targets(
        &self,
        inventory: &ParsedInventory,
    ) -> Result<Vec<DeploymentTarget>, ConversionError> {
        let mut targets = Vec::new();

        for (host_name, host) in &inventory.hosts {
            let target_triple = host
                .target_triple
                .clone()
                .or_else(|| self.detector.detect_target_triple(host))
                .unwrap_or_else(|| "x86_64-unknown-linux-gnu".to_string());

            let deployment_method = match host.connection.method {
                crate::types::inventory::ConnectionMethod::Ssh => DeploymentMethod::Ssh,
                crate::types::inventory::ConnectionMethod::WinRm => DeploymentMethod::Custom {
                    command: format!(
                        "winrm copy {{binary_path}} {}/rustle-runner.exe",
                        host.connection.host.as_ref().unwrap_or(host_name)
                    ),
                },
                crate::types::inventory::ConnectionMethod::Local => DeploymentMethod::Scp,
                crate::types::inventory::ConnectionMethod::Docker => DeploymentMethod::Custom {
                    command: format!("docker cp {{binary_path}} {host_name}:/tmp/rustle-runner"),
                },
                crate::types::inventory::ConnectionMethod::Podman => DeploymentMethod::Custom {
                    command: format!("podman cp {{binary_path}} {host_name}:/tmp/rustle-runner"),
                },
            };

            let target_path = self.determine_target_path(&target_triple, &host.variables);

            // Use the host address or connection host, fallback to host name
            let deployment_host = host
                .address
                .clone()
                .or_else(|| host.connection.host.clone())
                .unwrap_or_else(|| host_name.clone());

            targets.push(DeploymentTarget {
                host: deployment_host,
                target_path,
                binary_compilation_id: format!("rustle-{target_triple}"),
                deployment_method,
                status: DeploymentStatus::Pending,
                deployed_at: None,
                version: "1.0.0".to_string(),
            });
        }

        Ok(targets)
    }

    fn determine_target_path(
        &self,
        target_triple: &str,
        variables: &HashMap<String, serde_json::Value>,
    ) -> String {
        // Check for explicit target path in variables
        if let Some(path_val) = variables.get("rustle_target_path") {
            if let Some(path_str) = path_val.as_str() {
                return path_str.to_string();
            }
        }

        // Check for ansible-style paths
        if let Some(path_val) = variables.get("ansible_remote_tmp") {
            if let Some(path_str) = path_val.as_str() {
                return format!("{path_str}/rustle-runner");
            }
        }

        // Default based on target platform
        match target_triple.contains("windows") {
            true => "C:\\temp\\rustle-runner.exe".to_string(),
            false => "/tmp/rustle-runner".to_string(),
        }
    }

    pub fn probe_host_info(
        &self,
        host: &crate::types::inventory::InventoryHost,
    ) -> Result<crate::types::inventory::HostInfo, crate::inventory::error::ProbeError> {
        self.host_prober.probe_host_info(host)
    }
}

impl Default for InventoryProcessor {
    fn default() -> Self {
        Self::new()
    }
}
