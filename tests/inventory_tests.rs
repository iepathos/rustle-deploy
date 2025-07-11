use chrono::Utc;
use rustle_deploy::inventory::{InventoryProcessor, JsonInventoryProcessor};
use rustle_deploy::types::inventory::{
    ConnectionConfig, ConnectionMethod, InventoryFormat, InventoryGroup, InventoryHost,
    InventoryMetadata, ParsedInventory,
};
use serde_json::json;
use std::collections::HashMap;

#[tokio::test]
async fn test_inventory_processor_basic() {
    let processor = InventoryProcessor::new();

    // Create a simple inventory
    let mut inventory = create_test_inventory();

    // Process the inventory
    let result = processor.process_inventory_data(&mut inventory);
    if let Err(e) = &result {
        println!("Error processing inventory: {:?}", e);
    }
    assert!(result.is_ok());

    // Verify that architectures were detected
    let host = inventory.hosts.get("test-host").unwrap();
    assert!(host.target_triple.is_some());
}

#[tokio::test]
async fn test_deployment_target_generation() {
    let processor = InventoryProcessor::new();
    let mut inventory = create_test_inventory();

    // Process inventory first
    processor.process_inventory_data(&mut inventory).unwrap();

    // Generate deployment targets
    let targets = processor.to_deployment_targets(&inventory).unwrap();

    assert_eq!(targets.len(), 1);
    let target = &targets[0];
    assert_eq!(target.host, "192.168.1.10");
    assert!(target.binary_compilation_id.starts_with("rustle-"));
}

#[tokio::test]
async fn test_architecture_detection() {
    let processor = InventoryProcessor::new();
    let mut inventory = create_test_inventory();

    // Add architecture info to host
    let host = inventory.hosts.get_mut("test-host").unwrap();
    host.variables.insert(
        "ansible_architecture".to_string(),
        serde_json::Value::String("x86_64".to_string()),
    );
    host.variables.insert(
        "ansible_os_family".to_string(),
        serde_json::Value::String("debian".to_string()),
    );

    let result = processor.detect_architectures(&mut inventory);
    assert!(result.is_ok());

    let host = inventory.hosts.get("test-host").unwrap();
    assert_eq!(
        host.target_triple,
        Some("x86_64-unknown-linux-gnu".to_string())
    );
}

#[tokio::test]
async fn test_variable_resolution() {
    let processor = InventoryProcessor::new();
    let mut inventory = create_test_inventory_with_groups();

    let result = processor.resolve_variables(&mut inventory);
    assert!(result.is_ok());

    // Check that group variables were inherited
    let host = inventory.hosts.get("web-server").unwrap();
    assert!(host.variables.contains_key("group_var"));
    assert!(host.variables.contains_key("host_var"));
}

#[tokio::test]
async fn test_process_ansible_dynamic_inventory() {
    let processor = JsonInventoryProcessor::new();

    let inventory_json = json!({
        "_meta": {
            "hostvars": {
                "web-01": {
                    "ansible_host": "192.168.1.10",
                    "ansible_user": "ubuntu",
                    "ansible_architecture": "x86_64",
                    "ansible_os_family": "Debian"
                },
                "web-02": {
                    "ansible_host": "192.168.1.11",
                    "ansible_user": "ubuntu",
                    "ansible_architecture": "x86_64",
                    "ansible_os_family": "Debian"
                }
            }
        },
        "webservers": {
            "hosts": ["web-01", "web-02"],
            "vars": {
                "http_port": 80,
                "max_clients": 100
            }
        },
        "all": {
            "vars": {
                "deployment_user": "deploy"
            }
        }
    });

    let result = processor.process_inventory_json(&inventory_json);
    assert!(result.is_ok());

    let inventory = result.unwrap();
    assert_eq!(inventory.hosts.len(), 2);
    assert_eq!(inventory.groups.len(), 1);

    // Check host details
    let host = inventory.hosts.get("web-01").unwrap();
    assert_eq!(host.address, Some("192.168.1.10".to_string()));
    assert_eq!(host.connection.username, Some("ubuntu".to_string()));

    // Check group variables
    let group = inventory.groups.get("webservers").unwrap();
    assert!(group.variables.contains_key("http_port"));

    // Check global variables
    assert!(inventory.global_vars.contains_key("deployment_user"));
}

