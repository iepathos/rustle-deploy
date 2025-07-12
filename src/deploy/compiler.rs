use crate::deploy::{CompilationCache, DeployError, Result};
use crate::types::*;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::process::Command;
use tracing::{debug, info};

pub struct BinaryCompiler {
    cache: CompilationCache,
}

impl BinaryCompiler {
    pub fn new(cache: CompilationCache) -> Self {
        Self { cache }
    }

    pub async fn compile_binary(&self, compilation: &BinaryCompilation) -> Result<CompiledBinary> {
        self.compile_binary_with_modules(compilation, &[]).await
    }

    /// Compile binary with integrated module support
    pub async fn compile_binary_with_modules(
        &self,
        compilation: &BinaryCompilation,
        compiled_modules: &[crate::modules::CompiledModule],
    ) -> Result<CompiledBinary> {
        info!(
            "Starting compilation for {} with {} modules",
            compilation.binary_name,
            compiled_modules.len()
        );

        let start_time = std::time::Instant::now();

        // Check cache first (with module checksum included)
        let cache_key = self.calculate_compilation_checksum(compilation, compiled_modules);
        if let Some(cached) = self.check_cache(&cache_key) {
            info!("Using cached binary for {}", compilation.binary_name);
            return Ok(cached);
        }

        // Create temporary project directory
        let temp_dir = tempfile::TempDir::new()
            .map_err(|e| DeployError::compilation(format!("Failed to create temp dir: {e}")))?;

        let project_dir = temp_dir.path();

        // Generate the binary project with modules
        self.generate_binary_project_with_modules(project_dir, compilation, compiled_modules)
            .await?;

        // Compile the binary
        let binary_data = self
            .cross_compile(
                project_dir,
                &compilation.target_triple,
                &compilation.compilation_options,
            )
            .await?;

        // Calculate checksum and size
        let checksum = self.calculate_checksum(&binary_data);
        let size = binary_data.len() as u64;
        let compilation_time = start_time.elapsed();

        let compiled_binary = CompiledBinary {
            compilation_id: compilation.compilation_id.clone(),
            target_triple: compilation.target_triple.clone(),
            binary_data,
            checksum: checksum.clone(),
            size,
            compilation_time,
            optimization_level: compilation.compilation_options.optimization_level.clone(),
            source_info: crate::types::compilation::BinarySourceInfo {
                source_type: crate::types::compilation::BinarySourceType::FreshCompilation {
                    project_path: compilation.output_path.clone(),
                },
                template_hash: "generated".to_string(),
                build_metadata: crate::types::compilation::BuildMetadata {
                    created_at: chrono::Utc::now(),
                    toolchain_version: "cargo".to_string(),
                    features: compilation.compilation_options.custom_features.clone(),
                },
            },
        };

        // Cache the result
        self.cache.store_binary(&cache_key, &compiled_binary)?;

        info!(
            "Compilation completed for {} in {:?}",
            compilation.binary_name, compilation_time
        );

        Ok(compiled_binary)
    }

    pub fn check_cache(&self, compilation_hash: &str) -> Option<CompiledBinary> {
        self.cache.get_cached_binary(compilation_hash)
    }

