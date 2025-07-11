use anyhow::Result;
use async_trait::async_trait;
use thiserror::Error;

use super::{GeneratedTemplate, TemplateError};

#[derive(Error, Debug)]
pub enum OptimizationError {
    #[error("Optimization failed: {0}")]
    Failed(String),
    #[error("Strategy not supported: {0}")]
    UnsupportedStrategy(String),
}

/// Template optimization engine
pub struct TemplateOptimizer {
    strategies: Vec<Box<dyn OptimizationStrategy>>,
}

impl TemplateOptimizer {
    pub fn new() -> Self {
        let strategies: Vec<Box<dyn OptimizationStrategy>> = vec![
            Box::new(DeadCodeElimination),
            Box::new(StaticLinkingOptimizer),
            Box::new(CompressionOptimizer),
            Box::new(InliningOptimizer),
        ];

        Self { strategies }
    }

    /// Optimize template for smaller binary size
    pub async fn optimize_for_size(
        &self,
        template: &mut GeneratedTemplate,
    ) -> Result<(), TemplateError> {
        // Apply size-focused optimizations
        for strategy in &self.strategies {
            if matches!(
                strategy.name(),
                "dead_code_elimination" | "compression" | "static_linking"
            ) {
                strategy
                    .apply(template)
                    .await
                    .map_err(|e| TemplateError::Optimization(e.to_string()))?;
            }
        }

        // Update estimated binary size
        template.estimated_binary_size = (template.estimated_binary_size as f64 * 0.7) as u64;

        Ok(())
    }

    /// Optimize template for faster execution
    pub async fn optimize_for_speed(
        &self,
        template: &mut GeneratedTemplate,
    ) -> Result<(), TemplateError> {
        // Apply speed-focused optimizations
        for strategy in &self.strategies {
            if matches!(strategy.name(), "inlining" | "static_linking") {
                strategy
                    .apply(template)
                    .await
                    .map_err(|e| TemplateError::Optimization(e.to_string()))?;
            }
        }

        // Add speed optimization flags
        template.compilation_flags.push("-C".to_string());
        template.compilation_flags.push("opt-level=3".to_string());
        template.compilation_flags.push("-C".to_string());
        template
            .compilation_flags
            .push("target-cpu=native".to_string());

        Ok(())
    }

    /// Optimize template for lower memory usage
    pub async fn optimize_for_memory(
        &self,
        template: &mut GeneratedTemplate,
    ) -> Result<(), TemplateError> {
        // Apply memory-focused optimizations
        for strategy in &self.strategies {
            if matches!(strategy.name(), "compression") {
                strategy
                    .apply(template)
                    .await
                    .map_err(|e| TemplateError::Optimization(e.to_string()))?;
            }
        }

        // Add memory optimization flags
        template.compilation_flags.push("-C".to_string());
        template.compilation_flags.push("opt-level=s".to_string());

        Ok(())
    }

    /// Apply all optimization strategies
    pub async fn optimize_all(
        &self,
        template: &mut GeneratedTemplate,
    ) -> Result<(), TemplateError> {
        let mut strategies = self.strategies.clone();
        strategies.sort_by_key(|s| s.priority());

        for strategy in strategies {
            strategy.apply(template).await.map_err(|e| {
                TemplateError::Optimization(format!("Strategy '{}' failed: {}", strategy.name(), e))
            })?;
        }

        Ok(())
    }
}

impl Default for TemplateOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for template optimization strategies
#[async_trait]
pub trait OptimizationStrategy: Send + Sync {
    async fn apply(&self, template: &mut GeneratedTemplate) -> Result<(), OptimizationError>;
    fn name(&self) -> &'static str;
    fn priority(&self) -> u8;
}

// Make OptimizationStrategy cloneable through dynamic dispatch
impl Clone for Box<dyn OptimizationStrategy> {
    fn clone(&self) -> Self {
        // This is a simplified clone - in practice, each strategy would implement its own clone
        match self.name() {
            "dead_code_elimination" => Box::new(DeadCodeElimination),
            "static_linking" => Box::new(StaticLinkingOptimizer),
            "compression" => Box::new(CompressionOptimizer),
            "inlining" => Box::new(InliningOptimizer),
            _ => Box::new(DeadCodeElimination), // fallback
        }
    }
}

/// Remove unused code and dependencies
pub struct DeadCodeElimination;

