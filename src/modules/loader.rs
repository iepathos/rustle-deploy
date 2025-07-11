use crate::execution::plan::{ExecutionPlan, ModuleSource, ModuleSpec};
use crate::modules::cache::ModuleCache;
use crate::modules::compiler::CodeGenerator;
use crate::modules::error::{CompileError, ModuleError, ValidationError};
use crate::modules::resolver::{ModuleSourceCode, ModuleSourceResolver};
use crate::modules::validator::ModuleValidator;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;

/// Module compiler that processes modules from execution plans
pub struct ModuleCompiler {
    module_cache: ModuleCache,
    source_resolvers: Vec<Box<dyn ModuleSourceResolver>>,
    validator: ModuleValidator,
    code_generator: CodeGenerator,
}

impl ModuleCompiler {
    pub fn new(cache_dir: PathBuf) -> Self {
        use crate::modules::resolver::{
            FileSystemResolver, GitResolver, HttpResolver, RegistryResolver,
        };

        Self {
            module_cache: ModuleCache::new(cache_dir, Duration::from_secs(86400)), // 24 hour TTL
            source_resolvers: vec![
                Box::new(FileSystemResolver::new(vec![])),
                Box::new(GitResolver::new()),
                Box::new(HttpResolver::new()),
                Box::new(RegistryResolver::new()),
            ],
            validator: ModuleValidator::new(),
            code_generator: CodeGenerator::new(),
        }
    }

    pub async fn extract_module_requirements(
        &self,
        execution_plan: &ExecutionPlan,
    ) -> Result<Vec<ModuleSpec>, ModuleError> {
        let mut required_modules = HashSet::new();
        let mut module_specs = Vec::new();

        // Scan all tasks for required modules
        for task in &execution_plan.tasks {
            required_modules.insert(task.module.clone());
        }

        // Extract module specifications from execution plan
        for module_name in required_modules {
            let spec = self.extract_module_spec(&module_name, &execution_plan.modules)?;
            module_specs.push(spec);
        }

        // Resolve dependencies
        let resolved_specs = self.resolve_dependencies(&module_specs)?;

        Ok(resolved_specs)
    }

    pub async fn load_module(&mut self, spec: &ModuleSpec) -> Result<LoadedModule, ModuleError> {
        // Check cache first
        if let Some(cached) = self.module_cache.get(spec).await {
            return Ok(cached);
        }

        // Load module source
        let source_code = self.load_module_source(&spec.source).await?;

        // Parse module manifest
        let manifest = self.parse_module_manifest(&source_code)?;

        // Load dependencies recursively (simplified for now)
        let mut resolved_dependencies = Vec::new();
        for dep_name in &spec.dependencies {
            let dep_spec = ModuleSpec {
                name: dep_name.clone(),
                version: Some("latest".to_string()),
                source: self.resolve_dependency_source(dep_name, "latest")?,
                checksum: None,
                dependencies: vec![],
                static_link: true,
            };
            let loaded_dep = Box::pin(self.load_module(&dep_spec)).await?;
            resolved_dependencies.push(loaded_dep);
        }

        let loaded = LoadedModule {
            spec: spec.clone(),
            source_code,
            manifest,
            resolved_dependencies,
        };

        // Validate module
        self.validator.validate_module(&loaded)?;

        // Cache the loaded module
        self.module_cache.store(loaded.clone()).await?;

        Ok(loaded)
    }

    pub async fn load_module_source(
        &self,
        source: &ModuleSource,
    ) -> Result<ModuleSourceCode, ModuleError> {
        // Try each resolver until one can handle this source
        for resolver in &self.source_resolvers {
            if resolver.can_resolve(source) {
                return resolver
                    .resolve(source)
                    .await
                    .map_err(|e| ModuleError::LoadError {
                        location: format!("{source:?}"),
                        error: e.to_string(),
                    });
            }
        }

        Err(ModuleError::UnsupportedSource(format!("{source:?}")))
    }

    pub fn validate_module(&self, module: &LoadedModule) -> Result<(), ValidationError> {
        let result = self.validator.validate_module(module)?;
        if !result.passed {
            return Err(ValidationError::Failed {
                errors: result.errors,
                warnings: result.warnings,
            });
        }
        Ok(())
    }

    pub async fn compile_module(
        &self,
        module: &LoadedModule,
        target: &str,
    ) -> Result<CompiledModule, CompileError> {
        self.code_generator.compile_module(module, target).await
    }

