//! Test helper utilities for file operations testing

pub mod assertions;
pub mod builders;
pub mod environment;
pub mod fixtures;

// Configuration for tests
#[derive(Clone)]
pub struct TestConfig {
    pub timeout_seconds: u64,
    pub temp_dir_prefix: String,
    pub preserve_temp_files: bool,
    pub max_file_size: u64,
    pub enable_property_tests: bool,
    pub property_test_cases: u32,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: 30,
            temp_dir_prefix: "rustle_test_".to_string(),
            preserve_temp_files: std::env::var("RUSTLE_TEST_PRESERVE_TEMP").is_ok(),
            max_file_size: 100 * 1024 * 1024,
            enable_property_tests: std::env::var("RUSTLE_TEST_SKIP_PROPERTY").is_err(),
            property_test_cases: 100,
        }
    }
}
