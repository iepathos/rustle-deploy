use rustle_deploy::execution::{
    ExecutionPlan, ExecutionPlanMetadata, Task, TaskType, TargetSelector, 
    FailurePolicy, InventorySpec, InventoryFormat, InventorySource, 
    ExecutionStrategy, FactsTemplate, DeploymentConfig
};
use rustle_deploy::runtime::{LocalExecutor, RuntimeConfig, TaskStatus};
use chrono::Utc;
use std::collections::HashMap;
use std::time::Duration;
use tokio_test;

#[tokio::test]
async fn test_basic_execution() {
    let execution_plan = create_test_execution_plan();
    let config = RuntimeConfig::default();
    
    let mut executor = LocalExecutor::new(config);
    let result = executor.execute_plan(execution_plan).await.unwrap();
    
    assert!(!result.failed);
    assert_eq!(result.summary.total_tasks, 1);
    assert_eq!(result.summary.completed_tasks, 1);
    assert_eq!(result.summary.failed_tasks, 0);
}

#[tokio::test]
async fn test_debug_task_execution() {
    let mut execution_plan = create_test_execution_plan();
    execution_plan.tasks[0].module = "debug".to_string();
    execution_plan.tasks[0].args = [("msg".to_string(), serde_json::json!("Test message"))].into();
    
    let config = RuntimeConfig::default();
    let mut executor = LocalExecutor::new(config);
    let result = executor.execute_plan(execution_plan).await.unwrap();
    
    assert!(!result.failed);
    assert_eq!(result.summary.completed_tasks, 1);
    
    let task_result = result.task_results.get("test-task-1").unwrap();
    assert_eq!(task_result.status, TaskStatus::Success);
    assert!(!task_result.changed);
    assert!(!task_result.failed);
}

#[tokio::test]
async fn test_command_task_execution() {
    let mut execution_plan = create_test_execution_plan();
    execution_plan.tasks[0].module = "command".to_string();
    execution_plan.tasks[0].args = [("_raw_params".to_string(), serde_json::json!("echo 'Hello World'"))].into();
    
    let config = RuntimeConfig::default();
    let mut executor = LocalExecutor::new(config);
    let result = executor.execute_plan(execution_plan).await.unwrap();
    
    assert!(!result.failed);
    
    let task_result = result.task_results.get("test-task-1").unwrap();
    assert_eq!(task_result.status, TaskStatus::Success);
    assert!(task_result.changed);
    assert!(!task_result.failed);
    assert!(task_result.stdout.as_ref().unwrap().contains("Hello World"));
}

#[tokio::test]
async fn test_multiple_tasks_execution() {
    let mut execution_plan = create_test_execution_plan();
    
    // Add a second task
    execution_plan.tasks.push(Task {
        id: "test-task-2".to_string(),
        name: "Test Task 2".to_string(),
        task_type: TaskType::Command,
        module: "debug".to_string(),
        args: [("msg".to_string(), serde_json::json!("Second task"))].into(),
        dependencies: vec!["test-task-1".to_string()], // Depends on first task
        conditions: vec![],
        target_hosts: TargetSelector::All,
        timeout: None,
        retry_policy: None,
        failure_policy: FailurePolicy::Abort,
    });
    
    let config = RuntimeConfig::default();
    let mut executor = LocalExecutor::new(config);
    let result = executor.execute_plan(execution_plan).await.unwrap();
    
    assert!(!result.failed);
    assert_eq!(result.summary.total_tasks, 2);
    assert_eq!(result.summary.completed_tasks, 2);
    
    // Verify both tasks completed successfully
    let task1_result = result.task_results.get("test-task-1").unwrap();
    let task2_result = result.task_results.get("test-task-2").unwrap();
    
    assert_eq!(task1_result.status, TaskStatus::Success);
    assert_eq!(task2_result.status, TaskStatus::Success);
    
    // Task 2 should have started after task 1 completed
    assert!(task2_result.start_time >= task1_result.end_time);
}

