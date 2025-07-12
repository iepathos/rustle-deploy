use crate::deploy::{BinaryCompiler, BinaryDeployer, CompilationCache, DeployError, Result};
use crate::execution::{ExecutionPlan, ExecutionPlanParser, PlanFormat};
use crate::types::*;
use chrono::Utc;
use tracing::{debug, info, warn};
use uuid::Uuid;

pub struct DeploymentManager {
    config: DeploymentConfig,
    compiler: BinaryCompiler,
    deployer: BinaryDeployer,
    cache: CompilationCache,
    parser: ExecutionPlanParser,
}

impl DeploymentManager {
    pub fn new(config: DeploymentConfig) -> Self {
        let cache = CompilationCache::new(config.cache_dir.clone());
        let compiler = BinaryCompiler::new(cache.clone());
        let deployer = BinaryDeployer::new();
        let parser = ExecutionPlanParser::new();

        Self {
            config,
            compiler,
            deployer,
            cache,
            parser,
        }
    }

    pub async fn create_deployment_plan_from_execution(
        &self,
        execution_plan: &ExecutionPlan,
        targets: &[DeploymentTarget],
    ) -> Result<DeploymentPlan> {
        info!("Creating deployment plan");

        let execution_plan_hash = self.calculate_hash_from_plan(execution_plan);
        let deployment_id = Uuid::new_v4().to_string();

        // Create binary compilations for each unique target architecture
        let binary_compilations =
            self.create_binary_compilations_from_plan(execution_plan, targets, &deployment_id)?;

        let deployment_plan = DeploymentPlan {
            metadata: DeploymentMetadata {
                deployment_id: deployment_id.clone(),
                created_at: Utc::now(),
                rustle_version: env!("CARGO_PKG_VERSION").to_string(),
                execution_plan_hash,
                compiler_version: self.get_compiler_version(),
            },
            binary_compilations,
            deployment_targets: targets.to_vec(),
            deployment_strategy: DeploymentStrategy::Parallel, // Default strategy
            rollback_info: None,
        };

        debug!(
            "Created deployment plan with {} targets",
            deployment_plan.deployment_targets.len()
        );
        Ok(deployment_plan)
    }

    pub async fn create_deployment_plan(
        &self,
        execution_plan_content: &str,
        format: PlanFormat,
    ) -> Result<DeploymentPlan> {
        // Parse the execution plan
        let execution_plan = self
            .parser
            .parse(execution_plan_content, format)
            .map_err(|e| {
                DeployError::Configuration(format!("Failed to parse execution plan: {e}"))
            })?;

        // Extract deployment targets from the execution plan
        let targets = self
            .parser
            .extract_deployment_targets(&execution_plan)
            .map_err(|e| {
                DeployError::Configuration(format!("Failed to extract deployment targets: {e}"))
            })?;

        // Create deployment plan using the structured data
        self.create_deployment_plan_from_execution(&execution_plan, &targets)
            .await
    }

    pub async fn compile_binaries(&self, plan: &DeploymentPlan) -> Result<Vec<BinaryCompilation>> {
        info!("Compiling {} binaries", plan.binary_compilations.len());

        let mut compiled_binaries = Vec::new();

        for compilation in &plan.binary_compilations {
            // Check cache first
            if !self.config.binary_size_limit_mb > 0 {
                if let Some(_cached) = self.cache.get_cached_binary(&compilation.checksum) {
                    info!("Using cached binary for {}", compilation.binary_name);
                    compiled_binaries.push(compilation.clone());
                    continue;
                }
            }

            info!("Compiling binary: {}", compilation.binary_name);
            let compiled = self.compiler.compile_binary(compilation).await?;

            // Validate binary size
            if self.config.binary_size_limit_mb > 0 {
                let size_mb = compiled.size / (1024 * 1024);
                if size_mb > self.config.binary_size_limit_mb {
                    return Err(DeployError::BinarySizeExceeded {
                        size: compiled.size,
                        limit: self.config.binary_size_limit_mb * 1024 * 1024,
                    });
                }
            }

            // Update compilation with actual results
            let mut updated_compilation = compilation.clone();
            updated_compilation.checksum = compiled.checksum;
            updated_compilation.size = compiled.size;

            compiled_binaries.push(updated_compilation);
        }

        info!("Successfully compiled {} binaries", compiled_binaries.len());
        Ok(compiled_binaries)
    }