    pub async fn cross_compile(
        &self,
        source_dir: &Path,
        target_triple: &str,
        options: &LegacyCompilationOptions,
    ) -> Result<Vec<u8>> {
        info!("Cross-compiling for target: {}", target_triple);

        let mut cmd = Command::new("cargo");

        cmd.args(["build", "--release"])
            .arg("--target")
            .arg(target_triple)
            .current_dir(source_dir);

        // Add optimization flags based on options
        let mut rustflags = Vec::new();

        match options.optimization_level {
            OptimizationLevel::Debug => {
                cmd.arg("--").arg("--debug");
            }
            OptimizationLevel::Release | OptimizationLevel::Aggressive => {
                rustflags.push("-C opt-level=3".to_string());
            }
            OptimizationLevel::ReleaseWithDebugInfo => {
                rustflags.push("-C opt-level=3".to_string());
                rustflags.push("-C debuginfo=2".to_string());
            }
            OptimizationLevel::MinSize
            | OptimizationLevel::MinSizeRelease
            | OptimizationLevel::MinimalSize => {
                rustflags.push("-C opt-level=z".to_string());
                rustflags.push("-C panic=abort".to_string());
            }
        }

        if let Some(target_cpu) = &options.target_cpu {
            rustflags.push(format!("-C target-cpu={target_cpu}"));
        }

        if options.strip_symbols {
            rustflags.push("-C strip=symbols".to_string());
        }

        if options.static_linking {
            rustflags.push("-C target-feature=+crt-static".to_string());
        }

        if !rustflags.is_empty() {
            cmd.env("RUSTFLAGS", rustflags.join(" "));
        }

        // Execute compilation
        debug!("Running cargo build command");
        let output = cmd
            .output()
            .await
            .map_err(|e| DeployError::CompilationFailed {
                target: target_triple.to_string(),
                reason: format!("Failed to execute cargo: {e}"),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DeployError::CompilationFailed {
                target: target_triple.to_string(),
                reason: format!("Compilation failed: {stderr}"),
            });
        }

        // Find the compiled binary
        let binary_path = self.find_compiled_binary(source_dir, target_triple)?;

        // Read binary data
        let mut binary_data = fs::read(&binary_path)
            .await
            .map_err(|e| DeployError::compilation(format!("Failed to read binary: {e}")))?;

        // Apply compression if requested
        if options.compression {
            binary_data = self.compress_binary(&binary_data)?;
        }

        info!("Successfully compiled binary ({} bytes)", binary_data.len());
        Ok(binary_data)
    }

    #[allow(dead_code)]
    async fn generate_binary_project(
        &self,
        project_dir: &Path,
        compilation: &BinaryCompilation,
    ) -> Result<()> {
        self.generate_binary_project_with_modules(project_dir, compilation, &[])
            .await
    }

    async fn generate_binary_project_with_modules(
        &self,
        project_dir: &Path,
        compilation: &BinaryCompilation,
        compiled_modules: &[crate::modules::CompiledModule],
    ) -> Result<()> {
        info!(
            "Generating binary project in {:?} with {} modules",
            project_dir,
            compiled_modules.len()
        );

        // Create project structure
        fs::create_dir_all(project_dir.join("src")).await?;

        // Generate Cargo.toml with module dependencies
        let cargo_toml = self.generate_cargo_toml_with_modules(compilation, compiled_modules)?;
        fs::write(project_dir.join("Cargo.toml"), cargo_toml).await?;

        // Generate main.rs with module integration
        let main_rs = self.generate_main_rs_with_modules(compilation, compiled_modules)?;
        fs::write(project_dir.join("src").join("main.rs"), main_rs).await?;

        // Write module files
        if !compiled_modules.is_empty() {
            self.write_module_files(project_dir, compiled_modules)
                .await?;
        }

        // Write embedded files
        self.write_embedded_files(project_dir, &compilation.embedded_data.static_files)
            .await?;

        debug!("Binary project generated successfully");
        Ok(())
    }

