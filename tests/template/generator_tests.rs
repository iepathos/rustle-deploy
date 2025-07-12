use rustle_deploy::template::{
    BinaryTemplateGenerator, TemplateConfig, TargetInfo, OptimizationLevel, CompressionType,
};
use rustle_deploy::execution::rustle_plan::{
    RustlePlanOutput, BinaryDeploymentPlan, RustlePlanMetadata, PlanningOptions, PlayPlan,
    TaskBatch, TaskPlan, StaticFileRef, SecretRef, CompilationRequirements,
    SecretSource, TaskCondition, RiskLevel,
};
use rustle_deploy::execution::plan::ExecutionStrategy;
use rustle_deploy::types::Platform;
use chrono::Utc;
use std::collections::HashMap;
use std::time::Duration;
use tempfile;

fn create_test_execution_plan() -> RustlePlanOutput {
    RustlePlanOutput {
        metadata: RustlePlanMetadata {
            created_at: Utc::now(),
            rustle_plan_version: "1.0.0".to_string(),
            playbook_hash: "test-hash".to_string(),
            inventory_hash: "test-inventory-hash".to_string(),
            planning_options: PlanningOptions {
                limit: None,
                tags: vec![],
                skip_tags: vec![],
                check_mode: false,
                diff_mode: false,
                forks: 5,
                serial: None,
                strategy: ExecutionStrategy::Linear,
                binary_threshold: 10,
                force_binary: false,
                force_ssh: false,
            },
        },
        plays: vec![PlayPlan {
            play_id: "play-1".to_string(),
            name: "Test Play".to_string(),
            strategy: ExecutionStrategy::Linear,
            serial: None,
            hosts: vec!["test-host".to_string()],
            batches: vec![TaskBatch {
                batch_id: "batch-1".to_string(),
                hosts: vec!["test-host".to_string()],
                tasks: vec![TaskPlan {
                    task_id: "task-1".to_string(),
                    name: "Test Task".to_string(),
                    module: "command".to_string(),
                    args: {
                        let mut args = HashMap::new();
                        args.insert("cmd".to_string(), serde_json::Value::String("echo hello".to_string()));
                        args
                    },
                    hosts: vec!["test-host".to_string()],
                    dependencies: vec![],
                    conditions: vec![],
                    tags: vec![],
                    notify: vec![],
                    execution_order: 1,
                    can_run_parallel: true,
                    estimated_duration: Duration::from_secs(5),
                    risk_level: RiskLevel::Low,
                }],
                parallel_groups: vec![],
                dependencies: vec![],
                estimated_duration: Some(Duration::from_secs(10)),
            }],
            handlers: vec![],
            estimated_duration: Some(Duration::from_secs(20)),
        }],
        binary_deployments: vec![],
        total_tasks: 1,
        estimated_duration: Some(Duration::from_secs(30)),
        estimated_compilation_time: Some(Duration::from_secs(60)),
        parallelism_score: 0.8,
        network_efficiency_score: 0.9,
        hosts: vec!["test-host".to_string()],
    }
}

fn create_test_binary_deployment() -> BinaryDeploymentPlan {
    BinaryDeploymentPlan {
        deployment_id: "deployment-1".to_string(),
        target_hosts: vec!["test-host".to_string()],
        target_architecture: "x86_64".to_string(),
        task_ids: vec!["task-1".to_string()],
        estimated_savings: Duration::from_secs(30),
        compilation_requirements: CompilationRequirements {
            modules: vec!["command".to_string()],
            static_files: vec![],
            target_triple: "x86_64-unknown-linux-gnu".to_string(),
            optimization_level: "release".to_string(),
            features: vec![],
        },
        controller_endpoint: Some("http://localhost:8080/api/results".to_string()),
        execution_timeout: Some(Duration::from_secs(300)),
        report_interval: Some(Duration::from_secs(30)),
        cleanup_on_completion: Some(true),
        log_level: Some("info".to_string()),
        max_retries: Some(3),
        static_files: vec![],
        secrets: vec![],
        verbose: Some(false),
    }
}

fn create_test_target_info() -> TargetInfo {
    TargetInfo {
        target_triple: "x86_64-unknown-linux-gnu".to_string(),
        platform: Platform::Linux,
        architecture: "x86_64".to_string(),
        os_family: "unix".to_string(),
        libc: Some("glibc".to_string()),
        features: vec![],
    }
}

