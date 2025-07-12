use crate::modules::ast_parser::AstParser;
use crate::modules::error::{CompileError, GenerationError};
use crate::modules::loader::{CompiledModule, LoadedModule};
use anyhow::Result;
use handlebars::Handlebars;
use serde_json::json;
use tracing::info;

/// Code generator for module compilation
pub struct CodeGenerator {
    template_engine: Handlebars<'static>,
    ast_parser: AstParser,
}

impl Default for CodeGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeGenerator {
    pub fn new() -> Self {
        let mut template_engine = Handlebars::new();
        template_engine.set_strict_mode(true);

        // Register module wrapper template
        let wrapper_template = include_str!("../../templates/modules/module_wrapper.rs.template");
        template_engine
            .register_template_string("module_wrapper", wrapper_template)
            .expect("Failed to register module wrapper template");

        Self {
            template_engine,
            ast_parser: AstParser::new(),
        }
    }

    pub async fn compile_module(
        &self,
        module: &LoadedModule,
        target_triple: &str,
    ) -> Result<CompiledModule, CompileError> {
        info!(
            "Compiling module '{}' for target '{}'",
            module.spec.name, target_triple
        );

        // Generate wrapper code for the module
        let wrapper_code = self.generate_module_wrapper(module)?;

        // Generate registration code
        let registration_code = self.generate_registration_code(module)?;

        // Compile any static data
        let static_data = self.compile_static_data(module).await?;

        Ok(CompiledModule {
            spec: module.spec.clone(),
            compiled_code: wrapper_code,
            registration_code,
            static_data,
        })
    }

    pub fn generate_module_wrapper(&self, module: &LoadedModule) -> Result<String, CompileError> {
        let struct_name = self.to_struct_name(&module.spec.name);

        // Prepare template context
        let context = json!({
            "module_name": module.spec.name,
            "struct_name": struct_name,
            "description": module.manifest.description.as_deref().unwrap_or(""),
            "required_args": module.manifest.required_args,
            "optional_args": module.manifest.optional_args,
            "module_implementation": self.generate_module_implementation(module)?,
            "module_source_code": self.prepare_module_source(module)?,
            "additional_imports": self.generate_imports(module)?,
            "security_wrapper_start": self.generate_security_wrapper_start(module)?,
            "security_wrapper_end": self.generate_security_wrapper_end(module)?,
        });

        self.template_engine
            .render("module_wrapper", &context)
            .map_err(|e| CompileError::TemplateError {
                template: "module_wrapper".to_string(),
                error: e.to_string(),
            })
    }

    pub fn generate_registration_code(
        &self,
        module: &LoadedModule,
    ) -> Result<String, CompileError> {
        let struct_name = self.to_struct_name(&module.spec.name);
        Ok(format!(
            r#"registry.register("{}", Box::new({}::new()));"#,
            module.spec.name, struct_name
        ))
    }

    pub fn generate_module_registry(
        &self,
        modules: &[CompiledModule],
    ) -> Result<String, GenerationError> {
        let mut code = String::new();

        // Include all module files
        code.push_str("// Auto-generated module registry\n\n");

        for (i, _module) in modules.iter().enumerate() {
            code.push_str(&format!("mod module_{i};\n"));
        }

        code.push('\n');
        code.push_str("use crate::modules::registry::ModuleRegistry;\n\n");
        code.push_str("pub fn register_all_modules(registry: &mut ModuleRegistry) {\n");

        // Add registration calls
        for module in modules {
            code.push_str(&format!("    // Register module: {}\n", module.spec.name));
            code.push_str(&format!("    {}\n", module.registration_code));
        }

        code.push_str("}\n");

        Ok(code)
    }

    fn generate_module_implementation(
        &self,
        module: &LoadedModule,
    ) -> Result<String, CompileError> {
        let source = &module.source_code.main_file;

        // Use AST parser to extract the execute function
        match self.ast_parser.extract_execute_function(source)? {
            Some(implementation) => Ok(self.adapt_implementation(&implementation, module)),
            None => {
                // If no execute function found, generate a default one
                Ok(self.generate_default_implementation(module))
            }
        }
    }

    fn prepare_module_source(&self, module: &LoadedModule) -> Result<String, CompileError> {
        let source = &module.source_code.main_file;

        // Use AST parser to properly prepare the source
        let mut prepared = self.ast_parser.prepare_module_source(source)?;

        // Add module implementation marker
        prepared.push_str("\n\n// Module implementation injected by rustle-deploy\n");

        Ok(prepared)
    }

    fn generate_imports(&self, _module: &LoadedModule) -> Result<String, CompileError> {
        // Add common imports (simplified for now)
        let imports = [
            "use std::fs;",
            "use std::path::{Path, PathBuf};",
            "use tokio::process::Command;",
        ];

        Ok(imports.join("\n"))
    }

    fn generate_security_wrapper_start(
        &self,
        _module: &LoadedModule,
    ) -> Result<String, CompileError> {
        // Simplified security wrapper for now
        Ok(String::new())
    }

    fn generate_security_wrapper_end(
        &self,
        _module: &LoadedModule,
    ) -> Result<String, CompileError> {
        // Security guard will be dropped automatically
        Ok(String::new())
    }

    async fn compile_static_data(&self, _module: &LoadedModule) -> Result<Vec<u8>, CompileError> {
        // Compile any static data that needs to be embedded with the module
        // For now, just return empty data
        Ok(Vec::new())
    }

    fn to_struct_name(&self, module_name: &str) -> String {
        // Convert module name to a valid Rust struct name
        let mut name = String::new();
        let mut capitalize_next = true;

        for ch in module_name.chars() {
            if ch.is_alphanumeric() {
                if capitalize_next {
                    name.extend(ch.to_uppercase());
                    capitalize_next = false;
                } else {
                    name.push(ch);
                }
            } else {
                capitalize_next = true;
            }
        }

        name.push_str("Module");
        name
    }

    fn adapt_implementation(&self, implementation: &str, _module: &LoadedModule) -> String {
        // Adapt the implementation to match our module interface
        let mut adapted = implementation.to_string();

        // Replace function signature
        adapted = adapted.replace(
            "fn execute",
            "fn execute(&self, args: &HashMap<String, Value>, context: &ExecutionContext) -> Result<ModuleResult, ModuleError>"
        );

        // Skip advanced security adaptations for now

        adapted
    }

    fn generate_default_implementation(&self, module: &LoadedModule) -> String {
        format!(
            r#"
        // Default implementation for module '{}'
        let module_name = "{}";
        
        // Extract and validate arguments
        let mut result = ModuleResult::default();
        
        // Module implementation would go here
        result.failed = true;
        result.message = Some(format!("Module '{{}}' is not yet implemented", module_name));
        
        Ok(result)
        "#,
            module.spec.name, module.spec.name
        )
    }
}