    pub async fn deploy_binaries(&self, plan: &DeploymentPlan) -> Result<DeploymentReport> {
        info!(
            "Deploying binaries to {} targets",
            plan.deployment_targets.len()
        );

        let mut deployment_results = Vec::new();
        let mut successful_deployments = 0;
        let mut failed_deployments = 0;

        for target in &plan.deployment_targets {
            info!("Deploying to host: {}", target.host);

            // Find the corresponding binary compilation
            let compilation = plan
                .binary_compilations
                .iter()
                .find(|c| c.compilation_id == target.binary_compilation_id)
                .ok_or_else(|| {
                    DeployError::Configuration(format!(
                        "No compilation found for target {}",
                        target.host
                    ))
                })?;

            match self.deployer.deploy_to_host(compilation, target).await {
                Ok(_) => {
                    info!("Successfully deployed to {}", target.host);
                    successful_deployments += 1;

                    if self.config.verify_deployments {
                        match self.deployer.verify_deployment(target).await {
                            Ok(true) => {
                                info!("Deployment verification successful for {}", target.host);
                            }
                            Ok(false) => {
                                warn!("Deployment verification failed for {}", target.host);
                                failed_deployments += 1;
                            }
                            Err(e) => {
                                warn!("Deployment verification error for {}: {}", target.host, e);
                                failed_deployments += 1;
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to deploy to {}: {}", target.host, e);
                    failed_deployments += 1;
                }
            }

            deployment_results.push(DeploymentResult {
                host: target.host.clone(),
                status: if successful_deployments > failed_deployments {
                    DeploymentStatus::Deployed
                } else {
                    DeploymentStatus::Failed {
                        error: "Deployment failed".to_string(),
                    }
                },
                deployed_at: Some(Utc::now()),
            });
        }

        let report = DeploymentReport {
            deployment_id: plan.metadata.deployment_id.clone(),
            total_targets: plan.deployment_targets.len(),
            successful_deployments,
            failed_deployments,
            deployment_results,
            started_at: Utc::now(), // TODO: Track actual start time
            completed_at: Utc::now(),
        };

        info!(
            "Deployment completed: {}/{} successful",
            successful_deployments,
            plan.deployment_targets.len()
        );

        Ok(report)
    }

    pub async fn verify_deployments(
        &self,
        targets: &[DeploymentTarget],
    ) -> Result<VerificationReport> {
        info!("Verifying {} deployments", targets.len());

        let mut verification_results = Vec::new();
        let mut successful_verifications = 0;

        for target in targets {
            match self.deployer.verify_deployment(target).await {
                Ok(success) => {
                    if success {
                        successful_verifications += 1;
                    }
                    verification_results.push(VerificationResult {
                        host: target.host.clone(),
                        success,
                        error: None,
                    });
                }
                Err(e) => {
                    verification_results.push(VerificationResult {
                        host: target.host.clone(),
                        success: false,
                        error: Some(e.to_string()),
                    });
                }
            }
        }

        Ok(VerificationReport {
            total_targets: targets.len(),
            successful_verifications,
            verification_results,
        })
    }

    pub async fn cleanup_deployments(&self, targets: &[DeploymentTarget]) -> Result<()> {
        info!("Cleaning up deployments on {} targets", targets.len());

        for target in targets {
            if let Err(e) = self.deployer.cleanup_deployment(target).await {
                warn!("Failed to cleanup deployment on {}: {}", target.host, e);
            }
        }

        Ok(())
    }

    pub async fn rollback_deployment(&self, deployment_id: &str) -> Result<()> {
        info!("Rolling back deployment: {}", deployment_id);
        // TODO: Implement rollback logic
        // This would involve reading previous deployment state and reverting
        Err(DeployError::RollbackFailed {
            deployment_id: deployment_id.to_string(),
            reason: "Rollback functionality not yet implemented".to_string(),
        })
    }

    // Helper methods

    fn calculate_hash_from_plan(&self, plan: &ExecutionPlan) -> String {
        use sha2::{Digest, Sha256};
        let serialized = serde_json::to_string(plan).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(serialized.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn get_compiler_version(&self) -> String {
        // TODO: Get actual rustc version
        "rustc 1.70.0".to_string()
    }

    fn create_binary_compilations_from_plan(
        &self,
        execution_plan: &ExecutionPlan,
        targets: &[DeploymentTarget],
        deployment_id: &str,
    ) -> Result<Vec<BinaryCompilation>> {
        // Group targets by architecture to minimize compilation work
        let mut compilations = Vec::new();
        let mut processed_targets = std::collections::HashSet::new();

        for _target in targets {
            // TODO: Extract actual target triple from target or inventory
            let target_triple = "x86_64-unknown-linux-gnu".to_string();

            if processed_targets.contains(&target_triple) {
                continue;
            }
            processed_targets.insert(target_triple.clone());

            let compilation_id = format!("{deployment_id}-{target_triple}");

            let compilation = BinaryCompilation {
                compilation_id: compilation_id.clone(),
                binary_name: format!("rustle-runner-{target_triple}"),
                target_triple: target_triple.clone(),
                source_tasks: execution_plan
                    .tasks
                    .iter()
                    .map(|t| t.name.clone())
                    .collect(),
                embedded_data: EmbeddedExecutionData {
                    execution_plan: serde_json::to_string(execution_plan).unwrap_or_default(),
                    module_implementations: execution_plan
                        .modules
                        .iter()
                        .map(|m| ModuleImplementation {
                            module_name: m.name.clone(),
                            source_code: String::new(), // TODO: Load actual module source
                            dependencies: m.dependencies.clone(),
                            static_linked: m.static_link,
                        })
                        .collect(),
                    static_files: vec![],
                    runtime_config: crate::runtime::RuntimeConfig {
                        controller_endpoint: None,
                        execution_timeout: execution_plan
                            .deployment_config
                            .deployment_timeout
                            .unwrap_or(std::time::Duration::from_secs(3600)),
                        task_timeout: Some(std::time::Duration::from_secs(300)),
                        report_interval: std::time::Duration::from_secs(60),
                        cleanup_on_completion: execution_plan.deployment_config.cleanup_on_success,
                        log_level: "info".to_string(),
                        check_mode: Some(false),
                        parallel_tasks: Some(4),
                        facts_cache_ttl: std::time::Duration::from_secs(300),
                        retry_policy: None,
                        verbose: false,
                    },
                    facts_template: execution_plan.facts_template.global_facts.clone(),
                },
                compilation_options: LegacyCompilationOptions {
                    optimization_level: OptimizationLevel::Release,
                    strip_symbols: self.config.strip_symbols,
                    static_linking: true,
                    compression: self.config.compression,
                    custom_features: vec![],
                    target_cpu: None,
                },
                output_path: self
                    .config
                    .output_dir
                    .join(format!("rustle-runner-{target_triple}")),
                checksum: String::new(), // Will be calculated during compilation
                size: 0,                 // Will be set during compilation
            };

            compilations.push(compilation);
        }

        Ok(compilations)
    }
}

// Supporting types for reports
#[derive(Debug)]
pub struct DeploymentReport {
    pub deployment_id: String,
    pub total_targets: usize,
    pub successful_deployments: usize,
    pub failed_deployments: usize,
    pub deployment_results: Vec<DeploymentResult>,
    pub started_at: chrono::DateTime<Utc>,
    pub completed_at: chrono::DateTime<Utc>,
}

#[derive(Debug)]
pub struct DeploymentResult {
    pub host: String,
    pub status: DeploymentStatus,
    pub deployed_at: Option<chrono::DateTime<Utc>>,
}

#[derive(Debug)]
pub struct VerificationReport {
    pub total_targets: usize,
    pub successful_verifications: usize,
    pub verification_results: Vec<VerificationResult>,
}

#[derive(Debug)]
pub struct VerificationResult {
    pub host: String,
    pub success: bool,
    pub error: Option<String>,
}
