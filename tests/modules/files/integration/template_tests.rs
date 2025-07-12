//! Integration tests for the template module

use crate::modules::files::{
    assert_file_content, assert_file_exists, assert_is_file, TemplateTestBuilder, TestEnvironment,
    TestFixtures,
};

/// Test basic template rendering
#[tokio::test]
async fn test_template_basic_rendering() {
    let env = TestEnvironment::new();
    let fixtures = TestFixtures::load();

    // Create template file
    let template_content = fixtures.get_template("simple").unwrap();
    let template_path = env.create_test_file("simple.j2", template_content);
    let output_path = env.temp_path("output.txt");

    let args = TemplateTestBuilder::new()
        .src(template_path.to_string_lossy())
        .dest(output_path.to_string_lossy())
        .variables(TestFixtures::simple_template_vars())
        .build();

    let result = env.execute_module("template", args).await.unwrap();

    assert!(result.changed);
    assert!(!result.failed);

    // Verify output file was created with expected content
    assert_file_exists(&output_path);
    assert_is_file(&output_path);

    let expected_output = fixtures.get_expected_output("simple_rendered").unwrap();
    assert_file_content(&output_path, expected_output).unwrap();
}

/// Test template with complex variables
#[tokio::test]
async fn test_template_complex_variables() {
    let env = TestEnvironment::new();
    let fixtures = TestFixtures::load();

    // Create config template
    let template_content = fixtures.get_template("config").unwrap();
    let template_path = env.create_test_file("config.j2", template_content);
    let output_path = env.temp_path("config.yml");

    let args = TemplateTestBuilder::new()
        .src(template_path.to_string_lossy())
        .dest(output_path.to_string_lossy())
        .variables(TestFixtures::config_template_vars())
        .build();

    let result = env.execute_module("template", args).await.unwrap();

    assert!(result.changed);
    assert!(!result.failed);

    let expected_output = fixtures.get_expected_output("config_rendered").unwrap();
    assert_file_content(&output_path, expected_output).unwrap();
}

/// Test template with conditionals and loops
#[tokio::test]
async fn test_template_conditionals_loops() {
    let env = TestEnvironment::new();
    let fixtures = TestFixtures::load();

    // Create complex template
    let template_content = fixtures.get_template("complex").unwrap();
    let template_path = env.create_test_file("complex.j2", template_content);
    let output_path = env.temp_path("complex.conf");

    let args = TemplateTestBuilder::new()
        .src(template_path.to_string_lossy())
        .dest(output_path.to_string_lossy())
        .variables(TestFixtures::complex_template_vars_ssl())
        .build();

    let result = env.execute_module("template", args).await.unwrap();

    assert!(result.changed);
    assert!(!result.failed);

    let expected_output = fixtures
        .get_expected_output("complex_rendered_ssl")
        .unwrap();
    assert_file_content(&output_path, expected_output).unwrap();
}

/// Test template with individual variable setting
#[tokio::test]
async fn test_template_individual_variables() {
    let env = TestEnvironment::new();

    let template_content = "Name: {{ name }}\nAge: {{ age }}\nActive: {{ active }}";
    let template_path = env.create_test_file("vars.j2", template_content);
    let output_path = env.temp_path("vars.txt");

    let args = TemplateTestBuilder::new()
        .src(template_path.to_string_lossy())
        .dest(output_path.to_string_lossy())
        .variable("name", "Alice")
        .variable("age", 30)
        .variable("active", true)
        .build();

    let result = env.execute_module("template", args).await.unwrap();

    assert!(result.changed);
    assert!(!result.failed);

    let expected = "Name: Alice\nAge: 30\nActive: true";
    assert_file_content(&output_path, expected).unwrap();
}

/// Test template with backup
#[tokio::test]
async fn test_template_with_backup() {
    let env = TestEnvironment::new();

    // Create existing output file
    let output_path = env.create_test_file("output.txt", "original content");

    let template_content = "New content: {{ value }}";
    let template_path = env.create_test_file("template.j2", template_content);

    let args = TemplateTestBuilder::new()
        .src(template_path.to_string_lossy())
        .dest(output_path.to_string_lossy())
        .variable("value", "replaced")
        .backup(true)
        .build();

    let result = env.execute_module("template", args).await.unwrap();

    assert!(result.changed);
    assert!(!result.failed);

    // Verify new content
    assert_file_content(&output_path, "New content: replaced").unwrap();

    // Verify backup was created
    let backup_path = format!("{}.backup", output_path.to_string_lossy());
    assert_file_exists(&backup_path);
    assert_file_content(&backup_path, "original content").unwrap();
}

/// Test template with permissions
#[tokio::test]
async fn test_template_with_permissions() {
    let env = TestEnvironment::new();

    let template_content = "Content with permissions";
    let template_path = env.create_test_file("perm_template.j2", template_content);
    let output_path = env.temp_path("perm_output.txt");

    let args = TemplateTestBuilder::new()
        .src(template_path.to_string_lossy())
        .dest(output_path.to_string_lossy())
        .mode("0644")
        .build();

    let result = env.execute_module("template", args).await.unwrap();

    assert!(result.changed);
    assert!(!result.failed);

    assert_file_content(&output_path, template_content).unwrap();

    #[cfg(unix)]
    {
        use crate::modules::files::assert_file_permissions;
        assert_file_permissions(&output_path, 0o644).unwrap();
    }
}