    #[allow(dead_code)]
    fn generate_cargo_toml(&self, compilation: &BinaryCompilation) -> Result<String> {
        let cargo_toml = format!(
            r#"
[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = {{ version = "1", features = ["derive"] }}
serde_json = "1"
tokio = {{ version = "1", features = ["full"] }}
anyhow = "1"
tracing = "0.1"
"#,
            compilation.binary_name
        );

        Ok(cargo_toml)
    }

    #[allow(dead_code)]
    fn generate_main_rs(&self, compilation: &BinaryCompilation) -> Result<String> {
        let execution_plan_json = serde_json::to_string(&compilation.embedded_data.execution_plan)
            .map_err(|e| {
                DeployError::TemplateGeneration(format!("Failed to serialize execution plan: {e}"))
            })?;

        let runtime_config_json = serde_json::to_string(&compilation.embedded_data.runtime_config)
            .map_err(|e| {
                DeployError::TemplateGeneration(format!("Failed to serialize runtime config: {e}"))
            })?;

        let embedded_files =
            self.generate_embedded_file_declarations(&compilation.embedded_data.static_files);
        let modules =
            self.generate_module_implementations(&compilation.embedded_data.module_implementations);

        let main_rs = format!(
            "
use std::collections::HashMap;
use serde_json::Value;
use anyhow::Result;

mod embedded_data {{
    use super::*;
    
    pub const EXECUTION_PLAN: &str = {execution_plan_json};
    pub const RUNTIME_CONFIG: &str = {runtime_config_json};
    
    pub fn get_embedded_files() -> HashMap<String, Vec<u8>> {{
        let mut files = HashMap::new();
        {embedded_files}
        files
    }}
}}

mod modules {{
    use super::*;
    {modules}
}}

#[tokio::main]
async fn main() -> Result<()> {{
    tracing_subscriber::fmt::init();
    
    let execution_plan: Value = serde_json::from_str(embedded_data::EXECUTION_PLAN)?;
    let runtime_config: Value = serde_json::from_str(embedded_data::RUNTIME_CONFIG)?;
    
    println!(\"Rustle Runner - Embedded Execution\");
    println!(\"Execution Plan: {{}} tasks\", execution_plan.as_array().unwrap_or(&vec![]).len());
    
    // TODO: Implement actual execution logic
    // This would typically:
    // 1. Parse the execution plan
    // 2. Execute tasks in the specified order
    // 3. Report results back to controller if configured
    // 4. Clean up resources
    
    println!(\"Execution completed successfully\");
    Ok(())
}}
",
        );

        Ok(main_rs)
    }

    #[allow(dead_code)]
    fn generate_embedded_file_declarations(&self, files: &[StaticFile]) -> String {
        files
            .iter()
            .map(|file| {
                format!(
                    "files.insert(\"{}\", include_bytes!(\"{}\").to_vec());",
                    file.embedded_path, file.embedded_path
                )
            })
            .collect::<Vec<_>>()
            .join("\n        ")
    }

    #[allow(dead_code)]
    fn generate_module_implementations(&self, modules: &[ModuleImplementation]) -> String {
        modules
            .iter()
            .map(|module| {
                format!(
                    "
    pub mod {} {{
        use super::*;
        {}
    }}
    ",
                    module.module_name.replace('-', "_"),
                    module.source_code
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    async fn write_embedded_files(&self, project_dir: &Path, files: &[StaticFile]) -> Result<()> {
        for file in files {
            let file_path = project_dir.join(&file.embedded_path);

            // Create parent directories
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).await?;
            }

            // Write file content
            fs::write(&file_path, &file.content).await?;

            // Set permissions on Unix systems
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let permissions = std::fs::Permissions::from_mode(file.permissions);
                std::fs::set_permissions(&file_path, permissions)?;
            }
        }

        Ok(())
    }

    fn find_compiled_binary(&self, project_dir: &Path, target_triple: &str) -> Result<PathBuf> {
        let binary_path = project_dir
            .join("target")
            .join(target_triple)
            .join("release");

        // Try common binary names
        let possible_names = vec!["rustle-runner", "main"];

        for name in possible_names {
            let path = binary_path.join(name);
            if path.exists() {
                return Ok(path);
            }

            // Try with .exe extension on Windows
            let exe_path = binary_path.join(format!("{name}.exe"));
            if exe_path.exists() {
                return Ok(exe_path);
            }
        }

        Err(DeployError::CompilationFailed {
            target: target_triple.to_string(),
            reason: "Compiled binary not found".to_string(),
        })
    }

    fn compress_binary(&self, binary_data: &[u8]) -> Result<Vec<u8>> {
        use flate2::{write::GzEncoder, Compression};
        use std::io::Write;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
        encoder
            .write_all(binary_data)
            .map_err(|e| DeployError::compilation(format!("Compression failed: {e}")))?;

        encoder
            .finish()
            .map_err(|e| DeployError::compilation(format!("Compression failed: {e}")))
    }

    fn calculate_checksum(&self, data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    fn calculate_compilation_checksum(
        &self,
        compilation: &BinaryCompilation,
        compiled_modules: &[crate::modules::CompiledModule],
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(compilation.checksum.as_bytes());

        // Include module checksums
        for module in compiled_modules {
            hasher.update(module.spec.name.as_bytes());
            if let Some(version) = &module.spec.version {
                hasher.update(version.as_bytes());
            }
            if let Some(checksum) = &module.spec.checksum {
                hasher.update(checksum.as_bytes());
            }
            // Include a hash of the compiled code
            let mut code_hasher = Sha256::new();
            code_hasher.update(module.compiled_code.as_bytes());
            hasher.update(code_hasher.finalize());
        }

        format!("{:x}", hasher.finalize())
    }

    fn generate_cargo_toml_with_modules(
        &self,
        compilation: &BinaryCompilation,
        compiled_modules: &[crate::modules::CompiledModule],
    ) -> Result<String> {
        let mut cargo_toml = format!(
            r#"
[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = {{ version = "1", features = ["derive"] }}
serde_json = "1"
tokio = {{ version = "1", features = ["full"] }}
anyhow = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
async-trait = "0.1"
"#,
            compilation.binary_name
        );

        // Add module-specific dependencies (simplified for now)
        let mut additional_deps = std::collections::HashSet::new();
        for module in compiled_modules {
            // Add common dependencies based on module type
            match &module.spec.source {
                crate::execution::plan::ModuleSource::Http { .. } => {
                    additional_deps
                        .insert("reqwest = { version = \"0.11\", features = [\"json\"] }");
                }
                crate::execution::plan::ModuleSource::File { .. } => {
                    additional_deps.insert("walkdir = \"2.4\"");
                }
                _ => {}
            }
        }

        for dep in additional_deps {
            cargo_toml.push_str(dep);
            cargo_toml.push('\n');
        }

        Ok(cargo_toml)
    }

    fn generate_main_rs_with_modules(
        &self,
        compilation: &BinaryCompilation,
        compiled_modules: &[crate::modules::CompiledModule],
    ) -> Result<String> {
        // Use the runtime template generator for consistency
        let template_generator = crate::compiler::RuntimeTemplateGenerator::new()?;

        // Convert execution plan
        let execution_plan: crate::execution::ExecutionPlan =
            serde_json::from_str(&compilation.embedded_data.execution_plan).map_err(|e| {
                DeployError::TemplateGeneration(format!("Failed to parse execution plan: {e}"))
            })?;

        let main_rs = template_generator.generate_main_rs_with_modules(
            &execution_plan,
            &compilation.embedded_data.runtime_config,
            compiled_modules,
        )?;

        Ok(main_rs)
    }

    async fn write_module_files(
        &self,
        project_dir: &Path,
        compiled_modules: &[crate::modules::CompiledModule],
    ) -> Result<()> {
        let modules_dir = project_dir.join("src").join("modules");
        fs::create_dir_all(&modules_dir).await?;

        // Write each compiled module as a separate file
        for (i, module) in compiled_modules.iter().enumerate() {
            let module_file = modules_dir.join(format!("module_{i}.rs"));
            fs::write(&module_file, &module.compiled_code).await?;
        }

        // Write module index file
        let mut mod_rs = String::new();
        for i in 0..compiled_modules.len() {
            mod_rs.push_str(&format!("pub mod module_{i};\n"));
        }

        fs::write(modules_dir.join("mod.rs"), mod_rs).await?;

        debug!("Module files written successfully");
        Ok(())
    }
}

// Add Compilation error variant to DeployError
impl DeployError {
    pub fn compilation(msg: String) -> Self {
        DeployError::Configuration(msg) // Reuse Configuration for now
    }
}
