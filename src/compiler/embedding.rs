use crate::deploy::{DeployError, Result};
use crate::types::*;

pub struct DataEmbedder;

impl DataEmbedder {
    pub fn new() -> Self {
        Self
    }

    pub fn embed_execution_data(
        &self,
        execution_plan: &str,
        modules: &[ModuleImplementation],
        static_files: &[StaticFile],
        runtime_config: &crate::runtime::RuntimeConfig,
    ) -> Result<EmbeddedExecutionData> {
        // Validate execution plan
        self.validate_execution_plan(execution_plan)?;

        // Validate modules
        for module in modules {
            self.validate_module(module)?;
        }

        // Validate static files
        for file in static_files {
            self.validate_static_file(file)?;
        }

        // Create embedded data structure
        let embedded_data = EmbeddedExecutionData {
            execution_plan: execution_plan.to_string(),
            module_implementations: modules.to_vec(),
            static_files: static_files.to_vec(),
            runtime_config: runtime_config.clone(),
            facts_template: vec![], // TODO: Extract from execution plan
        };

        Ok(embedded_data)
    }

    pub fn generate_embedding_code(&self, embedded_data: &EmbeddedExecutionData) -> Result<String> {
        let mut code = String::new();

        // Generate constants for embedded data
        code.push_str(&self.generate_execution_plan_constant(&embedded_data.execution_plan)?);
        code.push_str(&self.generate_runtime_config_constant(&embedded_data.runtime_config)?);
        code.push_str(&self.generate_static_files_code(&embedded_data.static_files)?);
        code.push_str(&self.generate_modules_code(&embedded_data.module_implementations)?);

        Ok(code)
    }

    pub fn calculate_embedding_size(&self, embedded_data: &EmbeddedExecutionData) -> u64 {
        let mut total_size = 0u64;

        // Size of execution plan JSON
        total_size += embedded_data.execution_plan.len() as u64;

        // Size of runtime config JSON
        if let Ok(config_json) = serde_json::to_string(&embedded_data.runtime_config) {
            total_size += config_json.len() as u64;
        }

        // Size of static files
        for file in &embedded_data.static_files {
            total_size += file.content.len() as u64;
        }

        // Size of module source code
        for module in &embedded_data.module_implementations {
            total_size += module.source_code.len() as u64;
        }

        total_size
    }

    pub fn optimize_embedding(&self, embedded_data: &mut EmbeddedExecutionData) -> Result<()> {
        // Remove unnecessary whitespace from execution plan
        if let Ok(plan_value) =
            serde_json::from_str::<serde_json::Value>(&embedded_data.execution_plan)
        {
            embedded_data.execution_plan = serde_json::to_string(&plan_value)?;
        }

        // Compress large static files
        for file in &mut embedded_data.static_files {
            if file.content.len() > 1024 * 1024 {
                // Files larger than 1MB
                file.content = self.compress_data(&file.content)?;
            }
        }

        // Remove comments and unnecessary whitespace from module source code
        for module in &mut embedded_data.module_implementations {
            module.source_code = self.minify_rust_code(&module.source_code)?;
        }

        Ok(())
    }

    // Private helper methods

    fn validate_execution_plan(&self, execution_plan: &str) -> Result<()> {
        // Validate that execution plan is valid JSON
        let _: serde_json::Value = serde_json::from_str(execution_plan).map_err(|e| {
            DeployError::TemplateGeneration(format!("Invalid execution plan JSON: {e}"))
        })?;

        // Additional validation could include:
        // - Schema validation
        // - Task dependency validation
        // - Resource requirement checks

        Ok(())
    }

    fn validate_module(&self, module: &ModuleImplementation) -> Result<()> {
        // Basic validation of module implementation
        if module.module_name.is_empty() {
            return Err(DeployError::TemplateGeneration(
                "Module name cannot be empty".to_string(),
            ));
        }

        if module.source_code.is_empty() {
            return Err(DeployError::TemplateGeneration(
                "Module source code cannot be empty".to_string(),
            ));
        }

        // Validate that module name is a valid Rust identifier
        if !module
            .module_name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_')
        {
            return Err(DeployError::TemplateGeneration(
                "Invalid module name".to_string(),
            ));
        }

        // TODO: Validate Rust syntax of source code

        Ok(())
    }