#[async_trait]
impl OptimizationStrategy for DeadCodeElimination {
    async fn apply(&self, template: &mut GeneratedTemplate) -> Result<(), OptimizationError> {
        // Analyze which modules and functions are actually used
        let used_modules = self.analyze_module_usage(template)?;

        // Remove unused module implementations
        template.source_files.retain(|path, _| {
            if let Some(module_name) = path.to_str().and_then(|p| {
                if p.starts_with("src/modules/") && p.ends_with(".rs") {
                    Some(
                        p.strip_prefix("src/modules/")
                            .unwrap()
                            .strip_suffix(".rs")
                            .unwrap(),
                    )
                } else {
                    None
                }
            }) {
                used_modules.contains(module_name)
            } else {
                true // Keep non-module files
            }
        });

        // Add dead code elimination flags
        if !template.compilation_flags.contains(&"-C".to_string())
            || !template.compilation_flags.contains(&"lto=fat".to_string())
        {
            template.compilation_flags.push("-C".to_string());
            template.compilation_flags.push("lto=fat".to_string());
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "dead_code_elimination"
    }

    fn priority(&self) -> u8 {
        10 // High priority - run early
    }
}

impl DeadCodeElimination {
    fn analyze_module_usage(
        &self,
        template: &GeneratedTemplate,
    ) -> Result<std::collections::HashSet<String>, OptimizationError> {
        let mut used_modules = std::collections::HashSet::new();

        // Parse the execution plan to find used modules
        if let Ok(plan) = serde_json::from_str::<crate::execution::rustle_plan::RustlePlanOutput>(
            &template.embedded_data.execution_plan,
        ) {
            for play in &plan.plays {
                for batch in &play.batches {
                    for task in &batch.tasks {
                        used_modules.insert(task.module.replace(':', "_"));
                    }
                }
            }
        }

        Ok(used_modules)
    }
}

/// Optimize static linking and reduce binary size
pub struct StaticLinkingOptimizer;

#[async_trait]
impl OptimizationStrategy for StaticLinkingOptimizer {
    async fn apply(&self, template: &mut GeneratedTemplate) -> Result<(), OptimizationError> {
        // Add static linking flags
        template.compilation_flags.push("-C".to_string());
        template
            .compilation_flags
            .push("link-arg=-static".to_string());

        // Update Cargo.toml to prefer static linking
        if template.cargo_toml.contains("[profile.release]") {
            template.cargo_toml = template.cargo_toml.replace(
                "[profile.release]",
                "[profile.release]\nlto = true\ncodegen-units = 1",
            );
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "static_linking"
    }

    fn priority(&self) -> u8 {
        5 // Medium priority
    }
}

/// Compress embedded data to reduce binary size
pub struct CompressionOptimizer;

#[async_trait]
impl OptimizationStrategy for CompressionOptimizer {
    async fn apply(&self, template: &mut GeneratedTemplate) -> Result<(), OptimizationError> {
        // The compression is already handled in the embedder
        // This optimizer could add additional compression passes

        // Add compression-related compilation flags
        template.compilation_flags.push("-C".to_string());
        template.compilation_flags.push("opt-level=z".to_string()); // Optimize for size

        Ok(())
    }

    fn name(&self) -> &'static str {
        "compression"
    }

    fn priority(&self) -> u8 {
        3 // Lower priority - run after dead code elimination
    }
}

/// Inline small functions for better performance
pub struct InliningOptimizer;

#[async_trait]
impl OptimizationStrategy for InliningOptimizer {
    async fn apply(&self, template: &mut GeneratedTemplate) -> Result<(), OptimizationError> {
        // Add inlining hints to generated code
        for (path, content) in template.source_files.iter_mut() {
            if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                // Add #[inline] attributes to small functions
                *content = self.add_inline_attributes(content)?;
            }
        }

        // Add inlining compilation flags
        template.compilation_flags.push("-C".to_string());
        template
            .compilation_flags
            .push("inline-threshold=275".to_string());

        Ok(())
    }

    fn name(&self) -> &'static str {
        "inlining"
    }

    fn priority(&self) -> u8 {
        1 // Lowest priority - run last
    }
}

impl InliningOptimizer {
    fn add_inline_attributes(&self, content: &str) -> Result<String, OptimizationError> {
        // Simple heuristic: add #[inline] to functions shorter than 10 lines
        let lines: Vec<&str> = content.lines().collect();
        let mut result = Vec::new();
        let mut in_function = false;
        let mut function_start = 0;
        let mut brace_count = 0;

        for (i, line) in lines.iter().enumerate() {
            if line.trim_start().starts_with("pub fn ") || line.trim_start().starts_with("fn ") {
                in_function = true;
                function_start = i;
                brace_count = 0;
            }

            if in_function {
                brace_count += line.chars().filter(|&c| c == '{').count() as i32;
                brace_count -= line.chars().filter(|&c| c == '}').count() as i32;

                if brace_count == 0 && line.contains('}') {
                    // End of function
                    let function_length = i - function_start;
                    if function_length < 10 {
                        // Add #[inline] before the function
                        if function_start > 0
                            && !lines[function_start - 1].trim().starts_with("#[inline")
                        {
                            result.insert(function_start, "    #[inline]");
                        }
                    }
                    in_function = false;
                }
            }

            result.push(line);
        }

        Ok(result.join("\n"))
    }
}