#[tokio::test]
async fn test_process_simple_json_inventory() {
    let processor = JsonInventoryProcessor::new();

    let inventory_json = json!({
        "database": {
            "hosts": ["db-01"],
            "vars": {
                "db_port": 5432
            }
        },
        "web": {
            "hosts": ["web-01", "web-02"]
        }
    });

    let result = processor.process_inventory_json(&inventory_json);
    assert!(result.is_ok());

    let inventory = result.unwrap();
    assert_eq!(inventory.hosts.len(), 3);
    assert_eq!(inventory.groups.len(), 2);

    // Check that default hosts were created
    let host = inventory.hosts.get("db-01").unwrap();
    assert_eq!(host.name, "db-01");
    assert_eq!(host.address, Some("db-01".to_string()));
}

fn create_test_inventory() -> ParsedInventory {
    let mut hosts = HashMap::new();
    let connection = ConnectionConfig {
        method: ConnectionMethod::Ssh,
        host: Some("192.168.1.10".to_string()),
        port: Some(22),
        username: Some("ubuntu".to_string()),
        password: None,
        private_key: None,
        private_key_file: None,
        timeout: None,
        ssh_args: None,
        winrm_transport: None,
    };

    let mut variables = HashMap::new();
    variables.insert(
        "ansible_architecture".to_string(),
        serde_json::Value::String("x86_64".to_string()),
    );
    variables.insert(
        "ansible_os_family".to_string(),
        serde_json::Value::String("Debian".to_string()),
    );

    let host = InventoryHost {
        name: "test-host".to_string(),
        address: Some("192.168.1.10".to_string()),
        connection,
        variables,
        groups: Vec::new(),
        target_triple: None,
        architecture: None,
        operating_system: None,
        platform: None,
    };

    hosts.insert("test-host".to_string(), host);

    let metadata = InventoryMetadata {
        format: InventoryFormat::Json,
        source: "test".to_string(),
        parsed_at: Utc::now(),
        host_count: 1,
        group_count: 0,
    };

    ParsedInventory {
        hosts,
        groups: HashMap::new(),
        global_vars: HashMap::new(),
        metadata,
    }
}

fn create_test_inventory_with_groups() -> ParsedInventory {
    let mut hosts = HashMap::new();
    let mut groups = HashMap::new();

    // Create a host
    let mut host_vars = HashMap::new();
    host_vars.insert(
        "host_var".to_string(),
        serde_json::Value::String("host_value".to_string()),
    );

    let connection = ConnectionConfig {
        method: ConnectionMethod::Ssh,
        host: Some("192.168.1.10".to_string()),
        port: Some(22),
        username: Some("ubuntu".to_string()),
        password: None,
        private_key: None,
        private_key_file: None,
        timeout: None,
        ssh_args: None,
        winrm_transport: None,
    };

    let host = InventoryHost {
        name: "web-server".to_string(),
        address: Some("192.168.1.10".to_string()),
        connection,
        variables: host_vars,
        groups: vec!["web".to_string()],
        target_triple: None,
        architecture: None,
        operating_system: None,
        platform: None,
    };

    hosts.insert("web-server".to_string(), host);

    // Create a group
    let mut group_vars = HashMap::new();
    group_vars.insert(
        "group_var".to_string(),
        serde_json::Value::String("group_value".to_string()),
    );

    let group = InventoryGroup {
        name: "web".to_string(),
        hosts: vec!["web-server".to_string()],
        children: Vec::new(),
        variables: group_vars,
        parent_groups: Vec::new(),
    };

    groups.insert("web".to_string(), group);

    let metadata = InventoryMetadata {
        format: InventoryFormat::Json,
        source: "test".to_string(),
        parsed_at: Utc::now(),
        host_count: 1,
        group_count: 1,
    };

    ParsedInventory {
        hosts,
        groups,
        global_vars: HashMap::new(),
        metadata,
    }
}
