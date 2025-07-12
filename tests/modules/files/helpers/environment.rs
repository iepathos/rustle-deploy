//! Test environment setup and management

use crate::modules::files::helpers::TestConfig;
use anyhow::Result;
use rustle_deploy::modules::files::{CopyModule, FileModule, StatModule, TemplateModule};
use rustle_deploy::modules::interface::{
    ExecutionContext, ExecutionModule, HostInfo, ModuleArgs, ModuleResult, SpecialParameters,
};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::{tempdir, TempDir};

/// Test environment that provides isolated testing capabilities
pub struct TestEnvironment {
    temp_dir: TempDir,
    context: ExecutionContext,
    config: TestConfig,
}

impl Clone for TestEnvironment {
    fn clone(&self) -> Self {
        // Create a new TestEnvironment with the same config
        // Note: This creates a separate temp directory
        Self::with_config(self.config.clone())
    }
}

impl Default for TestEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

impl TestEnvironment {
    /// Get access to the execution context
    pub fn context(&self) -> &ExecutionContext {
        &self.context
    }

    /// Get access to the config
    pub fn config(&self) -> &TestConfig {
        &self.config
    }
    /// Create a new test environment with default configuration
    pub fn new() -> Self {
        Self::with_config(TestConfig::default())
    }

    /// Create a test environment with custom configuration
    pub fn with_config(config: TestConfig) -> Self {
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let context = ExecutionContext {
            facts: HashMap::new(),
            variables: HashMap::new(),
            host_info: HostInfo::detect(),
            working_directory: temp_dir.path().to_path_buf(),
            environment: HashMap::new(),
            check_mode: false,
            diff_mode: false,
            verbosity: 0,
        };

        Self {
            temp_dir,
            context,
            config,
        }
    }

    /// Create a test environment configured for a specific platform
    pub fn with_platform_config(platform: &str) -> Self {
        let config = TestConfig {
            temp_dir_prefix: format!("rustle_test_{platform}_"),
            ..Default::default()
        };
        Self::with_config(config)
    }

    /// Execute a module with the given arguments
    pub async fn execute_module(&self, name: &str, args: ModuleArgs) -> Result<ModuleResult> {
        self.execute_module_with_context(name, args, &self.context)
            .await
    }

    /// Execute a module with a custom context
    pub async fn execute_module_with_context(
        &self,
        name: &str,
        args: ModuleArgs,
        context: &ExecutionContext,
    ) -> Result<ModuleResult> {
        let result = match name {
            "file" => {
                let file_module = FileModule;
                file_module.execute(&args, context).await
            }
            "copy" => {
                let copy_module = CopyModule;
                copy_module.execute(&args, context).await
            }
            "stat" => {
                let stat_module = StatModule;
                stat_module.execute(&args, context).await
            }
            "template" => {
                let template_module = TemplateModule;
                template_module.execute(&args, context).await
            }
            _ => anyhow::bail!("Unknown module: {}", name),
        };

        result.map_err(|e| anyhow::anyhow!("Module execution failed: {}", e))
    }

    /// Create a test file with the given content
    pub fn create_test_file(&self, relative_path: &str, content: &str) -> PathBuf {
        let file_path = self.temp_path(relative_path);

        // Create parent directories if they don't exist
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).expect("Failed to create parent directories");
        }

        std::fs::write(&file_path, content).expect("Failed to write test file");
        file_path
    }

    /// Create a test file with binary content
    pub fn create_test_file_binary(&self, relative_path: &str, content: &[u8]) -> PathBuf {
        let file_path = self.temp_path(relative_path);

        // Create parent directories if they don't exist
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).expect("Failed to create parent directories");
        }

        std::fs::write(&file_path, content).expect("Failed to write test file");
        file_path
    }

    /// Create a test directory
    pub fn create_test_directory(&self, relative_path: &str) -> PathBuf {
        let dir_path = self.temp_path(relative_path);
        std::fs::create_dir_all(&dir_path).expect("Failed to create test directory");
        dir_path
    }

    /// Get the absolute path for a relative path within the temp directory
    pub fn temp_path(&self, relative_path: &str) -> PathBuf {
        self.temp_dir.path().join(relative_path)
    }

    /// Get the path to a fixture file
    pub fn fixture_path(&self, relative_path: &str) -> PathBuf {
        PathBuf::from("tests/fixtures/files").join(relative_path)
    }

    /// Check if a file exists in the temp directory
    pub fn file_exists(&self, relative_path: &str) -> bool {
        self.temp_path(relative_path).exists()
    }

    /// Read file content from temp directory
    pub fn read_file(&self, relative_path: &str) -> Result<String> {
        let content = std::fs::read_to_string(self.temp_path(relative_path))?;
        Ok(content)
    }

    /// Read binary file content from temp directory
    pub fn read_file_binary(&self, relative_path: &str) -> Result<Vec<u8>> {
        let content = std::fs::read(self.temp_path(relative_path))?;
        Ok(content)
    }

    /// Get file metadata from temp directory
    pub fn file_metadata(&self, relative_path: &str) -> Result<std::fs::Metadata> {
        let metadata = std::fs::metadata(self.temp_path(relative_path))?;
        Ok(metadata)
    }

    /// Set file permissions (Unix only)
    #[cfg(unix)]
    pub fn set_file_permissions(&self, relative_path: &str, mode: u32) -> Result<()> {
        use std::os::unix::fs::PermissionsExt;
        let path = self.temp_path(relative_path);
        let permissions = std::fs::Permissions::from_mode(mode);
        std::fs::set_permissions(path, permissions)?;
        Ok(())
    }

    /// Copy a fixture file to the temp directory
    pub fn copy_fixture(
        &self,
        fixture_relative_path: &str,
        dest_relative_path: &str,
    ) -> Result<PathBuf> {
        let fixture_path = self.fixture_path(fixture_relative_path);
        let dest_path = self.temp_path(dest_relative_path);

        // Create parent directories if they don't exist
        if let Some(parent) = dest_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::copy(&fixture_path, &dest_path)?;
        Ok(dest_path)
    }
}

// Note: Drop implementation removed as TempDir cannot be moved out of borrowed context
// Users can check RUSTLE_TEST_PRESERVE_TEMP environment variable for debugging

/// Helper trait for converting JSON to ModuleArgs
pub trait FromJson {
    fn from_json(value: Value) -> Self;
}

impl FromJson for ModuleArgs {
    fn from_json(value: Value) -> Self {
        match value {
            Value::Object(map) => ModuleArgs {
                args: map.into_iter().collect(),
                special: SpecialParameters::default(),
            },
            _ => panic!("Expected JSON object for ModuleArgs"),
        }
    }
}