#[tokio::test]
async fn test_basic_template_generation() {
    let config = TemplateConfig {
        optimization_level: OptimizationLevel::Release,
        ..Default::default()
    };
    
    let generator = BinaryTemplateGenerator::new(config).expect("Failed to create generator");
    
    let execution_plan = create_test_execution_plan();
    let binary_deployment = create_test_binary_deployment();
    let target_info = create_test_target_info();
    
    let template = generator
        .generate_binary_template(&execution_plan, &binary_deployment, &target_info)
        .await
        .expect("Failed to generate template");
    
    // Verify template structure
    assert!(!template.template_id.is_empty());
    assert!(template.source_files.contains_key(&std::path::PathBuf::from("src/main.rs")));
    assert!(!template.cargo_toml.is_empty());
    assert_eq!(template.target_info.target_triple, "x86_64-unknown-linux-gnu");
    assert!(template.estimated_binary_size > 0);
    
    // Verify main.rs contains expected content
    let main_rs = template.source_files.get(&std::path::PathBuf::from("src/main.rs")).unwrap();
    assert!(main_rs.contains("mod embedded_data"));
    assert!(main_rs.contains("mod modules"));
    assert!(main_rs.contains("mod runtime"));
    assert!(main_rs.contains("async fn main()"));
    
    // Verify Cargo.toml contains expected dependencies
    assert!(template.cargo_toml.contains("tokio"));
    assert!(template.cargo_toml.contains("serde"));
    assert!(template.cargo_toml.contains("anyhow"));
}

#[tokio::test]
async fn test_template_with_static_files() {
    let config = TemplateConfig::default();
    let generator = BinaryTemplateGenerator::new(config).expect("Failed to create generator");
    
    let execution_plan = create_test_execution_plan();
    let mut binary_deployment = create_test_binary_deployment();
    
    // Add static files
    binary_deployment.static_files = vec![
        StaticFileRef {
            source_path: "/test/config.yaml".to_string(),
            target_path: "config.yaml".to_string(),
            permissions: Some(0o644),
            compress: true,
        },
    ];
    
    let target_info = create_test_target_info();
    
    let template = generator
        .generate_binary_template(&execution_plan, &binary_deployment, &target_info)
        .await
        .expect("Failed to generate template");
    
    // Verify static files are embedded
    assert!(!template.embedded_data.static_files.is_empty());
}

#[tokio::test]
async fn test_template_with_secrets() {
    let config = TemplateConfig::default();
    let generator = BinaryTemplateGenerator::new(config).expect("Failed to create generator");
    
    let execution_plan = create_test_execution_plan();
    let mut binary_deployment = create_test_binary_deployment();
    
    // Add secrets
    binary_deployment.secrets = vec![
        SecretRef {
            key: "api_key".to_string(),
            source: SecretSource::Environment { var: "API_KEY".to_string() },
            target_env_var: Some("API_KEY".to_string()),
        },
    ];
    
    let target_info = create_test_target_info();
    
    let template = generator
        .generate_binary_template(&execution_plan, &binary_deployment, &target_info)
        .await
        .expect("Failed to generate template");
    
    // Verify secrets are embedded
    assert!(!template.embedded_data.secrets.vault_data.is_empty() || 
            template.embedded_data.secrets.encryption_key_id != "none");
}

#[test]
fn test_cargo_toml_generation() {
    let config = TemplateConfig::default();
    let generator = BinaryTemplateGenerator::new(config).expect("Failed to create generator");
    
    let dependencies = vec![
        rustle_deploy::template::ModuleDependency {
            name: "test-dep".to_string(),
            version: "1.0".to_string(),
            features: vec!["feature1".to_string()],
        },
    ];
    
    let cargo_toml = generator
        .generate_cargo_toml(&dependencies, "x86_64-unknown-linux-gnu")
        .expect("Failed to generate Cargo.toml");
    
    assert!(cargo_toml.contains("test-dep"));
    assert!(cargo_toml.contains("1.0"));
    assert!(cargo_toml.contains("[profile.release]"));
    assert!(cargo_toml.contains("opt-level = 3"));
}

