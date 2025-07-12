use crate::compilation::cache::CompilationCache;
use crate::compilation::capabilities::{CompilationCapabilities, CompilationStrategy};
use crate::compilation::optimizer::{DeploymentOptimizer, DeploymentPlan};
use crate::compilation::zigbuild::{CompiledBinary, ZigBuildCompiler};
use crate::deploy::{DeployError, Result};
use crate::template::GeneratedTemplate;
use crate::types::compilation::OptimizationLevel;
use crate::types::compilation::TargetSpecification;
use crate::ParsedInventory;
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{debug, info, warn};
use uuid;

/// Zero-infrastructure cross-compilation manager
pub struct ZeroInfraCompiler {
    capabilities: CompilationCapabilities,
    #[allow(dead_code)]
    cache: CompilationCache,
    optimizer: DeploymentOptimizer,
    zigbuild_compiler: Option<ZigBuildCompiler>,
}

#[derive(Debug, Clone)]
pub struct CompilationError {
    pub message: String,
    pub target: Option<String>,
    pub recoverable: bool,
}

impl std::fmt::Display for CompilationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Compilation error: {}", self.message)
    }
}

impl std::error::Error for CompilationError {}

impl From<CompilationError> for DeployError {
    fn from(err: CompilationError) -> Self {
        DeployError::compilation(err.message)
    }
}

impl ZeroInfraCompiler {
    /// Detect capabilities and create new compiler instance
    pub async fn detect_capabilities(cache_dir: PathBuf) -> Result<Self> {
        info!("Detecting zero-infrastructure compilation capabilities");

        let capabilities = CompilationCapabilities::detect_full().await?;
        let cache = CompilationCache::new(cache_dir.join("compilation"), true);
        let optimizer = DeploymentOptimizer::new();

        // Initialize ZigBuild compiler if available
        let zigbuild_compiler = if capabilities.zigbuild_available {
            match ZigBuildCompiler::new(cache_dir.join("zigbuild")).await {
                Ok(compiler) => {
                    info!("ZigBuild compiler initialized successfully");
                    Some(compiler)
                }
                Err(e) => {
                    warn!("Failed to initialize ZigBuild compiler: {}", e);
                    None
                }
            }
        } else {
            debug!("ZigBuild not available, using standard cross-compilation");
            None
        };

        Ok(Self {
            capabilities,
            cache,
            optimizer,
            zigbuild_compiler,
        })
    }

    /// Compile or fallback to SSH deployment
    pub async fn compile_or_fallback(
        &self,
        template: &GeneratedTemplate,
        inventory: &ParsedInventory,
    ) -> Result<DeploymentPlan> {
        info!("Creating deployment plan with zero-infrastructure compilation");

        // Parse execution plan from embedded data
        let execution_plan: crate::execution::RustlePlanOutput =
            serde_json::from_str(&template.embedded_data.execution_plan).map_err(|e| {
                DeployError::Configuration(format!("Failed to parse execution plan: {e}"))
            })?;

        // Analyze optimization potential
        let analysis = self
            .optimizer
            .analyze_optimization_potential(&execution_plan, &self.capabilities, inventory)
            .await?;

        debug!("Optimization analysis: {:?}", analysis);

        if analysis.optimization_score < 0.3 {
            info!("Low optimization potential, using SSH-only deployment");
            return self.create_ssh_only_plan(template, inventory).await;
        }

        // Create optimal deployment plan
        let mut deployment_plan = DeploymentPlan::new();

        // Group hosts by target architecture
        let target_groups = self.group_hosts_by_target(inventory).await?;

        for (target_triple, hosts) in target_groups {
            match self
                .compile_for_target(template, &target_triple, &hosts)
                .await
            {
                Ok(binary_deployment) => {
                    deployment_plan.binary_deployments.push(binary_deployment);
                }
                Err(e) => {
                    warn!("Failed to compile for target {}: {}", target_triple, e);
                    // Add SSH fallback for this target
                    deployment_plan.ssh_deployments.push(
                        self.create_ssh_deployment_for_hosts(
                            template,
                            &hosts,
                            FallbackReason::CompilationFailure,
                        )
                        .await?,
                    );
                }
            }
        }

        // Calculate performance metrics
        deployment_plan.estimated_performance_gain =
            self.estimate_performance_gain(&deployment_plan);
        deployment_plan.total_targets = inventory.hosts.len();

        info!(
            "Deployment plan created: {} binary deployments, {} SSH fallbacks",
            deployment_plan.binary_deployments.len(),
            deployment_plan.ssh_deployments.len()
        );

        Ok(deployment_plan)
    }

