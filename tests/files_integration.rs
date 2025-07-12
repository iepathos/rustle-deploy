//! Integration tests for file operations modules
//!
//! This module provides comprehensive testing for the file operations modules
//! according to specification 200.

mod modules {
    pub mod files;
}

// Re-export for convenience
pub use modules::files::*;

#[cfg(test)]
mod comprehensive_tests {
    use super::*;

    /// Basic smoke test to ensure test infrastructure works
    #[tokio::test]
    async fn test_infrastructure_smoke_test() {
        let env = TestEnvironment::new();

        // Test that we can create files
        let test_file = env.create_test_file("smoke_test.txt", "hello world");
        assert_file_exists(&test_file);

        // Test that we can read files
        let content = env.read_file("smoke_test.txt").unwrap();
        assert_eq!(content, "hello world");

        // Test that TestFixtures work
        let fixtures = TestFixtures::load();
        assert!(fixtures.get_template("simple").is_some());
        assert!(fixtures.get_sample_file("small_text").is_some());
    }

    /// Test that all modules can be executed
    #[tokio::test]
    async fn test_all_modules_executable() {
        let env = TestEnvironment::new();

        // Test file module
        let file_args = FileTestBuilder::new()
            .path(env.temp_path("test.txt").to_string_lossy())
            .state(rustle_deploy::modules::files::FileState::Present)
            .build();

        let result = env.execute_module("file", file_args).await;
        assert!(result.is_ok(), "File module should be executable");

        // Test copy module
        let src = env.create_test_file("copy_src.txt", "content");
        let copy_args = CopyTestBuilder::new()
            .src(src.to_string_lossy())
            .dest(env.temp_path("copy_dest.txt").to_string_lossy())
            .build();

        let result = env.execute_module("copy", copy_args).await;
        assert!(result.is_ok(), "Copy module should be executable");

        // Test stat module
        let stat_args = StatTestBuilder::new().path(src.to_string_lossy()).build();

        let result = env.execute_module("stat", stat_args).await;
        assert!(result.is_ok(), "Stat module should be executable");

        // Test template module
        let template_src = env.create_test_file("template.j2", "Hello {{ name }}!");
        let template_args = TemplateTestBuilder::new()
            .src(template_src.to_string_lossy())
            .dest(env.temp_path("template_out.txt").to_string_lossy())
            .variable("name", "World")
            .build();

        let result = env.execute_module("template", template_args).await;
        assert!(result.is_ok(), "Template module should be executable");
    }
}