    fn validate_static_file(&self, file: &StaticFile) -> Result<()> {
        if file.embedded_path.is_empty() {
            return Err(DeployError::TemplateGeneration(
                "Embedded path cannot be empty".to_string(),
            ));
        }

        if file.target_path.is_empty() {
            return Err(DeployError::TemplateGeneration(
                "Target path cannot be empty".to_string(),
            ));
        }

        // Validate path safety
        if file.embedded_path.contains("..") || file.target_path.contains("..") {
            return Err(DeployError::TemplateGeneration(
                "Path traversal not allowed".to_string(),
            ));
        }

        Ok(())
    }

    fn generate_execution_plan_constant(&self, execution_plan: &str) -> Result<String> {
        // Use serde_json to properly escape the string
        let escaped_plan = serde_json::to_string(execution_plan)?;

        Ok(format!(
            "pub const EMBEDDED_EXECUTION_PLAN: &str = {escaped_plan};\n"
        ))
    }

    fn generate_runtime_config_constant(
        &self,
        runtime_config: &crate::runtime::RuntimeConfig,
    ) -> Result<String> {
        let config_json = serde_json::to_string(runtime_config)?;
        let escaped_config = serde_json::to_string(&config_json)?;

        Ok(format!(
            "pub const EMBEDDED_RUNTIME_CONFIG: &str = {escaped_config};\n"
        ))
    }

    fn generate_static_files_code(&self, static_files: &[StaticFile]) -> Result<String> {
        let mut code = String::new();

        code.push_str(
            "pub fn get_embedded_files() -> std::collections::HashMap<String, Vec<u8>> {\n",
        );
        code.push_str("    let mut files = std::collections::HashMap::new();\n");

        for (index, file) in static_files.iter().enumerate() {
            // Generate a unique constant name for each file
            let const_name = format!("EMBEDDED_FILE_{index}");

            // Generate the file data constant
            code.push_str(&format!(
                "    const {}: &[u8] = &{:?};\n",
                const_name, file.content
            ));

            // Insert into the map
            code.push_str(&format!(
                "    files.insert(\"{}\".to_string(), {}.to_vec());\n",
                file.embedded_path, const_name
            ));
        }

        code.push_str("    files\n");
        code.push_str("}\n\n");

        Ok(code)
    }

    fn generate_modules_code(&self, modules: &[ModuleImplementation]) -> Result<String> {
        let mut code = String::new();

        code.push_str("pub mod embedded_modules {\n");
        code.push_str("    use super::*;\n\n");

        for module in modules {
            let module_name = module.module_name.replace('-', "_");
            code.push_str(&format!("    pub mod {module_name} {{\n"));
            code.push_str("        use super::*;\n\n");

            // Add the module source code
            code.push_str(&self.indent_code(&module.source_code, 2));

            code.push_str("    }\n\n");
        }

        code.push_str("}\n\n");

        Ok(code)
    }

    fn indent_code(&self, code: &str, level: usize) -> String {
        let indent = "    ".repeat(level);
        code.lines()
            .map(|line| {
                if line.trim().is_empty() {
                    line.to_string()
                } else {
                    format!("{indent}{line}")
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn compress_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        use flate2::{write::GzEncoder, Compression};
        use std::io::Write;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
        encoder
            .write_all(data)
            .map_err(|e| DeployError::TemplateGeneration(format!("Compression failed: {e}")))?;

        encoder
            .finish()
            .map_err(|e| DeployError::TemplateGeneration(format!("Compression failed: {e}")))
    }

    fn minify_rust_code(&self, code: &str) -> Result<String> {
        // Simple minification - remove comments and extra whitespace
        let mut minified = String::new();
        let mut in_string = false;
        let mut in_comment = false;
        let mut chars = code.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                '"' if !in_comment => {
                    in_string = !in_string;
                    minified.push(ch);
                }
                '/' if !in_string && !in_comment => {
                    if let Some('/') = chars.peek() {
                        in_comment = true;
                        chars.next(); // consume the second '/'
                    } else {
                        minified.push(ch);
                    }
                }
                '\n' if in_comment => {
                    in_comment = false;
                    minified.push(ch);
                }
                _ if in_comment => {
                    // Skip comment characters
                }
                ' ' | '\t' if !in_string => {
                    // Compress multiple whitespace into single space
                    if !minified.ends_with(' ') {
                        minified.push(' ');
                    }
                }
                _ => {
                    minified.push(ch);
                }
            }
        }

        Ok(minified)
    }
}

impl Default for DataEmbedder {
    fn default() -> Self {
        Self::new()
    }
}
