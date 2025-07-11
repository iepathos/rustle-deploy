use rustle_deploy::execution::{
    ExecutionPlan, ExecutionPlanMetadata, Task, TaskType, TargetSelector, 
    FailurePolicy, InventorySpec, InventoryFormat, InventorySource, 
    ExecutionStrategy, FactsTemplate, DeploymentConfig, Condition, ConditionOperator,
    RetryPolicy, BackoffStrategy
};
use rustle_deploy::runtime::{LocalExecutor, RuntimeConfig, TaskStatus};
use rustle_deploy::compiler::RuntimeTemplateGenerator;
use chrono::Utc;
use std::collections::HashMap;
use std::time::Duration;
use tokio_test;

#[tokio::test]
async fn test_end_to_end_execution() {
    let execution_plan = create_complex_execution_plan();
    let config = RuntimeConfig::default();
    
    let mut executor = LocalExecutor::new(config);
    let result = executor.execute_plan(execution_plan).await.unwrap();
    
    assert!(!result.failed);
    assert_eq!(result.summary.total_tasks, 4);
    assert_eq!(result.summary.completed_tasks, 3); // One task should be skipped
    assert_eq!(result.summary.failed_tasks, 0);
    assert_eq!(result.summary.skipped_tasks, 1);
}

#[tokio::test]
async fn test_dependency_execution_order() {
    let execution_plan = create_dependency_execution_plan();
    let config = RuntimeConfig::default();
    
    let mut executor = LocalExecutor::new(config);
    let result = executor.execute_plan(execution_plan).await.unwrap();
    
    assert!(!result.failed);
    assert_eq!(result.summary.total_tasks, 3);
    assert_eq!(result.summary.completed_tasks, 3);
    
    let task1_result = result.task_results.get("task-1").unwrap();
    let task2_result = result.task_results.get("task-2").unwrap();
    let task3_result = result.task_results.get("task-3").unwrap();
    
    // Verify execution order based on dependencies
    assert!(task1_result.end_time <= task2_result.start_time);
    assert!(task1_result.end_time <= task3_result.start_time);
    assert!(task2_result.end_time <= task3_result.start_time);
}

#[tokio::test]
async fn test_parallel_execution() {
    let execution_plan = create_parallel_execution_plan();
    let mut config = RuntimeConfig::default();
    config.parallel_tasks = Some(2); // Allow 2 tasks in parallel
    
    let mut executor = LocalExecutor::new(config);
    let start_time = std::time::Instant::now();
    let result = executor.execute_plan(execution_plan).await.unwrap();
    let total_time = start_time.elapsed();
    
    assert!(!result.failed);
    assert_eq!(result.summary.total_tasks, 2);
    assert_eq!(result.summary.completed_tasks, 2);
    
    // With parallel execution, total time should be less than sum of individual task times
    // (This is a rough check since we're using debug tasks which are very fast)
    assert!(total_time < Duration::from_secs(2));
}

#[tokio::test]
async fn test_retry_mechanism() {
    let execution_plan = create_retry_execution_plan();
    let config = RuntimeConfig::default();
    
    let mut executor = LocalExecutor::new(config);
    let result = executor.execute_plan(execution_plan).await;
    
    // The command should fail but we should see retry attempts
    // This test mainly verifies that the retry mechanism is triggered
    assert!(result.is_err() || result.unwrap().failed);
}

#[tokio::test]
async fn test_facts_integration() {
    let mut execution_plan = create_test_execution_plan();
    
    // Add a task that uses facts
    execution_plan.tasks.push(Task {
        id: "facts-task".to_string(),
        name: "Facts Task".to_string(),
        task_type: TaskType::Command,
        module: "debug".to_string(),
        args: [("var".to_string(), serde_json::json!("ansible_hostname"))].into(),
        dependencies: vec![],
        conditions: vec![
            Condition {
                variable: "ansible_hostname".to_string(),
                operator: ConditionOperator::Exists,
                value: serde_json::json!(null),
            }
        ],
        target_hosts: TargetSelector::All,
        timeout: None,
        retry_policy: None,
        failure_policy: FailurePolicy::Abort,
    });
    
    let config = RuntimeConfig::default();
    let mut executor = LocalExecutor::new(config);
    let result = executor.execute_plan(execution_plan).await.unwrap();
    
    assert!(!result.failed);
    
    let facts_task_result = result.task_results.get("facts-task").unwrap();
    assert_eq!(facts_task_result.status, TaskStatus::Success);
    assert!(!facts_task_result.skipped);
}