#[test]
fn test_main_rs_generation() {
    let config = TemplateConfig::default();
    let generator = BinaryTemplateGenerator::new(config).expect("Failed to create generator");
    
    let execution_plan = create_test_execution_plan();
    let embedded_data = rustle_deploy::template::EmbeddedData {
        execution_plan: serde_json::to_string(&execution_plan).unwrap(),
        static_files: HashMap::new(),
        module_binaries: HashMap::new(),
        runtime_config: rustle_deploy::runtime::RuntimeConfig {
            controller_endpoint: None,
            execution_timeout: Duration::from_secs(300),
            report_interval: Duration::from_secs(30),
            cleanup_on_completion: true,
            log_level: "info".to_string(),
            heartbeat_interval: Duration::from_secs(60),
            max_retries: 3,
        },
        secrets: rustle_deploy::template::EncryptedSecrets {
            vault_data: HashMap::new(),
            encryption_key_id: "test".to_string(),
            decryption_method: "none".to_string(),
        },
        facts_cache: None,
    };
    
    let main_rs = generator
        .generate_main_rs(&execution_plan, &embedded_data)
        .expect("Failed to generate main.rs");
    
    assert!(main_rs.contains("async fn main()"));
    assert!(main_rs.contains("embedded_data::EXECUTION_PLAN"));
    assert!(main_rs.contains("embedded_data::RUNTIME_CONFIG"));
    assert!(main_rs.contains("LocalExecutor"));
}

#[tokio::test]
async fn test_module_implementations_generation() {
    let config = TemplateConfig::default();
    let generator = BinaryTemplateGenerator::new(config).expect("Failed to create generator");
    
    let modules = vec![
        rustle_deploy::execution::plan::ModuleSpec {
            name: "command".to_string(),
            source: rustle_deploy::execution::plan::ModuleSource::Builtin,
            version: Some("1.0.0".to_string()),
            checksum: None,
            dependencies: vec![],
        },
        rustle_deploy::execution::plan::ModuleSpec {
            name: "package".to_string(),
            source: rustle_deploy::execution::plan::ModuleSource::Builtin,
            version: Some("1.0.0".to_string()),
            checksum: None,
            dependencies: vec![],
        },
    ];
    
    let implementations = generator
        .generate_module_implementations(&modules, &Platform::Linux)
        .expect("Failed to generate module implementations");
    
    assert!(implementations.contains_key("modules/command.rs"));
    assert!(implementations.contains_key("modules/package.rs"));
    
    let command_impl = implementations.get("modules/command.rs").unwrap();
    assert!(command_impl.contains("pub async fn execute"));
    assert!(command_impl.contains("HashMap<String, Value>"));
}

#[tokio::test]
async fn test_template_optimization() {
    let config = TemplateConfig {
        optimization_level: OptimizationLevel::Aggressive,
        ..Default::default()
    };
    
    let generator = BinaryTemplateGenerator::new(config).expect("Failed to create generator");
    
    let execution_plan = create_test_execution_plan();
    let binary_deployment = create_test_binary_deployment();
    let target_info = create_test_target_info();
    
    let mut template = generator
        .generate_binary_template(&execution_plan, &binary_deployment, &target_info)
        .await
        .expect("Failed to generate template");
    
    let original_size = template.estimated_binary_size;
    
    let optimized_template = generator
        .optimize_template(&template, OptimizationLevel::Aggressive)
        .await
        .expect("Failed to optimize template");
    
    // Verify optimization flags are added
    assert!(optimized_template.compilation_flags.contains(&"-C".to_string()));
    assert!(optimized_template.compilation_flags.contains(&"target-cpu=native".to_string()));
}

#[tokio::test]
async fn test_file_operations_template_generation() {
    // Test that file and copy modules can be generated correctly
    let config = TemplateConfig::default();
    let generator = BinaryTemplateGenerator::new(config).expect("Failed to create generator");
    
    let modules = vec![
        rustle_deploy::execution::plan::ModuleSpec {
            name: "file".to_string(),
            source: rustle_deploy::execution::plan::ModuleSource::Builtin,
            version: Some("1.0.0".to_string()),
            checksum: None,
            dependencies: vec![],
            static_link: false,
        },
        rustle_deploy::execution::plan::ModuleSpec {
            name: "copy".to_string(),
            source: rustle_deploy::execution::plan::ModuleSource::Builtin,
            version: Some("1.0.0".to_string()),
            checksum: None,
            dependencies: vec![],
            static_link: false,
        },
    ];
    
    let implementations = generator
        .generate_module_implementations(&modules, &Platform::Linux)
        .expect("Failed to generate module implementations");
    
    // Verify file and copy modules are generated
    assert!(implementations.contains_key("modules/file.rs"));
    assert!(implementations.contains_key("modules/copy.rs"));
    assert!(implementations.contains_key("modules/mod.rs"));
    
    // Verify file module implementation
    let file_impl = implementations.get("modules/file.rs").unwrap();
    assert!(file_impl.contains("pub async fn execute"));
    assert!(file_impl.contains("HashMap<String, Value>"));
    assert!(file_impl.contains("state"));
    assert!(file_impl.contains("directory"));
    assert!(file_impl.contains("fs::create_dir"));
    
    // Verify copy module implementation
    let copy_impl = implementations.get("modules/copy.rs").unwrap();
    assert!(copy_impl.contains("pub async fn execute"));
    assert!(copy_impl.contains("HashMap<String, Value>"));
    assert!(copy_impl.contains("src"));
    assert!(copy_impl.contains("dest"));
    assert!(copy_impl.contains("fs::copy"));
    
    // Verify mod.rs declares the modules
    let mod_rs = implementations.get("modules/mod.rs").unwrap();
    assert!(mod_rs.contains("pub mod file;"));
    assert!(mod_rs.contains("pub mod copy;"));
}

