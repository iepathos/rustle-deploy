//! End-to-end workflow integration tests

use crate::modules::files::{
    assert_file_exists, assert_is_directory, CopyTestBuilder, FileTestBuilder, StatTestBuilder,
    TemplateTestBuilder, TestEnvironment, TestFixtures,
};
use rustle_deploy::modules::files::FileState;
use serde_json::Value;

/// Test complete file workflow: create directory → copy template → process template → verify with stat
#[tokio::test]
async fn test_complete_file_workflow() {
    let env = TestEnvironment::new();
    let fixtures = TestFixtures::load();

    // Step 1: Create directory structure
    let config_dir = env.temp_path("app/config");
    let args = FileTestBuilder::new()
        .path(config_dir.to_string_lossy())
        .state(FileState::Directory)
        .mode("0755")
        .build();

    let result = env.execute_module("file", args).await.unwrap();
    assert!(result.changed);
    assert!(!result.failed);
    assert_is_directory(&config_dir);

    // Step 2: Copy template file
    let template_content = fixtures.get_template("config").unwrap();
    let template_src = env.create_test_file("app.conf.j2", template_content);
    let template_dest = env.temp_path("app/config/app.conf.j2");

    let copy_args = CopyTestBuilder::new()
        .src(template_src.to_string_lossy())
        .dest(template_dest.to_string_lossy())
        .mode("0644")
        .build();

    let result = env.execute_module("copy", copy_args).await.unwrap();
    assert!(result.changed);
    assert!(!result.failed);
    assert_file_exists(&template_dest);

    // Step 3: Process template
    let config_file = env.temp_path("app/config/app.conf");
    let template_args = TemplateTestBuilder::new()
        .src(template_dest.to_string_lossy())
        .dest(config_file.to_string_lossy())
        .variables(TestFixtures::config_template_vars())
        .build();

    let result = env.execute_module("template", template_args).await.unwrap();
    assert!(result.changed);
    assert!(!result.failed);
    assert_file_exists(&config_file);

    // Step 4: Verify with stat
    let stat_args = StatTestBuilder::new()
        .path(config_file.to_string_lossy())
        .get_checksum(true)
        .build();

    let result = env.execute_module("stat", stat_args).await.unwrap();
    assert!(!result.failed);

    let stat_data = &result.ansible_facts;
    let stat_info = stat_data.get("stat").unwrap();
    if let Value::Object(stat_obj) = stat_info {
        assert_eq!(stat_obj.get("exists").unwrap(), &Value::Bool(true));
        assert_eq!(stat_obj.get("isreg").unwrap(), &Value::Bool(true));
        assert!(stat_obj.contains_key("checksum"));
    }

    // Verify rendered content contains expected values
    let content = env.read_file("app/config/app.conf").unwrap();
    assert!(content.contains("name = \"test_app\""));
    assert!(content.contains("port = 8080"));
    assert!(content.contains("debug = true"));
}

/// Test backup and restore workflow
#[tokio::test]
async fn test_backup_and_restore_workflow() {
    let env = TestEnvironment::new();

    // Create original file
    let original_content = "original configuration\nversion = 1.0";
    let file_path = env.create_test_file("important.conf", original_content);

    // Copy with backup to replace content
    let new_content = "updated configuration\nversion = 2.0";
    let temp_src = env.create_test_file("new.conf", new_content);

    let copy_args = CopyTestBuilder::new()
        .src(temp_src.to_string_lossy())
        .dest(file_path.to_string_lossy())
        .backup(true)
        .build();

    let result = env.execute_module("copy", copy_args).await.unwrap();
    assert!(result.changed);
    assert!(!result.failed);

    // Verify backup was created
    let backup_path = format!("{}.backup", file_path.to_string_lossy());
    assert_file_exists(&backup_path);

    let backup_content = env
        .read_file(&format!("{}.backup", "important.conf"))
        .unwrap();
    assert_eq!(backup_content, original_content);

    let current_content = env.read_file("important.conf").unwrap();
    assert_eq!(current_content, new_content);

    // Verify with stat that both files exist
    let stat_args = StatTestBuilder::new()
        .path(file_path.to_string_lossy())
        .get_checksum(true)
        .build();

    let result = env.execute_module("stat", stat_args).await.unwrap();
    assert!(!result.failed);

    let stat_data = &result.ansible_facts;
    let stat_info = stat_data.get("stat").unwrap();
    if let Value::Object(stat_obj) = stat_info {
        assert_eq!(stat_obj.get("exists").unwrap(), &Value::Bool(true));
    }
}

