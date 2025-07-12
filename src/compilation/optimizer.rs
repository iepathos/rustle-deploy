use crate::compilation::capabilities::CompilationCapabilities;
use crate::compilation::zero_infra::{BinaryDeployment, SshDeployment};
use crate::deploy::{DeployError, Result};
use crate::execution::{RustlePlanOutput, TaskPlan};
use crate::ParsedInventory;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, info};

/// Analyzes execution plans and determines optimal deployment strategies
pub struct DeploymentOptimizer {
    binary_analyzer: BinaryDeploymentAnalyzer,
    performance_predictor: PerformancePredictor,
}

#[derive(Debug, Clone)]
pub struct OptimizationAnalysis {
    pub optimization_score: f32,           // 0.0 to 1.0
    pub binary_compatible_tasks: usize,
    pub total_tasks: usize,
    pub estimated_speedup: f32,
    pub compilation_overhead: Duration,
    pub recommended_strategy: RecommendedStrategy,
    pub target_breakdown: HashMap<String, TargetAnalysis>,
}

#[derive(Debug, Clone)]
pub enum RecommendedStrategy {
    BinaryOnly,
    Hybrid,
    SshOnly,
}

#[derive(Debug, Clone)]
pub struct TargetAnalysis {
    pub target_triple: String,
    pub host_count: usize,
    pub compatible_tasks: usize,
    pub compilation_feasible: bool,
    pub estimated_benefit: f32,
}

#[derive(Debug, Clone)]
pub struct DeploymentPlan {
    pub binary_deployments: Vec<BinaryDeployment>,
    pub ssh_deployments: Vec<SshDeployment>,
    pub estimated_performance_gain: f32,
    pub compilation_time: Duration,
    pub total_targets: usize,
}

#[derive(Debug, Clone)]
pub enum BinaryDeploymentDecision {
    Recommended { confidence: f32 },
    Feasible { limitations: Vec<String> },
    NotRecommended { reasons: Vec<String> },
}

#[derive(Debug, Clone)]
pub enum AnalysisError {
    InsufficientData(String),
    InvalidConfiguration(String),
    PerformancePredictionFailed(String),
}

#[derive(Debug, Clone)]
pub enum OptimizationError {
    AnalysisFailed(String),
    CompilationStrategyFailed(String),
    TargetGroupingFailed(String),
}

impl std::fmt::Display for AnalysisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnalysisError::InsufficientData(msg) => write!(f, "Insufficient data: {}", msg),
            AnalysisError::InvalidConfiguration(msg) => write!(f, "Invalid configuration: {}", msg),
            AnalysisError::PerformancePredictionFailed(msg) => write!(f, "Performance prediction failed: {}", msg),
        }
    }
}

impl std::fmt::Display for OptimizationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OptimizationError::AnalysisFailed(msg) => write!(f, "Analysis failed: {}", msg),
            OptimizationError::CompilationStrategyFailed(msg) => write!(f, "Compilation strategy failed: {}", msg),
            OptimizationError::TargetGroupingFailed(msg) => write!(f, "Target grouping failed: {}", msg),
        }
    }
}

impl std::error::Error for AnalysisError {}

impl From<AnalysisError> for DeployError {
    fn from(err: AnalysisError) -> Self {
        DeployError::Configuration(err.to_string())
    }
}

impl From<OptimizationError> for DeployError {
    fn from(err: OptimizationError) -> Self {
        DeployError::Configuration(err.to_string())
    }
}

impl DeploymentOptimizer {
    pub fn new() -> Self {
        Self {
            binary_analyzer: BinaryDeploymentAnalyzer::new(),
            performance_predictor: PerformancePredictor::new(),
        }
    }

    /// Analyze optimization potential for an execution plan
    pub async fn analyze_optimization_potential(
        &self,
        execution_plan: &RustlePlanOutput,
        capabilities: &CompilationCapabilities,
        inventory: &ParsedInventory,
    ) -> Result<OptimizationAnalysis> {
        info!("Analyzing optimization potential for execution plan");

        let total_tasks = execution_plan.total_tasks as usize;
        if total_tasks == 0 {
            return Err(AnalysisError::InsufficientData("No tasks in execution plan".to_string()).into());
        }

        // Analyze binary compatibility of tasks
        // Extract all tasks from the execution plan
        let all_tasks: Vec<_> = execution_plan.plays.iter()
            .flat_map(|play| play.batches.iter())
            .flat_map(|batch| batch.tasks.iter())
            .collect();
        
        // Convert collected references to owned values for the method call
        let task_plans: Vec<crate::execution::TaskPlan> = all_tasks.into_iter().cloned().collect();
        let binary_compatible_tasks = self.binary_analyzer
            .count_binary_compatible_tasks(&task_plans)?;

        // Group hosts by target architecture
        let target_breakdown = self.analyze_targets(inventory, capabilities).await?;

        // Calculate compilation overhead
        let compilation_overhead = self.estimate_compilation_time(
            binary_compatible_tasks,
            target_breakdown.len(),
        );

        // Estimate performance speedup
        let estimated_speedup = self.performance_predictor.estimate_speedup(
            binary_compatible_tasks,
            total_tasks,
            inventory.hosts.len(),
        );

        // Calculate optimization score
        let compatibility_ratio = binary_compatible_tasks as f32 / total_tasks as f32;
        let target_support_ratio = self.calculate_target_support_ratio(&target_breakdown, capabilities);
        let optimization_score = (compatibility_ratio * 0.6) + (target_support_ratio * 0.4);

        // Determine recommended strategy
        let recommended_strategy = self.determine_strategy(
            optimization_score,
            estimated_speedup,
            compilation_overhead,
        );

        debug!("Optimization analysis: score={:.2}, compatible_tasks={}/{}, speedup={:.1}x", 
               optimization_score, binary_compatible_tasks, total_tasks, estimated_speedup);

        Ok(OptimizationAnalysis {
            optimization_score,
            binary_compatible_tasks,
            total_tasks,
            estimated_speedup,
            compilation_overhead,
            recommended_strategy,
            target_breakdown,
        })
    }