#[tokio::test]
async fn test_file_operations_plan_fixture_template() {
    // Test template generation from the actual file_operations_plan.json fixture
    let config = TemplateConfig::default();
    let generator = BinaryTemplateGenerator::new(config).expect("Failed to create generator");
    
    // Load the fixture
    let fixture_path = "tests/fixtures/execution_plans/file_operations_plan.json";
    let fixture_content = std::fs::read_to_string(fixture_path)
        .expect("Failed to read file_operations_plan.json fixture");
    
    let rustle_plan: RustlePlanOutput = serde_json::from_str(&fixture_content)
        .expect("Failed to parse file_operations_plan.json");
    
    // Extract the binary deployment plan
    assert!(!rustle_plan.binary_deployments.is_empty(), "Fixture should have binary deployments");
    let binary_deployment = &rustle_plan.binary_deployments[0];
    
    let target_info = TargetInfo {
        target_triple: "x86_64-unknown-linux-gnu".to_string(),
        platform: Platform::Linux,
        architecture: "x86_64".to_string(),
        os_family: "unix".to_string(),
        libc: Some("glibc".to_string()),
        features: vec![],
    };
    
    // Generate template
    let template = generator
        .generate_binary_template(&rustle_plan, binary_deployment, &target_info)
        .await
        .expect("Failed to generate template from file_operations_plan.json");
    
    // Verify template structure
    assert!(!template.template_id.is_empty());
    assert!(template.source_files.contains_key(&std::path::PathBuf::from("src/main.rs")));
    assert!(!template.cargo_toml.is_empty());
    
    // Verify that the main.rs file contains references to file and copy modules
    let main_rs = template.source_files.get(&std::path::PathBuf::from("src/main.rs")).unwrap();
    assert!(main_rs.contains("modules::file::execute"));
    assert!(main_rs.contains("modules::copy::execute"));
    
    // Verify module files are generated
    assert!(template.source_files.contains_key(&std::path::PathBuf::from("src/modules/file.rs")));
    assert!(template.source_files.contains_key(&std::path::PathBuf::from("src/modules/copy.rs")));
    
    // Verify the template can be written to disk without syntax errors
    let temp_dir = tempfile::TempDir::new().unwrap();
    template.write_to_directory(temp_dir.path()).await
        .expect("Failed to write template to directory");
    
    // Verify Cargo.toml and main.rs exist
    assert!(temp_dir.path().join("Cargo.toml").exists());
    assert!(temp_dir.path().join("src/main.rs").exists());
    assert!(temp_dir.path().join("src/modules/file.rs").exists());
    assert!(temp_dir.path().join("src/modules/copy.rs").exists());
}

#[tokio::test]
async fn test_template_caching() {
    let config = TemplateConfig {
        cache_templates: true,
        ..Default::default()
    };
    
    let generator = BinaryTemplateGenerator::new(config).expect("Failed to create generator");
    
    let execution_plan = create_test_execution_plan();
    let binary_deployment = create_test_binary_deployment();
    let target_info = create_test_target_info();
    
    // Generate template first time
    let template1 = generator
        .generate_binary_template(&execution_plan, &binary_deployment, &target_info)
        .await
        .expect("Failed to generate template");
    
    // Generate template second time (should use cache)
    let template2 = generator
        .generate_binary_template(&execution_plan, &binary_deployment, &target_info)
        .await
        .expect("Failed to generate template");
    
    // Templates should be identical (from cache)
    assert_eq!(template1.cache_key, template2.cache_key);
    assert_eq!(template1.template_id, template2.template_id);
}