    pub fn resolve_dependencies(
        &self,
        modules: &[ModuleSpec],
    ) -> Result<Vec<ModuleSpec>, ModuleError> {
        // Simple topological sort for now
        // TODO: Implement proper dependency resolution with version constraints
        let mut resolved = Vec::new();
        let mut seen = HashSet::new();

        for module in modules {
            self.resolve_module_dependencies(module, &mut resolved, &mut seen)?;
        }

        Ok(resolved)
    }

    fn resolve_module_dependencies(
        &self,
        module: &ModuleSpec,
        resolved: &mut Vec<ModuleSpec>,
        seen: &mut HashSet<String>,
    ) -> Result<(), ModuleError> {
        if seen.contains(&module.name) {
            return Ok(());
        }

        seen.insert(module.name.clone());

        // Resolve dependencies first
        for dep_name in &module.dependencies {
            let dep_spec = ModuleSpec {
                name: dep_name.clone(),
                version: Some("latest".to_string()),
                source: self.resolve_dependency_source(dep_name, "latest")?,
                checksum: None,
                dependencies: vec![],
                static_link: true,
            };
            self.resolve_module_dependencies(&dep_spec, resolved, seen)?;
        }

        resolved.push(module.clone());
        Ok(())
    }

    pub fn generate_module_registry(
        &self,
        modules: &[CompiledModule],
    ) -> Result<String, ModuleError> {
        self.code_generator
            .generate_module_registry(modules)
            .map_err(|e| ModuleError::CompilationError {
                error: e.to_string(),
            })
    }

    fn extract_module_spec(
        &self,
        module_name: &str,
        defined_modules: &[ModuleSpec],
    ) -> Result<ModuleSpec, ModuleError> {
        // Check if module is explicitly defined in execution plan
        if let Some(spec) = defined_modules.iter().find(|m| m.name == module_name) {
            return Ok(spec.clone());
        }

        // Check if it's a built-in module
        if self.is_builtin_module(module_name) {
            return Ok(ModuleSpec {
                name: module_name.to_string(),
                version: Some("builtin".to_string()),
                source: ModuleSource::Builtin,
                checksum: None,
                dependencies: vec![],
                static_link: true,
            });
        }

        // Try to resolve from default sources
        Err(ModuleError::ModuleNotFound {
            name: module_name.to_string(),
            searched_sources: vec!["builtin", "execution_plan"]
                .into_iter()
                .map(String::from)
                .collect(),
        })
    }

    fn is_builtin_module(&self, name: &str) -> bool {
        matches!(
            name,
            "debug"
                | "command"
                | "shell"
                | "copy"
                | "template"
                | "package"
                | "service"
                | "file"
                | "setup"
                | "apt"
                | "yum"
                | "pacman"
                | "systemd"
                | "launchd"
                | "winservice"
        )
    }

    fn parse_module_manifest(
        &self,
        _source_code: &ModuleSourceCode,
    ) -> Result<ModuleManifest, ModuleError> {
        // For now, extract from module source code comments
        // TODO: Support proper manifest files
        Ok(ModuleManifest {
            name: String::new(),
            version: "0.1.0".to_string(),
            description: None,
            entry_point: "execute".to_string(),
            required_args: vec![],
            optional_args: vec![],
            return_type: "ModuleResult".to_string(),
            side_effects: vec![],
            capabilities: vec![],
        })
    }

    fn resolve_dependency_source(
        &self,
        name: &str,
        _version: &str,
    ) -> Result<ModuleSource, ModuleError> {
        // For now, assume dependencies are built-in
        if self.is_builtin_module(name) {
            Ok(ModuleSource::Builtin)
        } else {
            Err(ModuleError::DependencyNotFound {
                name: name.to_string(),
                version_req: _version.to_string(),
            })
        }
    }
}

// Re-export types from execution plan (avoid duplicate imports)

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityLevel {
    Trusted,   // Full system access
    Sandboxed, // Limited filesystem/network access
    Isolated,  // No system access
}

/// Loaded module with source code and metadata
#[derive(Debug, Clone)]
pub struct LoadedModule {
    pub spec: ModuleSpec,
    pub source_code: ModuleSourceCode,
    pub manifest: ModuleManifest,
    pub resolved_dependencies: Vec<LoadedModule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleManifest {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub entry_point: String,
    pub required_args: Vec<String>,
    pub optional_args: Vec<String>,
    pub return_type: String,
    pub side_effects: Vec<String>,
    pub capabilities: Vec<String>,
}

/// Compiled module ready for embedding
#[derive(Debug, Clone)]
pub struct CompiledModule {
    pub spec: ModuleSpec,
    pub compiled_code: String,
    pub registration_code: String,
    pub static_data: Vec<u8>,
}