/// Test multi-environment deployment workflow
#[tokio::test]
async fn test_multi_environment_deployment() {
    let env = TestEnvironment::new();
    let fixtures = TestFixtures::load();

    let environments = vec!["development", "staging", "production"];

    for environment in environments {
        // Create environment-specific directory
        let env_dir = env.temp_path(&format!("config/{environment}"));
        let dir_args = FileTestBuilder::new()
            .path(env_dir.to_string_lossy())
            .state(FileState::Directory)
            .build();

        let result = env.execute_module("file", dir_args).await.unwrap();
        assert!(result.changed);
        assert_is_directory(&env_dir);

        // Deploy environment-specific configuration
        let template_content = fixtures.get_template("config").unwrap();
        let template_path =
            env.create_test_file(&format!("{environment}_config.j2"), template_content);

        let config_file = env.temp_path(&format!("config/{environment}/app.conf"));

        let mut vars = TestFixtures::config_template_vars();
        vars.insert(
            "environment".to_string(),
            Value::String(environment.to_string()),
        );

        let template_args = TemplateTestBuilder::new()
            .src(template_path.to_string_lossy())
            .dest(config_file.to_string_lossy())
            .variables(vars)
            .build();

        let result = env.execute_module("template", template_args).await.unwrap();
        assert!(result.changed);
        assert_file_exists(&config_file);

        // Verify content is environment-specific
        let content = env
            .read_file(&format!("config/{environment}/app.conf"))
            .unwrap();
        assert!(content.contains("test_app"));
    }
}

/// Test configuration management workflow with validation
#[tokio::test]
async fn test_config_management_with_validation() {
    let env = TestEnvironment::new();

    // Step 1: Create configuration directory
    let config_dir = env.temp_path("etc/myapp");
    let dir_args = FileTestBuilder::new()
        .path(config_dir.to_string_lossy())
        .state(FileState::Directory)
        .mode("0755")
        .build();

    let result = env.execute_module("file", dir_args).await.unwrap();
    assert!(result.changed);

    // Step 2: Deploy main configuration
    let main_config_template = r#"
[server]
host = "{{ server_host | default('localhost') }}"
port = {{ server_port | default(8080) }}

[database]
url = "{{ db_url }}"
pool_size = {{ db_pool_size | default(10) }}

[logging]
level = "{{ log_level | default('info') }}"
"#;

    let template_path = env.create_test_file("main_config.j2", main_config_template);
    let main_config = env.temp_path("etc/myapp/main.conf");

    let mut config_vars = std::collections::HashMap::new();
    config_vars.insert(
        "server_host".to_string(),
        Value::String("0.0.0.0".to_string()),
    );
    config_vars.insert("server_port".to_string(), Value::Number(9000.into()));
    config_vars.insert(
        "db_url".to_string(),
        Value::String("postgresql://localhost/myapp".to_string()),
    );
    config_vars.insert("db_pool_size".to_string(), Value::Number(20.into()));
    config_vars.insert("log_level".to_string(), Value::String("debug".to_string()));

    let template_args = TemplateTestBuilder::new()
        .src(template_path.to_string_lossy())
        .dest(main_config.to_string_lossy())
        .variables(config_vars)
        .mode("0644")
        .build();

    let result = env.execute_module("template", template_args).await.unwrap();
    assert!(result.changed);

    // Step 3: Create additional configuration files
    let log_config = env.temp_path("etc/myapp/logging.conf");
    let log_args = FileTestBuilder::new()
        .path(log_config.to_string_lossy())
        .state(FileState::Present)
        .mode("0644")
        .build();

    let result = env.execute_module("file", log_args).await.unwrap();
    assert!(result.changed);

    // Step 4: Validate all configurations exist and have correct properties
    for config_file in &["main.conf", "logging.conf"] {
        let file_path = env.temp_path(&format!("etc/myapp/{config_file}"));

        let stat_args = StatTestBuilder::new()
            .path(file_path.to_string_lossy())
            .get_checksum(true)
            .build();

        let result = env.execute_module("stat", stat_args).await.unwrap();
        assert!(!result.failed);

        let stat_data = &result.ansible_facts;
        let stat_info = stat_data.get("stat").unwrap();
        if let Value::Object(stat_obj) = stat_info {
            assert_eq!(stat_obj.get("exists").unwrap(), &Value::Bool(true));
            assert_eq!(stat_obj.get("isreg").unwrap(), &Value::Bool(true));
        }
    }

    // Verify main configuration content
    let main_content = env.read_file("etc/myapp/main.conf").unwrap();
    assert!(main_content.contains("host = \"0.0.0.0\""));
    assert!(main_content.contains("port = 9000"));
    assert!(main_content.contains("postgresql://localhost/myapp"));
}