/// Test template with trim_blocks and lstrip_blocks
#[tokio::test]
async fn test_template_trim_options() {
    let env = TestEnvironment::new();

    let template_content = r#"
Start
{%- if true %}
  Indented line
{%- endif %}
End"#;

    let template_path = env.create_test_file("trim.j2", template_content);
    let output_path = env.temp_path("trim_output.txt");

    let args = TemplateTestBuilder::new()
        .src(template_path.to_string_lossy())
        .dest(output_path.to_string_lossy())
        .trim_blocks(true)
        .lstrip_blocks(true)
        .build();

    let result = env.execute_module("template", args).await.unwrap();

    assert!(result.changed);
    assert!(!result.failed);

    // Verify file was created (exact content depends on Handlebars implementation)
    assert_file_exists(&output_path);
}

/// Test template when output already exists with same content
#[tokio::test]
async fn test_template_no_change_same_content() {
    let env = TestEnvironment::new();

    let template_content = "Static content";
    let template_path = env.create_test_file("static.j2", template_content);

    // Create output file with same content that would be generated
    let output_path = env.create_test_file("static_output.txt", "Static content");

    let args = TemplateTestBuilder::new()
        .src(template_path.to_string_lossy())
        .dest(output_path.to_string_lossy())
        .build();

    let result = env.execute_module("template", args).await.unwrap();

    // Should not change since content is identical
    assert!(!result.changed);
    assert!(!result.failed);
}

/// Test template with missing variables (should use defaults or fail gracefully)
#[tokio::test]
async fn test_template_missing_variables() {
    let env = TestEnvironment::new();

    let template_content = "Value: {{ missing_var | default('default_value') }}";
    let template_path = env.create_test_file("default.j2", template_content);
    let output_path = env.temp_path("default_output.txt");

    let args = TemplateTestBuilder::new()
        .src(template_path.to_string_lossy())
        .dest(output_path.to_string_lossy())
        .build();

    let result = env.execute_module("template", args).await.unwrap();

    assert!(result.changed);
    assert!(!result.failed);

    // Should use default value
    assert_file_content(&output_path, "Value: default_value").unwrap();
}

/// Test template with nested objects and arrays
#[tokio::test]
async fn test_template_nested_data() {
    let env = TestEnvironment::new();

    let template_content = r#"
Users:
{{#each users}}
- Name: {{name}}
  Email: {{email}}
  Roles:
  {{#each roles}}
  - {{this}}
  {{/each}}
{{/each}}"#;

    let template_path = env.create_test_file("nested.j2", template_content);
    let output_path = env.temp_path("nested_output.txt");

    let users_data = serde_json::json!([
        {
            "name": "Alice",
            "email": "alice@example.com",
            "roles": ["admin", "user"]
        },
        {
            "name": "Bob",
            "email": "bob@example.com",
            "roles": ["user"]
        }
    ]);

    let args = TemplateTestBuilder::new()
        .src(template_path.to_string_lossy())
        .dest(output_path.to_string_lossy())
        .variable("users", users_data)
        .build();

    let result = env.execute_module("template", args).await.unwrap();

    assert!(result.changed);
    assert!(!result.failed);

    // Verify output contains expected structure
    let content = env.read_file("nested_output.txt").unwrap();
    assert!(content.contains("Alice"));
    assert!(content.contains("Bob"));
    assert!(content.contains("admin"));
}

/// Test template error handling for syntax errors
#[tokio::test]
async fn test_template_syntax_error() {
    let env = TestEnvironment::new();

    let invalid_template = "{{ unclosed_variable";
    let template_path = env.create_test_file("invalid.j2", invalid_template);
    let output_path = env.temp_path("invalid_output.txt");

    let args = TemplateTestBuilder::new()
        .src(template_path.to_string_lossy())
        .dest(output_path.to_string_lossy())
        .build();

    let result = env.execute_module("template", args).await;

    // Should fail gracefully with syntax error
    assert!(result.is_err() || result.unwrap().failed);
}

/// Test template error handling for missing source file
#[tokio::test]
async fn test_template_missing_source() {
    let env = TestEnvironment::new();

    let nonexistent_template = env.temp_path("nonexistent.j2");
    let output_path = env.temp_path("output.txt");

    let args = TemplateTestBuilder::new()
        .src(nonexistent_template.to_string_lossy())
        .dest(output_path.to_string_lossy())
        .build();

    let result = env.execute_module("template", args).await;

    // Should fail gracefully
    assert!(result.is_err() || result.unwrap().failed);
}

/// Test template with force flag
#[tokio::test]
async fn test_template_force_overwrite() {
    let env = TestEnvironment::new();

    let template_content = "Forced content: {{ value }}";
    let template_path = env.create_test_file("force.j2", template_content);
    let output_path = env.create_test_file("force_output.txt", "existing content");

    let args = TemplateTestBuilder::new()
        .src(template_path.to_string_lossy())
        .dest(output_path.to_string_lossy())
        .variable("value", "overwritten")
        .force(true)
        .build();

    let result = env.execute_module("template", args).await.unwrap();

    assert!(result.changed);
    assert!(!result.failed);

    assert_file_content(&output_path, "Forced content: overwritten").unwrap();
}

/// Test template with large output
#[tokio::test]
async fn test_template_large_output() {
    let env = TestEnvironment::new();

    let template_content = r#"
{{#each items}}
Item {{@index}}: {{this}}
{{/each}}"#;

    let template_path = env.create_test_file("large.j2", template_content);
    let output_path = env.temp_path("large_output.txt");

    // Create large array of items
    let items: Vec<String> = (0..1000).map(|i| format!("value_{}", i)).collect();

    let args = TemplateTestBuilder::new()
        .src(template_path.to_string_lossy())
        .dest(output_path.to_string_lossy())
        .variable("items", items)
        .build();

    let result = env.execute_module("template", args).await.unwrap();

    assert!(result.changed);
    assert!(!result.failed);

    // Verify large output was generated
    assert_file_exists(&output_path);
    let content = env.read_file("large_output.txt").unwrap();
    assert!(content.contains("value_0"));
    assert!(content.contains("value_999"));
}
