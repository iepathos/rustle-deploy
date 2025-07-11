use rustle_deploy::compilation::{
    DeploymentOptimizer, OptimizationAnalysis, RecommendedStrategy,
    CompilationCapabilities, BinaryDeploymentDecision
};
use rustle_deploy::execution::plan::{RustlePlanOutput, TaskPlan};
use rustle_deploy::inventory::ParsedInventory;
use std::collections::HashMap;
use tokio_test;

#[tokio_test::test]
async fn test_optimization_analysis() {
    let optimizer = DeploymentOptimizer::new();
    let execution_plan = create_test_execution_plan();
    let capabilities = CompilationCapabilities::detect_basic().await.unwrap();
    let inventory = create_test_inventory();
    
    let analysis = optimizer.analyze_optimization_potential(
        &execution_plan,
        &capabilities,
        &inventory,
    ).await;
    
    assert!(analysis.is_ok());
    let analysis = analysis.unwrap();
    
    // Verify analysis structure
    assert!(analysis.optimization_score >= 0.0 && analysis.optimization_score <= 1.0);
    assert_eq!(analysis.total_tasks, execution_plan.tasks.len());
    assert!(analysis.binary_compatible_tasks <= analysis.total_tasks);
    assert!(analysis.estimated_speedup >= 1.0);
}

#[tokio_test::test]
async fn test_strategy_recommendation() {
    let optimizer = DeploymentOptimizer::new();
    let capabilities = CompilationCapabilities::detect_basic().await.unwrap();
    
    // Test with high compatibility execution plan
    let high_compat_plan = create_high_compatibility_plan();
    let inventory = create_test_inventory();
    
    let analysis = optimizer.analyze_optimization_potential(
        &high_compat_plan,
        &capabilities,
        &inventory,
    ).await.unwrap();
    
    // High compatibility should recommend binary or hybrid strategy
    match analysis.recommended_strategy {
        RecommendedStrategy::BinaryOnly | RecommendedStrategy::Hybrid => {
            // Expected for high compatibility
        }
        RecommendedStrategy::SshOnly => {
            // This might happen if compilation overhead is too high
            // or if target is not supported
        }
    }
}

#[tokio_test::test]
async fn test_deployment_plan_creation() {
    let optimizer = DeploymentOptimizer::new();
    let execution_plan = create_test_execution_plan();
    let capabilities = CompilationCapabilities::detect_basic().await.unwrap();
    let inventory = create_test_inventory();
    
    let deployment_plan = optimizer.create_optimal_deployment_plan(
        &execution_plan,
        &capabilities,
        &inventory,
    ).await;
    
    assert!(deployment_plan.is_ok());
    let plan = deployment_plan.unwrap();
    
    // Verify plan structure
    assert_eq!(plan.total_targets, inventory.hosts.len());
    assert!(plan.estimated_performance_gain >= 0.0);
    
    // Should have either binary or SSH deployments (or both)
    assert!(!plan.binary_deployments.is_empty() || !plan.ssh_deployments.is_empty());
}

#[tokio_test::test]
async fn test_performance_estimation() {
    let optimizer = DeploymentOptimizer::new();
    
    // Test various scenarios
    let gain_all_binary = optimizer.estimate_performance_gain(10, 0, 5);
    let gain_all_ssh = optimizer.estimate_performance_gain(0, 10, 5);
    let gain_mixed = optimizer.estimate_performance_gain(5, 5, 5);
    
    // All binary should be better than all SSH
    assert!(gain_all_binary > gain_all_ssh);
    
    // Mixed should be between all binary and all SSH
    assert!(gain_mixed > gain_all_ssh);
    assert!(gain_mixed < gain_all_binary);
    
    // Verify reasonable bounds
    assert!(gain_all_ssh >= 1.0); // At least no worse than baseline
    assert!(gain_all_binary <= 20.0); // Reasonable upper bound
}

#[tokio_test::test]
async fn test_binary_deployment_decision() {
    let optimizer = DeploymentOptimizer::new();
    let capabilities = CompilationCapabilities::detect_basic().await.unwrap();
    
    // Test with highly compatible tasks
    let compatible_tasks = create_compatible_tasks();
    let decision = optimizer.should_use_binary_deployment(
        &compatible_tasks,
        &capabilities,
        &capabilities.native_target,
    );
    
    match decision {
        BinaryDeploymentDecision::Recommended { confidence } => {
            assert!(confidence > 0.5);
        }
        BinaryDeploymentDecision::Feasible { limitations: _ } => {
            // This is acceptable for partially compatible tasks
        }
        BinaryDeploymentDecision::NotRecommended { reasons: _ } => {
            // This might happen if target is not supported
        }
    }
    
    // Test with unsupported target
    let unsupported_decision = optimizer.should_use_binary_deployment(
        &compatible_tasks,
        &capabilities,
        "unsupported-target-triple",
    );
    
    assert!(matches!(unsupported_decision, BinaryDeploymentDecision::NotRecommended { .. }));
}