    /// Create optimal deployment plan
    pub async fn create_optimal_deployment_plan(
        &self,
        execution_plan: &RustlePlanOutput,
        capabilities: &CompilationCapabilities,
        inventory: &ParsedInventory,
    ) -> Result<DeploymentPlan> {
        let analysis = self.analyze_optimization_potential(execution_plan, capabilities, inventory).await?;
        
        let mut plan = DeploymentPlan::new();
        
        match analysis.recommended_strategy {
            RecommendedStrategy::BinaryOnly => {
                // All targets should use binary deployment where possible
                self.create_binary_deployments(&mut plan, execution_plan, &analysis.target_breakdown, capabilities).await?;
            }
            RecommendedStrategy::Hybrid => {
                // Mix of binary and SSH deployments based on compatibility
                self.create_hybrid_deployment(&mut plan, execution_plan, &analysis.target_breakdown, capabilities).await?;
            }
            RecommendedStrategy::SshOnly => {
                // Use SSH deployment for all targets
                self.create_ssh_deployments(&mut plan, execution_plan, inventory).await?;
            }
        }

        plan.estimated_performance_gain = analysis.estimated_speedup;
        plan.compilation_time = analysis.compilation_overhead;
        plan.total_targets = inventory.hosts.len();

        info!("Created deployment plan: {} binary, {} SSH deployments", 
              plan.binary_deployments.len(), plan.ssh_deployments.len());

        Ok(plan)
    }

    /// Estimate performance gain based on deployment mix
    pub fn estimate_performance_gain(
        &self,
        binary_tasks: usize,
        ssh_tasks: usize,
        target_hosts: usize,
    ) -> f32 {
        if binary_tasks + ssh_tasks == 0 {
            return 0.0;
        }

        let binary_ratio = binary_tasks as f32 / (binary_tasks + ssh_tasks) as f32;
        let host_multiplier = (target_hosts as f32).min(10.0) / 10.0; // Scale benefits with host count
        
        // Binary deployments provide 2-10x speedup, SSH provides 1x
        let base_speedup = 1.0 + (binary_ratio * 4.0); // Average 5x speedup for binary
        base_speedup * host_multiplier
    }

    /// Determine if binary deployment should be used for specific tasks
    pub fn should_use_binary_deployment(
        &self,
        tasks: &[TaskPlan],
        capabilities: &CompilationCapabilities,
        target: &str,
    ) -> BinaryDeploymentDecision {
        // Check if target is supported
        if !capabilities.supports_target(target) {
            return BinaryDeploymentDecision::NotRecommended {
                reasons: vec![format!("Target {} not supported by current toolchain", target)],
            };
        }

        // Analyze task compatibility
        let compatible_count = self.binary_analyzer
            .count_binary_compatible_tasks(tasks)
            .unwrap_or(0);

        let compatibility_ratio = compatible_count as f32 / tasks.len() as f32;

        if compatibility_ratio >= 0.8 {
            BinaryDeploymentDecision::Recommended { 
                confidence: compatibility_ratio 
            }
        } else if compatibility_ratio >= 0.3 {
            BinaryDeploymentDecision::Feasible {
                limitations: vec![
                    format!("{} of {} tasks are binary-compatible", compatible_count, tasks.len())
                ],
            }
        } else {
            BinaryDeploymentDecision::NotRecommended {
                reasons: vec![
                    format!("Low binary compatibility: {:.1}%", compatibility_ratio * 100.0)
                ],
            }
        }
    }

    // Private helper methods

