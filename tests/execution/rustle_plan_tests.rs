use std::fs;
use rustle_deploy::execution::{validate_rustle_plan_json, RustlePlanConverter};

#[tokio::test]
async fn test_parse_example_rustle_plan_output() {
    let content = fs::read_to_string("example_rustle_plan_output.json")
        .expect("Failed to read example rustle plan output file");
    
    let result = validate_rustle_plan_json(&content);
    assert!(result.is_ok(), "Failed to parse example rustle plan output: {:?}", result);
    
    let rustle_plan = result.unwrap();
    assert_eq!(rustle_plan.total_tasks, 3);
    assert_eq!(rustle_plan.hosts.len(), 1);
    assert_eq!(rustle_plan.hosts[0], "localhost");
    assert_eq!(rustle_plan.plays.len(), 1);
    
    let play = &rustle_plan.plays[0];
    assert_eq!(play.play_id, "play-0");
    assert_eq!(play.name, "Simple test playbook");
    assert_eq!(play.batches.len(), 1);
    
    let batch = &play.batches[0];
    assert_eq!(batch.batch_id, "binary-batch");
    assert_eq!(batch.tasks.len(), 3);
    
    // Test individual tasks
    let task1 = &batch.tasks[0];
    assert_eq!(task1.task_id, "task_0");
    assert_eq!(task1.name, "Print a message");
    assert_eq!(task1.module, "debug");
    assert!(task1.args.contains_key("msg"));
    
    let task2 = &batch.tasks[1];
    assert_eq!(task2.task_id, "task_1");
    assert_eq!(task2.name, "Install package");
    assert_eq!(task2.module, "package");
    
    let task3 = &batch.tasks[2];
    assert_eq!(task3.task_id, "task_2");
    assert_eq!(task3.name, "Notify handler");
    assert_eq!(task3.module, "command");
    assert!(!task3.notify.is_empty());
    assert_eq!(task3.notify[0], "restart service");
}

#[tokio::test]
async fn test_convert_rustle_plan_to_execution_plan() {
    let content = fs::read_to_string("example_rustle_plan_output.json")
        .expect("Failed to read example rustle plan output file");
    
    let rustle_plan = validate_rustle_plan_json(&content)
        .expect("Failed to parse rustle plan");
    
    let converter = RustlePlanConverter::new();
    let result = converter.convert_to_execution_plan(&rustle_plan);
    
    assert!(result.is_ok(), "Failed to convert rustle plan: {:?}", result);
    
    let execution_plan = result.unwrap();
    assert_eq!(execution_plan.tasks.len(), 3);
    assert!(!execution_plan.inventory.hosts.is_empty());
    assert!(execution_plan.inventory.hosts.contains_key("localhost"));
    
    // Test that modules are extracted
    assert!(!execution_plan.modules.is_empty());
    let module_names: Vec<&String> = execution_plan.modules.iter().map(|m| &m.name).collect();
    assert!(module_names.contains(&&"debug".to_string()));
    assert!(module_names.contains(&&"package".to_string()));
    assert!(module_names.contains(&&"command".to_string()));
}

#[tokio::test]
async fn test_binary_deployment_extraction() {
    let content = fs::read_to_string("example_rustle_plan_output.json")
        .expect("Failed to read example rustle plan output file");
    
    let rustle_plan = validate_rustle_plan_json(&content)
        .expect("Failed to parse rustle plan");
    
    let converter = RustlePlanConverter::new();
    let result = converter.extract_binary_deployments(&rustle_plan);
    
    assert!(result.is_ok(), "Failed to extract binary deployments: {:?}", result);
    
    // The example file has an empty binary_deployments array, 
    // so this should analyze and potentially create new ones
    let binary_deployments = result.unwrap();
    
    // With binary_threshold of 5 and only 3 tasks, we shouldn't get any deployments
    // But if the analyzer determines some tasks are compatible, we might get results
    println!("Binary deployments found: {}", binary_deployments.len());
}

#[tokio::test]
async fn test_task_condition_parsing() {
    let content = fs::read_to_string("example_rustle_plan_output.json")
        .expect("Failed to read example rustle plan output file");
    
    let rustle_plan = validate_rustle_plan_json(&content)
        .expect("Failed to parse rustle plan");
    
    // Find task with conditions
    let task_with_conditions = rustle_plan.plays[0].batches[0].tasks.iter()
        .find(|task| !task.conditions.is_empty())
        .expect("Expected to find task with conditions");
    
    assert!(!task_with_conditions.conditions.is_empty());
    
    // Convert to execution plan and check condition conversion
    let converter = RustlePlanConverter::new();
    let execution_plan = converter.convert_to_execution_plan(&rustle_plan)
        .expect("Failed to convert execution plan");
    
    // Find the converted task
    let converted_task = execution_plan.tasks.iter()
        .find(|task| task.id == task_with_conditions.task_id)
        .expect("Expected to find converted task");
    
    // Should have conditions converted
    assert!(!converted_task.conditions.is_empty());
}

