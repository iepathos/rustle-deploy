//! Test fixture management

use std::collections::HashMap;

/// Test fixtures for file operations testing
pub struct TestFixtures {
    pub templates: HashMap<String, String>,
    pub sample_files: HashMap<String, Vec<u8>>,
    pub expected_outputs: HashMap<String, String>,
}

impl TestFixtures {
    /// Load test fixtures from embedded data
    pub fn load() -> Self {
        let mut fixtures = TestFixtures {
            templates: HashMap::new(),
            sample_files: HashMap::new(),
            expected_outputs: HashMap::new(),
        };

        // Simple template
        fixtures.templates.insert(
            "simple".to_string(),
            "Hello {{ name }}!\nVersion: {{ version }}\n".to_string(),
        );

        // Configuration template
        fixtures.templates.insert(
            "config".to_string(),
            r#"[app]
name = "{{ app_name }}"
port = {{ port }}
debug = {{ debug }}

[database]
host = "{{ db_host | default('localhost') }}"
port = {{ db_port | default(5432) }}
"#
            .to_string(),
        );

        // Complex template with conditionals and loops
        fixtures.templates.insert(
            "complex".to_string(),
            r#"# Configuration for {{ service_name }}
{% if enable_ssl %}
ssl_enabled = true
ssl_cert = "{{ ssl_cert_path }}"
ssl_key = "{{ ssl_key_path }}"
{% else %}
ssl_enabled = false
{% endif %}

{% if servers %}
[servers]
{% for server in servers %}
  [[servers.list]]
  name = "{{ server.name }}"
  host = "{{ server.host }}"
  port = {{ server.port }}
{% endfor %}
{% endif %}

{% if environment == "production" %}
log_level = "warn"
{% else %}
log_level = "debug"
{% endif %}
"#
            .to_string(),
        );

        // Sample text file
        fixtures.sample_files.insert(
            "small_text".to_string(),
            b"This is a small test file.\nIt has multiple lines.\nLast line.".to_vec(),
        );

        // Sample binary file (represents a small image or binary data)
        let mut binary_data = Vec::new();
        for i in 0..1024 {
            binary_data.push((i % 256) as u8);
        }
        fixtures
            .sample_files
            .insert("small_binary".to_string(), binary_data);

        // Large file content (for performance testing)
        let large_content = "A".repeat(1024 * 1024); // 1MB of 'A's
        fixtures
            .sample_files
            .insert("large_text".to_string(), large_content.into_bytes());

        // Expected template outputs
        fixtures.expected_outputs.insert(
            "simple_rendered".to_string(),
            "Hello World!\nVersion: 1.0.0\n".to_string(),
        );

        fixtures.expected_outputs.insert(
            "config_rendered".to_string(),
            r#"[app]
name = "test_app"
port = 8080
debug = true

[database]
host = "localhost"
port = 5432
"#
            .to_string(),
        );

        fixtures.expected_outputs.insert(
            "complex_rendered_ssl".to_string(),
            r#"# Configuration for web_service
ssl_enabled = true
ssl_cert = "/etc/ssl/cert.pem"
ssl_key = "/etc/ssl/key.pem"

[servers]
  [[servers.list]]
  name = "web1"
  host = "192.168.1.10"
  port = 80
  [[servers.list]]
  name = "web2"
  host = "192.168.1.11"
  port = 80

log_level = "warn"
"#
            .to_string(),
        );

        fixtures
    }

    /// Get template by name
    pub fn get_template(&self, name: &str) -> Option<&String> {
        self.templates.get(name)
    }

    /// Get sample file by name
    pub fn get_sample_file(&self, name: &str) -> Option<&Vec<u8>> {
        self.sample_files.get(name)
    }

    /// Get expected output by name
    pub fn get_expected_output(&self, name: &str) -> Option<&String> {
        self.expected_outputs.get(name)
    }

    /// Create variables for simple template
    pub fn simple_template_vars() -> HashMap<String, serde_json::Value> {
        let mut vars = HashMap::new();
        vars.insert(
            "name".to_string(),
            serde_json::Value::String("World".to_string()),
        );
        vars.insert(
            "version".to_string(),
            serde_json::Value::String("1.0.0".to_string()),
        );
        vars
    }

    /// Create variables for config template
    pub fn config_template_vars() -> HashMap<String, serde_json::Value> {
        let mut vars = HashMap::new();
        vars.insert(
            "app_name".to_string(),
            serde_json::Value::String("test_app".to_string()),
        );
        vars.insert("port".to_string(), serde_json::Value::Number(8080.into()));
        vars.insert("debug".to_string(), serde_json::Value::Bool(true));
        vars
    }

    /// Create variables for complex template with SSL enabled
    pub fn complex_template_vars_ssl() -> HashMap<String, serde_json::Value> {
        let mut vars = HashMap::new();
        vars.insert(
            "service_name".to_string(),
            serde_json::Value::String("web_service".to_string()),
        );
        vars.insert("enable_ssl".to_string(), serde_json::Value::Bool(true));
        vars.insert(
            "ssl_cert_path".to_string(),
            serde_json::Value::String("/etc/ssl/cert.pem".to_string()),
        );
        vars.insert(
            "ssl_key_path".to_string(),
            serde_json::Value::String("/etc/ssl/key.pem".to_string()),
        );
        vars.insert(
            "environment".to_string(),
            serde_json::Value::String("production".to_string()),
        );

        // Add servers array
        let servers = serde_json::json!([
            {
                "name": "web1",
                "host": "192.168.1.10",
                "port": 80
            },
            {
                "name": "web2",
                "host": "192.168.1.11",
                "port": 80
            }
        ]);
        vars.insert("servers".to_string(), servers);

        vars
    }

    /// Create variables for complex template without SSL
    pub fn complex_template_vars_no_ssl() -> HashMap<String, serde_json::Value> {
        let mut vars = HashMap::new();
        vars.insert(
            "service_name".to_string(),
            serde_json::Value::String("dev_service".to_string()),
        );
        vars.insert("enable_ssl".to_string(), serde_json::Value::Bool(false));
        vars.insert(
            "environment".to_string(),
            serde_json::Value::String("development".to_string()),
        );
        vars
    }
}

impl Default for TestFixtures {
    fn default() -> Self {
        Self::load()
    }
}