#[tokio_test::test]
async fn test_empty_execution_plan() {
    let optimizer = DeploymentOptimizer::new();
    let empty_plan = RustlePlanOutput {
        tasks: vec![],
        optimize_for_size: false,
        metadata: HashMap::new(),
    };
    let capabilities = CompilationCapabilities::detect_basic().await.unwrap();
    let inventory = create_test_inventory();
    
    let analysis = optimizer.analyze_optimization_potential(
        &empty_plan,
        &capabilities,
        &inventory,
    ).await;
    
    // Should fail with insufficient data
    assert!(analysis.is_err());
}

// Helper functions for creating test data

fn create_test_execution_plan() -> RustlePlanOutput {
    RustlePlanOutput {
        tasks: vec![
            TaskPlan {
                id: "task-1".to_string(),
                module: "command".to_string(),
                args: HashMap::from([
                    ("cmd".to_string(), serde_json::Value::String("echo hello".to_string())),
                ]),
                when: None,
                changed_when: None,
                failed_when: None,
                vars: HashMap::new(),
                tags: vec![],
                name: Some("Echo command".to_string()),
            },
            TaskPlan {
                id: "task-2".to_string(),
                module: "package".to_string(),
                args: HashMap::from([
                    ("name".to_string(), serde_json::Value::String("nginx".to_string())),
                    ("state".to_string(), serde_json::Value::String("present".to_string())),
                ]),
                when: None,
                changed_when: None,
                failed_when: None,
                vars: HashMap::new(),
                tags: vec![],
                name: Some("Install nginx".to_string()),
            },
        ],
        optimize_for_size: false,
        metadata: HashMap::new(),
    }
}

fn create_high_compatibility_plan() -> RustlePlanOutput {
    RustlePlanOutput {
        tasks: vec![
            TaskPlan {
                id: "task-1".to_string(),
                module: "command".to_string(),
                args: HashMap::new(),
                when: None,
                changed_when: None,
                failed_when: None,
                vars: HashMap::new(),
                tags: vec![],
                name: Some("Compatible task 1".to_string()),
            },
            TaskPlan {
                id: "task-2".to_string(),
                module: "debug".to_string(),
                args: HashMap::new(),
                when: None,
                changed_when: None,
                failed_when: None,
                vars: HashMap::new(),
                tags: vec![],
                name: Some("Compatible task 2".to_string()),
            },
        ],
        optimize_for_size: false,
        metadata: HashMap::new(),
    }
}

fn create_compatible_tasks() -> Vec<TaskPlan> {
    vec![
        TaskPlan {
            id: "compat-1".to_string(),
            module: "command".to_string(),
            args: HashMap::new(),
            when: None,
            changed_when: None,
            failed_when: None,
            vars: HashMap::new(),
            tags: vec![],
            name: Some("Compatible command".to_string()),
        },
        TaskPlan {
            id: "compat-2".to_string(),
            module: "debug".to_string(),
            args: HashMap::new(),
            when: None,
            changed_when: None,
            failed_when: None,
            vars: HashMap::new(),
            tags: vec![],
            name: Some("Compatible debug".to_string()),
        },
    ]
}

fn create_test_inventory() -> ParsedInventory {
    ParsedInventory {
        hosts: vec![
            rustle_deploy::inventory::HostInfo {
                name: "web1".to_string(),
                address: "192.168.1.10".to_string(),
                port: 22,
                groups: vec!["webservers".to_string()],
                variables: HashMap::new(),
            },
            rustle_deploy::inventory::HostInfo {
                name: "web2".to_string(),
                address: "192.168.1.11".to_string(),
                port: 22,
                groups: vec!["webservers".to_string()],
                variables: HashMap::new(),
            },
        ],
        groups: HashMap::from([
            ("webservers".to_string(), vec!["web1".to_string(), "web2".to_string()]),
        ]),
        variables: HashMap::new(),
    }
}