    async fn analyze_targets(
        &self,
        inventory: &ParsedInventory,
        capabilities: &CompilationCapabilities,
    ) -> Result<HashMap<String, TargetAnalysis>> {
        let mut target_breakdown = HashMap::new();

        // Group hosts by target (simplified - in real implementation would detect actual targets)
        let default_target = "x86_64-unknown-linux-gnu".to_string();
        
        let analysis = TargetAnalysis {
            target_triple: default_target.clone(),
            host_count: inventory.hosts.len(),
            compatible_tasks: 0, // Will be calculated later
            compilation_feasible: capabilities.supports_target(&default_target),
            estimated_benefit: 5.0, // Estimated 5x speedup
        };

        target_breakdown.insert(default_target, analysis);
        Ok(target_breakdown)
    }

    fn calculate_target_support_ratio(
        &self,
        target_breakdown: &HashMap<String, TargetAnalysis>,
        capabilities: &CompilationCapabilities,
    ) -> f32 {
        let total_hosts: usize = target_breakdown.values().map(|t| t.host_count).sum();
        if total_hosts == 0 {
            return 0.0;
        }

        let supported_hosts: usize = target_breakdown.values()
            .filter(|t| capabilities.supports_target(&t.target_triple))
            .map(|t| t.host_count)
            .sum();

        supported_hosts as f32 / total_hosts as f32
    }

    fn estimate_compilation_time(&self, compatible_tasks: usize, target_count: usize) -> Duration {
        // Base compilation time + overhead per target
        let base_time = Duration::from_secs(30);
        let per_target_time = Duration::from_secs(20);
        let per_task_time = Duration::from_millis(100);

        base_time + (per_target_time * target_count as u32) + (per_task_time * compatible_tasks as u32)
    }

    fn determine_strategy(
        &self,
        optimization_score: f32,
        estimated_speedup: f32,
        compilation_overhead: Duration,
    ) -> RecommendedStrategy {
        // If compilation takes too long relative to expected benefits, prefer SSH
        let overhead_seconds = compilation_overhead.as_secs() as f32;
        let benefit_threshold = overhead_seconds / 60.0; // Benefit should exceed 1 minute per minute of compilation

        if optimization_score >= 0.8 && estimated_speedup > benefit_threshold {
            RecommendedStrategy::BinaryOnly
        } else if optimization_score >= 0.3 && estimated_speedup > benefit_threshold / 2.0 {
            RecommendedStrategy::Hybrid
        } else {
            RecommendedStrategy::SshOnly
        }
    }

    async fn create_binary_deployments(
        &self,
        _plan: &mut DeploymentPlan,
        _execution_plan: &RustlePlanOutput,
        _target_breakdown: &HashMap<String, TargetAnalysis>,
        _capabilities: &CompilationCapabilities,
    ) -> Result<()> {
        // TODO: Implement binary deployment creation
        Ok(())
    }

    async fn create_hybrid_deployment(
        &self,
        _plan: &mut DeploymentPlan,
        _execution_plan: &RustlePlanOutput,
        _target_breakdown: &HashMap<String, TargetAnalysis>,
        _capabilities: &CompilationCapabilities,
    ) -> Result<()> {
        // TODO: Implement hybrid deployment creation
        Ok(())
    }

    async fn create_ssh_deployments(
        &self,
        _plan: &mut DeploymentPlan,
        _execution_plan: &RustlePlanOutput,
        _inventory: &ParsedInventory,
    ) -> Result<()> {
        // TODO: Implement SSH deployment creation
        Ok(())
    }
}

impl DeploymentPlan {
    pub fn new() -> Self {
        Self {
            binary_deployments: Vec::new(),
            ssh_deployments: Vec::new(),
            estimated_performance_gain: 0.0,
            compilation_time: Duration::default(),
            total_targets: 0,
        }
    }
}

impl Default for DeploymentOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

// Supporting analyzer structs

struct BinaryDeploymentAnalyzer;

impl BinaryDeploymentAnalyzer {
    fn new() -> Self {
        Self
    }

    fn count_binary_compatible_tasks(&self, tasks: &[crate::execution::TaskPlan]) -> Result<usize> {
        // Simplified compatibility check - in real implementation would analyze module types
        let compatible_count = tasks.iter()
            .filter(|task| self.is_task_binary_compatible(task))
            .count();
        
        Ok(compatible_count)
    }

    fn is_task_binary_compatible(&self, _task: &crate::execution::TaskPlan) -> bool {
        // Simplified - assume most core modules are binary compatible
        // In real implementation, check against module registry
        true
    }
}

struct PerformancePredictor;

impl PerformancePredictor {
    fn new() -> Self {
        Self
    }

    fn estimate_speedup(&self, binary_tasks: usize, total_tasks: usize, host_count: usize) -> f32 {
        if total_tasks == 0 {
            return 1.0;
        }

        let binary_ratio = binary_tasks as f32 / total_tasks as f32;
        let host_factor = (host_count as f32).sqrt(); // Benefits scale with more hosts
        
        // Base speedup of 2-10x for binary deployment
        1.0 + (binary_ratio * 4.0 * host_factor.min(3.0))
    }
}