/// Test file synchronization workflow
#[tokio::test]
async fn test_file_synchronization_workflow() {
    let env = TestEnvironment::new();

    // Create source directory with files
    let _src_dir = env.create_test_directory("source");
    env.create_test_file("source/file1.txt", "content1");
    env.create_test_file("source/file2.txt", "content2");
    env.create_test_directory("source/subdir");
    env.create_test_file("source/subdir/file3.txt", "content3");

    // Create destination directory
    let dest_dir = env.temp_path("destination");
    let dir_args = FileTestBuilder::new()
        .path(dest_dir.to_string_lossy())
        .state(FileState::Directory)
        .build();

    let result = env.execute_module("file", dir_args).await.unwrap();
    assert!(result.changed);

    // Synchronize files
    let files_to_sync = vec!["file1.txt", "file2.txt", "subdir/file3.txt"];

    for file in files_to_sync {
        let src_file = env.temp_path(&format!("source/{file}"));
        let dest_file = env.temp_path(&format!("destination/{file}"));

        // Create destination subdirectory if needed
        if let Some(parent) = dest_file.parent() {
            if !parent.exists() {
                let parent_args = FileTestBuilder::new()
                    .path(parent.to_string_lossy())
                    .state(FileState::Directory)
                    .build();

                let result = env.execute_module("file", parent_args).await.unwrap();
                assert!(!result.failed);
            }
        }

        // Copy file
        let copy_args = CopyTestBuilder::new()
            .src(src_file.to_string_lossy())
            .dest(dest_file.to_string_lossy())
            .build();

        let result = env.execute_module("copy", copy_args).await.unwrap();
        assert!(result.changed);
        assert_file_exists(&dest_file);
    }

    // Verify synchronization with stat
    for file in &["file1.txt", "file2.txt", "subdir/file3.txt"] {
        let dest_file = env.temp_path(&format!("destination/{file}"));

        let stat_args = StatTestBuilder::new()
            .path(dest_file.to_string_lossy())
            .get_checksum(true)
            .build();

        let result = env.execute_module("stat", stat_args).await.unwrap();
        assert!(!result.failed);

        let stat_data = &result.ansible_facts;
        let stat_info = stat_data.get("stat").unwrap();
        if let Value::Object(stat_obj) = stat_info {
            assert_eq!(stat_obj.get("exists").unwrap(), &Value::Bool(true));
            assert!(stat_obj.contains_key("checksum"));
        }
    }
}

/// Test rollback workflow
#[tokio::test]
async fn test_rollback_workflow() {
    let env = TestEnvironment::new();

    // Deploy initial version
    let config_file = env.create_test_file("app.conf", "version=1.0\nfeature_x=false");

    // Deploy new version with backup
    let new_config_content = "version=2.0\nfeature_x=true\nfeature_y=true";
    let new_config_src = env.create_test_file("app_v2.conf", new_config_content);

    let copy_args = CopyTestBuilder::new()
        .src(new_config_src.to_string_lossy())
        .dest(config_file.to_string_lossy())
        .backup(true)
        .build();

    let result = env.execute_module("copy", copy_args).await.unwrap();
    assert!(result.changed);

    // Verify new version is deployed
    let current_content = env.read_file("app.conf").unwrap();
    assert!(current_content.contains("version=2.0"));
    assert!(current_content.contains("feature_y=true"));

    // Simulate rollback by restoring from backup
    let backup_file = format!("{}.backup", config_file.to_string_lossy());
    assert_file_exists(&backup_file);

    let rollback_args = CopyTestBuilder::new()
        .src(&backup_file)
        .dest(config_file.to_string_lossy())
        .force(true)
        .build();

    let result = env.execute_module("copy", rollback_args).await.unwrap();
    assert!(result.changed);

    // Verify rollback was successful
    let rolled_back_content = env.read_file("app.conf").unwrap();
    assert!(rolled_back_content.contains("version=1.0"));
    assert!(rolled_back_content.contains("feature_x=false"));
    assert!(!rolled_back_content.contains("feature_y"));
}