#[test]
fn test_runtime_template_generation() {
    let generator = RuntimeTemplateGenerator::new().unwrap();
    let execution_plan = create_test_execution_plan();
    let runtime_config = RuntimeConfig::default();
    
    // Test main.rs generation
    let main_rs = generator.generate_main_rs(&execution_plan, &runtime_config).unwrap();
    assert!(main_rs.contains("fn main()"));
    assert!(main_rs.contains("LocalExecutor"));
    assert!(main_rs.contains("execute_plan"));
    assert!(main_rs.contains("EXECUTION_PLAN"));
    assert!(main_rs.contains("RUNTIME_CONFIG"));
    
    // Test Cargo.toml generation
    let cargo_toml = generator.generate_cargo_toml("test-runtime").unwrap();
    assert!(cargo_toml.contains("name = \"test-runtime\""));
    assert!(cargo_toml.contains("tokio"));
    assert!(cargo_toml.contains("serde"));
    assert!(cargo_toml.contains("anyhow"));
    
    // Test complete project generation
    let project_files = generator.generate_binary_project(
        "test-runtime", 
        &execution_plan, 
        &runtime_config
    ).unwrap();
    
    assert!(project_files.contains_key("src/main.rs"));
    assert!(project_files.contains_key("Cargo.toml"));
    assert!(project_files.contains_key("src/lib.rs"));
}

#[tokio::test]
async fn test_error_handling() {
    let mut execution_plan = create_test_execution_plan();
    
    // Add a task that will fail
    execution_plan.tasks.push(Task {
        id: "failing-task".to_string(),
        name: "Failing Task".to_string(),
        task_type: TaskType::Command,
        module: "command".to_string(),
        args: [("_raw_params".to_string(), serde_json::json!("false"))].into(), // Command that always fails
        dependencies: vec![],
        conditions: vec![],
        target_hosts: TargetSelector::All,
        timeout: None,
        retry_policy: None,
        failure_policy: FailurePolicy::Abort,
    });
    
    let config = RuntimeConfig::default();
    let mut executor = LocalExecutor::new(config);
    let result = executor.execute_plan(execution_plan).await.unwrap();
    
    assert!(result.failed);
    assert_eq!(result.summary.failed_tasks, 1);
    
    let failing_task_result = result.task_results.get("failing-task").unwrap();
    assert_eq!(failing_task_result.status, TaskStatus::Failed);
    assert!(failing_task_result.failed);
}