#[tokio::test]
async fn test_task_with_conditions() {
    let mut execution_plan = create_test_execution_plan();
    
    // Add a condition that should evaluate to false
    execution_plan.tasks[0].conditions = vec![
        rustle_deploy::execution::Condition {
            variable: "nonexistent_variable".to_string(),
            operator: rustle_deploy::execution::ConditionOperator::Equals,
            value: serde_json::json!("some_value"),
        }
    ];
    
    let config = RuntimeConfig::default();
    let mut executor = LocalExecutor::new(config);
    let result = executor.execute_plan(execution_plan).await.unwrap();
    
    assert!(!result.failed);
    
    let task_result = result.task_results.get("test-task-1").unwrap();
    assert_eq!(task_result.status, TaskStatus::Skipped);
    assert!(task_result.skipped);
}

#[tokio::test]
async fn test_task_timeout() {
    let mut execution_plan = create_test_execution_plan();
    execution_plan.tasks[0].module = "command".to_string();
    execution_plan.tasks[0].args = [("_raw_params".to_string(), serde_json::json!("sleep 10"))].into();
    execution_plan.tasks[0].timeout = Some(Duration::from_millis(100)); // Very short timeout
    
    let config = RuntimeConfig::default();
    let mut executor = LocalExecutor::new(config);
    let result = executor.execute_plan(execution_plan).await;
    
    // Should fail due to timeout
    assert!(result.is_err() || result.unwrap().failed);
}

#[tokio::test]
async fn test_check_mode() {
    let execution_plan = create_test_execution_plan();
    let mut config = RuntimeConfig::default();
    config.check_mode = Some(true);
    
    let mut executor = LocalExecutor::new(config);
    let result = executor.execute_plan(execution_plan).await.unwrap();
    
    assert!(!result.failed);
    
    let task_result = result.task_results.get("test-task-1").unwrap();
    assert_eq!(task_result.status, TaskStatus::Success);
    // In check mode, tasks should not make actual changes
}

#[tokio::test]
async fn test_facts_collection() {
    let execution_plan = create_test_execution_plan();
    let config = RuntimeConfig::default();
    
    let mut executor = LocalExecutor::new(config);
    
    // Collect facts
    executor.collect_facts().unwrap();
    
    // Execute plan
    let result = executor.execute_plan(execution_plan).await.unwrap();
    
    assert!(!result.failed);
    // Facts should be available during execution (tested implicitly)
}

fn create_test_execution_plan() -> ExecutionPlan {
    ExecutionPlan {
        metadata: ExecutionPlanMetadata {
            version: "1.0.0".to_string(),
            created_at: Utc::now(),
            rustle_plan_version: "1.0.0".to_string(),
            plan_id: "test-plan".to_string(),
            description: Some("Test execution plan".to_string()),
            author: Some("test".to_string()),
            tags: vec!["test".to_string()],
        },
        tasks: vec![
            Task {
                id: "test-task-1".to_string(),
                name: "Test Task 1".to_string(),
                task_type: TaskType::Command,
                module: "debug".to_string(),
                args: [("msg".to_string(), serde_json::json!("Test message"))].into(),
                dependencies: vec![],
                conditions: vec![],
                target_hosts: TargetSelector::All,
                timeout: None,
                retry_policy: None,
                failure_policy: FailurePolicy::Abort,
            }
        ],
        inventory: InventorySpec {
            format: InventoryFormat::Json,
            source: InventorySource::Inline { content: "{}".to_string() },
            groups: HashMap::new(),
            hosts: HashMap::new(),
            variables: HashMap::new(),
        },
        strategy: ExecutionStrategy::Linear,
        facts_template: FactsTemplate {
            global_facts: vec!["ansible_hostname".to_string(), "ansible_os_family".to_string()],
            host_facts: vec!["ansible_architecture".to_string()],
            custom_facts: HashMap::new(),
        },
        deployment_config: DeploymentConfig {
            target_path: "/tmp/test".to_string(),
            backup_previous: false,
            verify_deployment: false,
            cleanup_on_success: true,
            deployment_timeout: Some(Duration::from_secs(300)),
        },
        modules: vec![],
    }
}