#[tokio::test]
async fn test_handler_references() {
    let content = fs::read_to_string("example_rustle_plan_output.json")
        .expect("Failed to read example rustle plan output file");
    
    let rustle_plan = validate_rustle_plan_json(&content)
        .expect("Failed to parse rustle plan");
    
    let play = &rustle_plan.plays[0];
    
    // Check that handlers exist
    assert!(!play.handlers.is_empty());
    let handler = &play.handlers[0];
    assert_eq!(handler.name, "restart service");
    assert_eq!(handler.module, "service");
    
    // Check that tasks reference the handler
    let task_with_notify = play.batches[0].tasks.iter()
        .find(|task| !task.notify.is_empty())
        .expect("Expected to find task with notify");
    
    assert!(task_with_notify.notify.contains(&"restart service".to_string()));
}

#[tokio::test]
async fn test_risk_level_parsing() {
    let content = fs::read_to_string("example_rustle_plan_output.json")
        .expect("Failed to read example rustle plan output file");
    
    let rustle_plan = validate_rustle_plan_json(&content)
        .expect("Failed to parse rustle plan");
    
    let tasks = &rustle_plan.plays[0].batches[0].tasks;
    
    // Check different risk levels are parsed correctly
    assert!(tasks.iter().any(|t| matches!(t.risk_level, rustle_deploy::execution::RiskLevel::Low)));
    assert!(tasks.iter().any(|t| matches!(t.risk_level, rustle_deploy::execution::RiskLevel::High)));
    assert!(tasks.iter().any(|t| matches!(t.risk_level, rustle_deploy::execution::RiskLevel::Critical)));
}

#[tokio::test]
async fn test_duration_parsing() {
    let content = fs::read_to_string("example_rustle_plan_output.json")
        .expect("Failed to read example rustle plan output file");
    
    let rustle_plan = validate_rustle_plan_json(&content)
        .expect("Failed to parse rustle plan");
    
    // Check that estimated_duration is parsed
    assert!(rustle_plan.estimated_duration.is_some());
    let duration = rustle_plan.estimated_duration.unwrap();
    assert!(duration.as_secs() > 0);
    
    // Check task durations
    for task in &rustle_plan.plays[0].batches[0].tasks {
        assert!(task.estimated_duration.as_nanos() > 0);
    }
}

#[tokio::test]
async fn test_execution_strategy_parsing() {
    let content = fs::read_to_string("example_rustle_plan_output.json")
        .expect("Failed to read example rustle plan output file");
    
    let rustle_plan = validate_rustle_plan_json(&content)
        .expect("Failed to parse rustle plan");
    
    // Check strategy parsing
    assert!(matches!(
        rustle_plan.metadata.planning_options.strategy,
        rustle_deploy::execution::ExecutionStrategy::BinaryHybrid
    ));
    
    assert!(matches!(
        rustle_plan.plays[0].strategy,
        rustle_deploy::execution::ExecutionStrategy::BinaryHybrid
    ));
}

#[tokio::test]
async fn test_metadata_parsing() {
    let content = fs::read_to_string("example_rustle_plan_output.json")
        .expect("Failed to read example rustle plan output file");
    
    let rustle_plan = validate_rustle_plan_json(&content)
        .expect("Failed to parse rustle plan");
    
    let metadata = &rustle_plan.metadata;
    assert_eq!(metadata.rustle_plan_version, "0.1.0");
    assert!(!metadata.playbook_hash.is_empty());
    assert!(!metadata.inventory_hash.is_empty());
    
    let options = &metadata.planning_options;
    assert_eq!(options.forks, 50);
    assert_eq!(options.binary_threshold, 5);
    assert!(!options.force_binary);
    assert!(!options.force_ssh);
}

#[cfg(test)]
mod error_cases {
    use super::*;

    #[tokio::test]
    async fn test_invalid_json() {
        let invalid_json = "{ invalid json }";
        let result = validate_rustle_plan_json(invalid_json);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_missing_required_fields() {
        let incomplete_json = r#"{
            "metadata": {
                "created_at": "2025-07-11T05:18:16.945474Z",
                "rustle_plan_version": "0.1.0"
            }
        }"#;
        
        let result = validate_rustle_plan_json(incomplete_json);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_invalid_execution_strategy() {
        let invalid_strategy_json = r#"{
            "metadata": {
                "created_at": "2025-07-11T05:18:16.945474Z",
                "rustle_plan_version": "0.1.0",
                "playbook_hash": "test",
                "inventory_hash": "test",
                "planning_options": {
                    "forks": 5,
                    "strategy": "InvalidStrategy",
                    "binary_threshold": 3
                }
            },
            "plays": [],
            "binary_deployments": [],
            "total_tasks": 0,
            "hosts": []
        }"#;
        
        let result = validate_rustle_plan_json(invalid_strategy_json);
        assert!(result.is_err());
    }
}