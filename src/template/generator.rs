use crate::execution::plan::ModuleSpec;
use crate::execution::rustle_plan::{BinaryDeploymentPlan, RustlePlanOutput};
use crate::types::compilation::OptimizationLevel;
use crate::types::deployment::RuntimeConfig;
use crate::types::platform::Platform;
use anyhow::Result;
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

use super::{DataEmbedder, TemplateCache, TemplateOptimizer};

#[derive(Error, Debug)]
pub enum TemplateError {
    #[error("Template generation failed: {0}")]
    Generation(String),
    #[error("Template compilation failed: {0}")]
    Compilation(String),
    #[error("Data embedding failed: {0}")]
    Embedding(String),
    #[error("Template optimization failed: {0}")]
    Optimization(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Template error: {0}")]
    Template(#[from] Box<handlebars::TemplateError>),
    #[error("Embed error: {0}")]
    EmbedError(#[from] super::EmbedError),
    #[error("General error: {0}")]
    Anyhow(#[from] anyhow::Error),
}

/// Binary template generator that creates Rust source code for deployment
pub struct BinaryTemplateGenerator {
    config: TemplateConfig,
    cache: TemplateCache,
    embedder: DataEmbedder,
    optimizer: TemplateOptimizer,
    handlebars: Handlebars<'static>,
}

#[derive(Debug, Clone)]
pub struct TemplateConfig {
    pub template_dir: PathBuf,
    pub cache_templates: bool,
    pub optimization_level: OptimizationLevel,
    pub generate_docs: bool,
    pub include_debug_info: bool,
    pub compress_static_files: bool,
    pub compression_algorithm: CompressionType,
    pub encrypt_secrets: bool,
}

// OptimizationLevel moved to crate::types::compilation
// Use: use crate::types::compilation::OptimizationLevel;
// Note: Aggressive variant is now OptimizationLevel::Aggressive

#[derive(Debug, Clone)]
pub enum CompressionType {
    None,
    Gzip,
    Lz4,
    Zstd,
}

impl Default for TemplateConfig {
    fn default() -> Self {
        Self {
            template_dir: PathBuf::from("templates"),
            cache_templates: true,
            optimization_level: OptimizationLevel::Release,
            generate_docs: false,
            include_debug_info: false,
            compress_static_files: true,
            compression_algorithm: CompressionType::Zstd,
            encrypt_secrets: true,
        }
    }
}

/// Complete generated template ready for compilation
#[derive(Debug, Clone)]
pub struct GeneratedTemplate {
    pub template_id: String,
    pub source_files: HashMap<PathBuf, String>,
    pub embedded_data: EmbeddedData,
    pub cargo_toml: String,
    pub build_script: Option<String>,
    pub target_info: TargetInfo,
    pub compilation_flags: Vec<String>,
    pub estimated_binary_size: u64,
    pub cache_key: String,
}

#[derive(Debug, Clone)]
pub struct EmbeddedData {
    pub execution_plan: String,
    pub static_files: HashMap<String, Vec<u8>>,
    pub module_binaries: HashMap<String, Vec<u8>>,
    pub runtime_config: RuntimeConfig,
    pub secrets: EncryptedSecrets,
    pub facts_cache: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TargetInfo {
    pub target_triple: String,
    pub platform: Platform,
    pub architecture: String,
    pub os_family: String,
    pub libc: Option<String>,
    pub features: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct EncryptedSecrets {
    pub vault_data: HashMap<String, Vec<u8>>,
    pub encryption_key_id: String,
    pub decryption_method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleDependency {
    pub name: String,
    pub version: String,
    pub features: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ExecutionPlanDiff {
    pub added_tasks: Vec<String>,
    pub removed_tasks: Vec<String>,
    pub modified_tasks: Vec<String>,
    pub modified_modules: Vec<String>,
}

impl BinaryTemplateGenerator {
    pub fn new(config: TemplateConfig) -> Result<Self> {
        let mut handlebars = Handlebars::new();

        // Register built-in templates
        handlebars
            .register_template_string("main_rs", include_str!("../templates/main_rs.template"))
            .map_err(Box::new)?;

        handlebars
            .register_template_string(
                "cargo_toml",
                include_str!("../templates/cargo_toml.template"),
            )
            .map_err(Box::new)?;

        let cache = TemplateCache::new(config.cache_templates);
        let embedder = DataEmbedder::new(&config)?;
        let optimizer = TemplateOptimizer::new();

        Ok(Self {
            config,
            cache,
            embedder,
            optimizer,
            handlebars,
        })
    }

    /// Generate complete binary template from execution plan
    pub async fn generate_binary_template(
        &self,
        execution_plan: &RustlePlanOutput,
        binary_deployment: &BinaryDeploymentPlan,
        target_info: &TargetInfo,
    ) -> Result<GeneratedTemplate, TemplateError> {
        let template_id = uuid::Uuid::new_v4().to_string();
        let cache_key = self.generate_cache_key(execution_plan, target_info)?;

        // Check cache first
        if let Some(cached_template) = self.cache.get(&cache_key) {
            return Ok(cached_template);
        }

        // Embed execution data
        let embedded_data = self
            .embedder
            .embed_execution_data(execution_plan, binary_deployment, target_info)
            .await?;

        // Generate main.rs
        let main_rs = self.generate_main_rs(execution_plan, &embedded_data)?;

        // Generate Cargo.toml
        let cargo_toml = self.generate_cargo_toml(
            &self.extract_dependencies(execution_plan),
            &target_info.target_triple,
        )?;

        // Generate module implementations
        let mut modules = std::collections::HashSet::new();

        // Collect modules from regular tasks and handlers
        for play in &execution_plan.plays {
            for batch in &play.batches {
                for task in &batch.tasks {
                    modules.insert(task.module.clone());
                }
            }
            // Collect modules from handlers
            for handler in &play.handlers {
                modules.insert(handler.module.clone());
            }
        }

        let module_files = self.generate_module_implementations(
            &modules
                .into_iter()
                .map(|m| ModuleSpec {
                    name: m,
                    source: crate::execution::plan::ModuleSource::Builtin,
                    version: Some("1.0.0".to_string()),
                    checksum: None,
                    dependencies: vec![],
                    static_link: false,
                })
                .collect::<Vec<_>>(),
            &target_info.platform,
        )?;

        // Create source files map
        let mut source_files = HashMap::new();
        source_files.insert(PathBuf::from("src/main.rs"), main_rs);

        for (path, content) in module_files {
            source_files.insert(PathBuf::from(format!("src/{path}")), content);
        }

        let template = GeneratedTemplate {
            template_id,
            source_files,
            embedded_data,
            cargo_toml,
            build_script: None,
            target_info: target_info.clone(),
            compilation_flags: self.generate_compilation_flags(target_info),
            estimated_binary_size: self.estimate_binary_size(execution_plan),
            cache_key: cache_key.clone(),
        };

        // Cache the template
        self.cache.insert(cache_key, template.clone());

        Ok(template)
    }

    /// Generate incremental template from base template and changes
    pub async fn generate_incremental_template(
        &self,
        _base_template: &GeneratedTemplate,
        _changes: &ExecutionPlanDiff,
    ) -> Result<GeneratedTemplate, TemplateError> {
        // For now, regenerate completely - incremental updates would be a future optimization
        // This would involve analyzing the diff and only updating affected parts
        Err(TemplateError::Generation(
            "Incremental generation not yet implemented".to_string(),
        ))
    }

    /// Generate Cargo.toml with dependencies and optimizations
    pub fn generate_cargo_toml(
        &self,
        dependencies: &[ModuleDependency],
        target_triple: &str,
    ) -> Result<String, TemplateError> {
        let template_data = serde_json::json!({
            "dependencies": dependencies,
            "target_triple": target_triple,
            "optimization_level": match self.config.optimization_level {
                OptimizationLevel::Debug => "0",
                OptimizationLevel::Release => "3",
                OptimizationLevel::Aggressive => "3",
                _ => "3", // Default to release level for other variants
            },
            "lto": matches!(self.config.optimization_level, OptimizationLevel::Release | OptimizationLevel::Aggressive),
            "strip": !self.config.include_debug_info,
            "panic_abort": matches!(self.config.optimization_level, OptimizationLevel::Release | OptimizationLevel::Aggressive),
        });

        self.handlebars
            .render("cargo_toml", &template_data)
            .map_err(|e| TemplateError::Generation(format!("Failed to render Cargo.toml: {e}")))
    }

    /// Generate main.rs with embedded execution logic
    pub fn generate_main_rs(
        &self,
        execution_plan: &RustlePlanOutput,
        embedded_data: &EmbeddedData,
    ) -> Result<String, TemplateError> {
        // Collect unique modules from the execution plan
        let mut modules = std::collections::HashSet::new();
        for play in &execution_plan.plays {
            for batch in &play.batches {
                for task in &batch.tasks {
                    modules.insert(task.module.clone());
                }
            }
            for handler in &play.handlers {
                modules.insert(handler.module.clone());
            }
        }

        // Convert to template data format
        let modules_data: Vec<serde_json::Value> = modules
            .iter()
            .map(|module| {
                serde_json::json!({
                    "name": module,
                    "normalized_name": module.replace(':', "_")
                })
            })
            .collect();

        let template_data = serde_json::json!({
            "execution_plan": serde_json::to_string(&execution_plan)?,
            "runtime_config": serde_json::to_string(&embedded_data.runtime_config)?,
            "static_files": self.generate_static_file_declarations(&embedded_data.static_files)?,
            "module_implementations": self.generate_module_declarations(execution_plan)?,
            "modules": modules_data,
            "total_tasks": execution_plan.total_tasks,
        });

        self.handlebars
            .render("main_rs", &template_data)
            .map_err(|e| TemplateError::Generation(format!("Failed to render main.rs: {e}")))
    }

    /// Generate module implementations for target platform
    pub fn generate_module_implementations(
        &self,
        modules: &[ModuleSpec],
        target_platform: &Platform,
    ) -> Result<HashMap<String, String>, TemplateError> {
        let mut implementations = HashMap::new();

        // Always include parameter mapping modules (required for compatibility)
        let param_mapping_modules = [
            (
                "parameter_mapping/error",
                include_str!("../templates/modules/parameter_mapping/error.rs"),
            ),
            (
                "parameter_mapping/mapper",
                include_str!("../templates/modules/parameter_mapping/mapper.rs"),
            ),
            (
                "parameter_mapping/handlers/mod",
                include_str!("../templates/modules/parameter_mapping/handlers/mod.rs"),
            ),
            (
                "parameter_mapping/handlers/command",
                include_str!("../templates/modules/parameter_mapping/handlers/command.rs"),
            ),
            (
                "parameter_mapping/handlers/package",
                include_str!("../templates/modules/parameter_mapping/handlers/package.rs"),
            ),
            (
                "parameter_mapping/handlers/service",
                include_str!("../templates/modules/parameter_mapping/handlers/service.rs"),
            ),
            (
                "parameter_mapping/handlers/debug",
                include_str!("../templates/modules/parameter_mapping/handlers/debug.rs"),
            ),
        ];

        for (module_path, content) in param_mapping_modules {
            implementations.insert(format!("modules/{module_path}.rs"), content.to_string());
        }

        // Generate implementations for execution plan modules
        for module in modules {
            let module_code = self.generate_module_wrapper(&module.name, target_platform)?;
            implementations.insert(
                format!("modules/{}.rs", module.name.replace(':', "_")),
                module_code,
            );
        }

        // Generate modules/mod.rs file to declare all modules
        let mut mod_declarations = vec!["pub mod parameter_mapping;".to_string()];

        for module in modules {
            let module_name = module.name.replace(':', "_");
            mod_declarations.push(format!("pub mod {module_name};"));
        }

        let mod_file_content = mod_declarations.join("\n");
        implementations.insert("modules/mod.rs".to_string(), mod_file_content);

        Ok(implementations)
    }

    /// Optimize template for size, speed, or memory usage
    pub async fn optimize_template(
        &self,
        template: &GeneratedTemplate,
        optimization_level: OptimizationLevel,
    ) -> Result<GeneratedTemplate, TemplateError> {
        let mut optimized_template = template.clone();

        match optimization_level {
            OptimizationLevel::Debug => {
                // No optimizations for debug builds
            }
            OptimizationLevel::Release => {
                self.optimizer
                    .optimize_for_size(&mut optimized_template)
                    .await?;
            }
            OptimizationLevel::Aggressive => {
                self.optimizer
                    .optimize_for_size(&mut optimized_template)
                    .await?;
                self.optimizer
                    .optimize_for_speed(&mut optimized_template)
                    .await?;
            }
            OptimizationLevel::ReleaseWithDebugInfo => {
                self.optimizer
                    .optimize_for_size(&mut optimized_template)
                    .await?;
            }
            OptimizationLevel::MinSize
            | OptimizationLevel::MinSizeRelease
            | OptimizationLevel::MinimalSize => {
                self.optimizer
                    .optimize_for_size(&mut optimized_template)
                    .await?;
            }
        }

        Ok(optimized_template)
    }

    // Helper methods

    fn generate_cache_key(
        &self,
        execution_plan: &RustlePlanOutput,
        target_info: &TargetInfo,
    ) -> Result<String> {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(serde_json::to_string(execution_plan)?);
        hasher.update(&target_info.target_triple);
        hasher.update(serde_json::to_string(&self.config.optimization_level)?);

        Ok(format!("{:x}", hasher.finalize()))
    }

    fn extract_dependencies(&self, execution_plan: &RustlePlanOutput) -> Vec<ModuleDependency> {
        let mut deps = vec![
            ModuleDependency {
                name: "tokio".to_string(),
                version: "1".to_string(),
                features: vec!["full".to_string()],
            },
            ModuleDependency {
                name: "serde".to_string(),
                version: "1".to_string(),
                features: vec!["derive".to_string()],
            },
            ModuleDependency {
                name: "serde_json".to_string(),
                version: "1".to_string(),
                features: vec![],
            },
            ModuleDependency {
                name: "anyhow".to_string(),
                version: "1".to_string(),
                features: vec![],
            },
            ModuleDependency {
                name: "tracing".to_string(),
                version: "0.1".to_string(),
                features: vec![],
            },
            ModuleDependency {
                name: "tracing-subscriber".to_string(),
                version: "0.3".to_string(),
                features: vec![],
            },
            ModuleDependency {
                name: "reqwest".to_string(),
                version: "0.11".to_string(),
                features: vec!["json".to_string()],
            },
            ModuleDependency {
                name: "thiserror".to_string(),
                version: "1".to_string(),
                features: vec![],
            },
        ];

        // Add module-specific dependencies based on what modules are used
        let used_modules: std::collections::HashSet<String> = execution_plan
            .plays
            .iter()
            .flat_map(|p| &p.batches)
            .flat_map(|b| &b.tasks)
            .map(|t| t.module.clone())
            .collect();

        if used_modules.contains("command") || used_modules.contains("shell") {
            deps.push(ModuleDependency {
                name: "shell-words".to_string(),
                version: "1.1".to_string(),
                features: vec![],
            });
        }

        if used_modules.contains("package") {
            deps.push(ModuleDependency {
                name: "regex".to_string(),
                version: "1.10".to_string(),
                features: vec![],
            });
        }

        deps
    }

    fn generate_compilation_flags(&self, _target_info: &TargetInfo) -> Vec<String> {
        let mut flags = vec![];

        flags.push("--release".to_string());

        if matches!(
            self.config.optimization_level,
            OptimizationLevel::Aggressive
        ) {
            flags.push("-C".to_string());
            flags.push("target-cpu=native".to_string());
        }

        flags
    }

    fn estimate_binary_size(&self, execution_plan: &RustlePlanOutput) -> u64 {
        // Rough estimation based on plan complexity
        let base_size = 5_000_000; // 5MB base
        let per_task_size = 1000; // 1KB per task
        let per_module_size = 500_000; // 500KB per unique module

        let unique_modules = execution_plan
            .plays
            .iter()
            .flat_map(|p| &p.batches)
            .flat_map(|b| &b.tasks)
            .map(|t| &t.module)
            .collect::<std::collections::HashSet<_>>()
            .len() as u64;

        base_size
            + (execution_plan.total_tasks as u64 * per_task_size)
            + (unique_modules * per_module_size)
    }

    fn generate_static_file_declarations(
        &self,
        static_files: &HashMap<String, Vec<u8>>,
    ) -> Result<String> {
        let declarations = static_files
            .keys()
            .map(|path| {
                format!(
                    r#"        files.insert("{}", include_bytes!("static_files/{}"));"#,
                    path,
                    path.replace('/', "_")
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        Ok(declarations)
    }

    fn generate_module_declarations(&self, execution_plan: &RustlePlanOutput) -> Result<String> {
        let mut modules = std::collections::HashSet::new();

        // Collect modules from regular tasks
        for play in &execution_plan.plays {
            for batch in &play.batches {
                for task in &batch.tasks {
                    modules.insert(task.module.clone());
                }
            }
            // Collect modules from handlers
            for handler in &play.handlers {
                modules.insert(handler.module.clone());
            }
        }

        let declarations = modules
            .iter()
            .map(|module| format!("    pub mod {};", module.replace(':', "_")))
            .collect::<Vec<_>>()
            .join("\n");

        Ok(declarations)
    }

    fn generate_module_wrapper(
        &self,
        module_name: &str,
        _target_platform: &Platform,
    ) -> Result<String> {
        // Use template-based modules for common modules
        match module_name {
            "command" | "shell" => Ok(include_str!("../templates/modules/command.rs").to_string()),
            "package" => Ok(include_str!("../templates/modules/package.rs").to_string()),
            "service" => Ok(include_str!("../templates/modules/service.rs").to_string()),
            "debug" => Ok(include_str!("../templates/modules/debug.rs").to_string()),
            "copy" => Ok(include_str!("../templates/modules/copy.rs").to_string()),
            "file" => Ok(include_str!("../templates/modules/file.rs").to_string()),
            _ => {
                // Generate a basic module wrapper for unknown modules
                let module_template = r#"
use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;

pub async fn execute(args: HashMap<String, Value>) -> Result<Value> {
    // Module implementation for {module_name}
    // This is a placeholder implementation
    
    Ok(serde_json::json!({
        "changed": false,
        "failed": false,
        "msg": "Module {module_name} executed successfully (placeholder)"
    }))
}
"#;
                Ok(module_template.replace("{module_name}", module_name))
            }
        }
    }
}

// OptimizationLevel serialization is handled by the derive macro in types::compilation

impl GeneratedTemplate {
    /// Calculate a hash of the template for caching and comparison
    pub fn calculate_hash(&self) -> String {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();

        // Hash the cache key (already includes execution plan and config)
        hasher.update(&self.cache_key);

        // Hash target info
        hasher.update(&self.target_info.target_triple);
        hasher.update(&self.target_info.architecture);
        hasher.update(&self.target_info.os_family);

        // Hash compilation flags
        for flag in &self.compilation_flags {
            hasher.update(flag);
        }

        // Hash source files
        let mut sorted_files: Vec<_> = self.source_files.iter().collect();
        sorted_files.sort_by_key(|(path, _)| *path);
        for (path, content) in sorted_files {
            hasher.update(path.to_string_lossy().as_bytes());
            hasher.update(content);
        }

        // Hash cargo.toml
        hasher.update(&self.cargo_toml);

        format!("{:x}", hasher.finalize())
    }

    /// Write the template files to a directory
    pub async fn write_to_directory(&self, target_dir: &std::path::Path) -> anyhow::Result<()> {
        use anyhow::Context;
        use tokio::fs;

        // Create the target directory if it doesn't exist
        fs::create_dir_all(target_dir)
            .await
            .context("Failed to create target directory")?;

        // Write Cargo.toml
        let cargo_toml_path = target_dir.join("Cargo.toml");
        fs::write(&cargo_toml_path, &self.cargo_toml)
            .await
            .context("Failed to write Cargo.toml")?;

        // Write source files
        for (file_path, content) in &self.source_files {
            let full_path = target_dir.join(file_path);

            // Create parent directory if needed
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent)
                    .await
                    .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
            }

            fs::write(&full_path, content)
                .await
                .with_context(|| format!("Failed to write file: {}", file_path.display()))?;
        }

        // Write build script if present
        if let Some(build_script) = &self.build_script {
            let build_script_path = target_dir.join("build.rs");
            fs::write(&build_script_path, build_script)
                .await
                .context("Failed to write build script")?;
        }

        Ok(())
    }
}