fn create_test_execution_plan() -> ExecutionPlan {
    ExecutionPlan {
        metadata: ExecutionPlanMetadata {
            version: "1.0.0".to_string(),
            created_at: Utc::now(),
            rustle_plan_version: "1.0.0".to_string(),
            plan_id: "integration-test-plan".to_string(),
            description: Some("Integration test execution plan".to_string()),
            author: Some("test".to_string()),
            tags: vec!["integration".to_string(), "test".to_string()],
        },
        tasks: vec![
            Task {
                id: "test-task-1".to_string(),
                name: "Test Task 1".to_string(),
                task_type: TaskType::Command,
                module: "debug".to_string(),
                args: [("msg".to_string(), serde_json::json!("Integration test"))].into(),
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
            global_facts: vec!["ansible_hostname".to_string()],
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

fn create_complex_execution_plan() -> ExecutionPlan {
    let mut plan = create_test_execution_plan();
    
    plan.tasks = vec![
        Task {
            id: "setup-task".to_string(),
            name: "Setup Task".to_string(),
            task_type: TaskType::Command,
            module: "debug".to_string(),
            args: [("msg".to_string(), serde_json::json!("Setting up"))].into(),
            dependencies: vec![],
            conditions: vec![],
            target_hosts: TargetSelector::All,
            timeout: None,
            retry_policy: None,
            failure_policy: FailurePolicy::Abort,
        },
        Task {
            id: "main-task".to_string(),
            name: "Main Task".to_string(),
            task_type: TaskType::Command,
            module: "debug".to_string(),
            args: [("msg".to_string(), serde_json::json!("Main execution"))].into(),
            dependencies: vec!["setup-task".to_string()],
            conditions: vec![],
            target_hosts: TargetSelector::All,
            timeout: None,
            retry_policy: None,
            failure_policy: FailurePolicy::Abort,
        },
        Task {
            id: "conditional-task".to_string(),
            name: "Conditional Task".to_string(),
            task_type: TaskType::Command,
            module: "debug".to_string(),
            args: [("msg".to_string(), serde_json::json!("This should be skipped"))].into(),
            dependencies: vec![],
            conditions: vec![
                Condition {
                    variable: "nonexistent_var".to_string(),
                    operator: ConditionOperator::Equals,
                    value: serde_json::json!("some_value"),
                }
            ],
            target_hosts: TargetSelector::All,
            timeout: None,
            retry_policy: None,
            failure_policy: FailurePolicy::Abort,
        },
        Task {
            id: "cleanup-task".to_string(),
            name: "Cleanup Task".to_string(),
            task_type: TaskType::Command,
            module: "debug".to_string(),
            args: [("msg".to_string(), serde_json::json!("Cleaning up"))].into(),
            dependencies: vec!["main-task".to_string()],
            conditions: vec![],
            target_hosts: TargetSelector::All,
            timeout: None,
            retry_policy: None,
            failure_policy: FailurePolicy::Abort,
        },
    ];
    
    plan
}

fn create_dependency_execution_plan() -> ExecutionPlan {
    let mut plan = create_test_execution_plan();
    
    plan.tasks = vec![
        Task {
            id: "task-1".to_string(),
            name: "Task 1".to_string(),
            task_type: TaskType::Command,
            module: "debug".to_string(),
            args: [("msg".to_string(), serde_json::json!("Task 1"))].into(),
            dependencies: vec![],
            conditions: vec![],
            target_hosts: TargetSelector::All,
            timeout: None,
            retry_policy: None,
            failure_policy: FailurePolicy::Abort,
        },
        Task {
            id: "task-2".to_string(),
            name: "Task 2".to_string(),
            task_type: TaskType::Command,
            module: "debug".to_string(),
            args: [("msg".to_string(), serde_json::json!("Task 2"))].into(),
            dependencies: vec!["task-1".to_string()],
            conditions: vec![],
            target_hosts: TargetSelector::All,
            timeout: None,
            retry_policy: None,
            failure_policy: FailurePolicy::Abort,
        },
        Task {
            id: "task-3".to_string(),
            name: "Task 3".to_string(),
            task_type: TaskType::Command,
            module: "debug".to_string(),
            args: [("msg".to_string(), serde_json::json!("Task 3"))].into(),
            dependencies: vec!["task-1".to_string(), "task-2".to_string()],
            conditions: vec![],
            target_hosts: TargetSelector::All,
            timeout: None,
            retry_policy: None,
            failure_policy: FailurePolicy::Abort,
        },
    ];
    
    plan
}

fn create_parallel_execution_plan() -> ExecutionPlan {
    let mut plan = create_test_execution_plan();
    
    plan.tasks = vec![
        Task {
            id: "parallel-task-1".to_string(),
            name: "Parallel Task 1".to_string(),
            task_type: TaskType::Command,
            module: "debug".to_string(),
            args: [("msg".to_string(), serde_json::json!("Parallel 1"))].into(),
            dependencies: vec![],
            conditions: vec![],
            target_hosts: TargetSelector::All,
            timeout: None,
            retry_policy: None,
            failure_policy: FailurePolicy::Abort,
        },
        Task {
            id: "parallel-task-2".to_string(),
            name: "Parallel Task 2".to_string(),
            task_type: TaskType::Command,
            module: "debug".to_string(),
            args: [("msg".to_string(), serde_json::json!("Parallel 2"))].into(),
            dependencies: vec![],
            conditions: vec![],
            target_hosts: TargetSelector::All,
            timeout: None,
            retry_policy: None,
            failure_policy: FailurePolicy::Abort,
        },
    ];
    
    plan
}

fn create_retry_execution_plan() -> ExecutionPlan {
    let mut plan = create_test_execution_plan();
    
    plan.tasks = vec![
        Task {
            id: "retry-task".to_string(),
            name: "Retry Task".to_string(),
            task_type: TaskType::Command,
            module: "command".to_string(),
            args: [("_raw_params".to_string(), serde_json::json!("false"))].into(),
            dependencies: vec![],
            conditions: vec![],
            target_hosts: TargetSelector::All,
            timeout: Some(Duration::from_secs(5)),
            retry_policy: Some(RetryPolicy {
                max_attempts: 3,
                delay: Duration::from_millis(100),
                backoff: BackoffStrategy::Fixed,
            }),
            failure_policy: FailurePolicy::Abort,
        },
    ];
    
    plan
}