    /// Compile binary using ZigBuild for specific target
    pub async fn compile_with_zigbuild(
        &self,
        _template: &GeneratedTemplate,
        target: &TargetSpecification,
    ) -> Result<CompiledBinary> {
        let zigbuild_compiler =
            self.zigbuild_compiler
                .as_ref()
                .ok_or_else(|| CompilationError {
                    message: "ZigBuild compiler not available".to_string(),
                    target: Some(target.target_triple.clone()),
                    recoverable: false,
                })?;

        // TODO: Check cache first (temporarily disabled due to type mismatch)
        // Need to convert between compiler::CompiledBinary and zigbuild::CompiledBinary

        // Generate template specifically for this target
        // For now, use a temporary directory for template preparation
        let template_dir =
            std::env::temp_dir().join(format!("rustle-template-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&template_dir).await?;

        // Compile with appropriate optimization level
        // Default to release optimization for now
        let optimization = OptimizationLevel::Release;

        let binary = zigbuild_compiler
            .compile_with_zigbuild(&template_dir, target, optimization)
            .await?;

        // Cache the compiled binary
        // Store binary in cache - simplified for now
        // self.cache.store_binary(&binary).await?;

        Ok(binary)
    }

    /// Get available compilation targets
    pub fn get_available_targets(&self) -> &std::collections::HashSet<String> {
        &self.capabilities.available_targets
    }

    /// Check if target requires fallback to SSH
    pub fn requires_fallback(&self, target: &str) -> bool {
        !self.capabilities.supports_target(target)
            || matches!(
                self.capabilities.get_strategy_for_target(target),
                CompilationStrategy::SshFallback
            )
    }

    /// Validate toolchain installation
    pub async fn validate_toolchain(&self) -> Result<ValidationResult> {
        info!("Validating zero-infrastructure toolchain");

        let mut result = ValidationResult {
            overall_status: ValidationStatus::Failed,
            rust_status: ComponentStatus::Missing,
            zig_status: ComponentStatus::Missing,
            zigbuild_status: ComponentStatus::Missing,
            issues: Vec::new(),
            recommendations: Vec::new(),
        };

        // Validate Rust
        match crate::compilation::capabilities::detect_rust_installation().await {
            Ok(rust_install) => {
                result.rust_status = ComponentStatus::Available {
                    version: rust_install.version,
                };
            }
            Err(e) => {
                result.issues.push(format!("Rust validation failed: {e}"));
                result
                    .recommendations
                    .push("Install Rust toolchain".to_string());
            }
        }

        // Validate Zig
        match crate::compilation::capabilities::detect_zig_installation().await {
            Ok(Some(zig_install)) => {
                result.zig_status = ComponentStatus::Available {
                    version: zig_install.version,
                };
            }
            Ok(None) => {
                result.zig_status = ComponentStatus::Missing;
                result
                    .recommendations
                    .push("Install Zig for enhanced cross-compilation".to_string());
            }
            Err(e) => {
                result.issues.push(format!("Zig validation failed: {e}"));
            }
        }

        // Validate cargo-zigbuild
        match crate::compilation::capabilities::is_zigbuild_available().await {
            Ok(true) => {
                result.zigbuild_status = ComponentStatus::Available {
                    version: "unknown".to_string(),
                };
            }
            Ok(false) => {
                result.zigbuild_status = ComponentStatus::Missing;
                if result.zig_status != ComponentStatus::Missing {
                    result
                        .recommendations
                        .push("Install cargo-zigbuild".to_string());
                }
            }
            Err(e) => {
                result
                    .issues
                    .push(format!("cargo-zigbuild validation failed: {e}"));
            }
        }

        // Determine overall status
        result.overall_status = match (
            &result.rust_status,
            &result.zig_status,
            &result.zigbuild_status,
        ) {
            (
                ComponentStatus::Available { .. },
                ComponentStatus::Available { .. },
                ComponentStatus::Available { .. },
            ) => ValidationStatus::Excellent,
            (ComponentStatus::Available { .. }, _, ComponentStatus::Available { .. }) => {
                ValidationStatus::Good
            }
            (ComponentStatus::Available { .. }, _, _) => ValidationStatus::Minimal,
            _ => ValidationStatus::Failed,
        };

        Ok(result)
    }

    // Private helper methods

    async fn group_hosts_by_target(
        &self,
        inventory: &ParsedInventory,
    ) -> Result<HashMap<String, Vec<String>>> {
        let mut target_groups: HashMap<String, Vec<String>> = HashMap::new();

        for host in &inventory.hosts {
            // Detect target architecture for this host
            let target_triple = self.detect_target_for_host(host.0).await?;

            target_groups
                .entry(target_triple)
                .or_default()
                .push(host.0.clone());
        }

        Ok(target_groups)
    }

    async fn detect_target_for_host(&self, _host: &str) -> Result<String> {
        // Simplified target detection - in real implementation this would
        // query the host for its architecture and OS
        // For now, default to Linux x86_64
        Ok("x86_64-unknown-linux-gnu".to_string())
    }

    async fn compile_for_target(
        &self,
        template: &GeneratedTemplate,
        target_triple: &str,
        hosts: &[String],
    ) -> Result<BinaryDeployment> {
        let mut target_spec = TargetSpecification::new(target_triple);
        target_spec.requires_zig = self.capabilities.get_strategy_for_target(target_triple)
            == CompilationStrategy::ZigBuild;
        target_spec.compilation_strategy = self
            .capabilities
            .get_strategy_for_target(target_triple)
            .into();

        let binary = self.compile_with_zigbuild(template, &target_spec).await?;

        Ok(BinaryDeployment {
            binary,
            target_hosts: hosts.to_vec(),
            deployment_method: BinaryDeploymentMethod::UploadAndExecute,
        })
    }

    async fn create_ssh_only_plan(
        &self,
        template: &GeneratedTemplate,
        inventory: &ParsedInventory,
    ) -> Result<DeploymentPlan> {
        let mut plan = DeploymentPlan::new();

        let ssh_deployment = self
            .create_ssh_deployment_for_hosts(
                template,
                &inventory
                    .hosts
                    .iter()
                    .map(|h| h.0.clone())
                    .collect::<Vec<_>>(),
                FallbackReason::UserPreference,
            )
            .await?;

        plan.ssh_deployments.push(ssh_deployment);
        plan.total_targets = inventory.hosts.len();
        plan.estimated_performance_gain = 0.0; // No optimization

        Ok(plan)
    }

    async fn create_ssh_deployment_for_hosts(
        &self,
        template: &GeneratedTemplate,
        hosts: &[String],
        reason: FallbackReason,
    ) -> Result<SshDeployment> {
        Ok(SshDeployment {
            execution_plan: template.embedded_data.execution_plan.clone(),
            target_hosts: hosts.to_vec(),
            fallback_reason: reason,
        })
    }

    fn estimate_performance_gain(&self, plan: &DeploymentPlan) -> f32 {
        let total_deployments = plan.binary_deployments.len() + plan.ssh_deployments.len();
        if total_deployments == 0 {
            return 0.0;
        }

        let binary_ratio = plan.binary_deployments.len() as f32 / total_deployments as f32;

        // Estimate 2-10x performance gain for binary deployments
        // This is a simplified calculation based on the ratio of binary vs SSH deployments
        binary_ratio * 5.0 // Average 5x speedup for binary deployments
    }
}

// Supporting types from the specification

#[derive(Debug, Clone)]
pub struct BinaryDeployment {
    pub binary: CompiledBinary,
    pub target_hosts: Vec<String>,
    pub deployment_method: BinaryDeploymentMethod,
}

#[derive(Debug, Clone)]
pub enum BinaryDeploymentMethod {
    DirectExecution,
    UploadAndExecute,
    CachedExecution,
}

#[derive(Debug, Clone)]
pub struct SshDeployment {
    pub execution_plan: String,
    pub target_hosts: Vec<String>,
    pub fallback_reason: FallbackReason,
}

#[derive(Debug, Clone)]
pub enum FallbackReason {
    UnsupportedTarget,
    CompilationFailure,
    ModuleIncompatibility,
    UserPreference,
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub overall_status: ValidationStatus,
    pub rust_status: ComponentStatus,
    pub zig_status: ComponentStatus,
    pub zigbuild_status: ComponentStatus,
    pub issues: Vec<String>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationStatus {
    Excellent, // All components working perfectly
    Good,      // Core functionality available
    Minimal,   // Basic compilation only
    Failed,    // Missing essential components
}

#[derive(Debug, Clone, PartialEq)]
pub enum ComponentStatus {
    Available { version: String },
    Missing,
    Error { message: String },
}
