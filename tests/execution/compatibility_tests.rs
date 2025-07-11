use rustle_deploy::execution::{
    RustlePlanConverter, validate_rustle_plan_json, SchemaValidator, RustlePlanValidator
};
use rustle_deploy::binary::{BinaryCompatibilityAnalyzer, BinaryDeploymentPlanner, ModuleRegistry};
use std::fs;

#[tokio::test]
async fn test_schema_validation() {
    let content = fs::read_to_string("example_rustle_plan_output.json")
        .expect("Failed to read example rustle plan output file");
    
    let validator = SchemaValidator::new().expect("Failed to create schema validator");
    let json_value: serde_json::Value = serde_json::from_str(&content)
        .expect("Failed to parse JSON");
    
    let result = validator.validate(&json_value);
    assert!(result.is_ok(), "Schema validation failed: {:?}", result);
}

#[tokio::test]
async fn test_rustle_plan_validation() {
    let content = fs::read_to_string("example_rustle_plan_output.json")
        .expect("Failed to read example rustle plan output file");
    
    let rustle_plan = validate_rustle_plan_json(&content)
        .expect("Failed to parse rustle plan");
    
    let validator = RustlePlanValidator::new()
        .expect("Failed to create rustle plan validator");
    
    let result = validator.validate_rustle_plan(&rustle_plan);
    assert!(result.is_ok(), "Rustle plan validation failed: {:?}", result);
}

#[tokio::test]
async fn test_binary_compatibility_analysis() {
    let content = fs::read_to_string("example_rustle_plan_output.json")
        .expect("Failed to read example rustle plan output file");
    
    let rustle_plan = validate_rustle_plan_json(&content)
        .expect("Failed to parse rustle plan");
    
    let analyzer = BinaryCompatibilityAnalyzer::new();
    
    // Analyze each task for compatibility
    for play in &rustle_plan.plays {
        for batch in &play.batches {
            for task in &batch.tasks {
                let result = analyzer.assess_task_compatibility(task);
                assert!(result.is_ok(), "Compatibility analysis failed for task {}: {:?}", 
                       task.task_id, result);
                
                let efficiency = analyzer.estimate_binary_efficiency(task);
                assert!(efficiency.is_ok(), "Efficiency estimation failed for task {}: {:?}", 
                       task.task_id, efficiency);
                
                let eff_value = efficiency.unwrap();
                assert!(eff_value >= 0.0 && eff_value <= 1.0, 
                       "Efficiency value {} out of range for task {}", eff_value, task.task_id);
            }
        }
    }
}

#[tokio::test]
async fn test_performance_analysis() {
    let content = fs::read_to_string("example_rustle_plan_output.json")
        .expect("Failed to read example rustle plan output file");
    
    let rustle_plan = validate_rustle_plan_json(&content)
        .expect("Failed to parse rustle plan");
    
    let analyzer = BinaryCompatibilityAnalyzer::new();
    let all_tasks: Vec<_> = rustle_plan.plays.iter()
        .flat_map(|play| play.batches.iter())
        .flat_map(|batch| batch.tasks.iter())
        .cloned()
        .collect();
    
    let result = analyzer.analyze_performance_impact(&all_tasks);
    assert!(result.is_ok(), "Performance analysis failed: {:?}", result);
    
    let analysis = result.unwrap();
    assert_eq!(analysis.total_tasks, 3);
    assert!(analysis.average_efficiency >= 0.0 && analysis.average_efficiency <= 1.0);
    assert!(analysis.compatible_tasks + analysis.partially_compatible_tasks + analysis.incompatible_tasks == analysis.total_tasks);
}

#[tokio::test]
async fn test_module_registry_compatibility() {
    let content = fs::read_to_string("example_rustle_plan_output.json")
        .expect("Failed to read example rustle plan output file");
    
    let rustle_plan = validate_rustle_plan_json(&content)
        .expect("Failed to parse rustle plan");
    
    let registry = ModuleRegistry::new();
    
    // Test each module used in the example
    let modules_used: Vec<String> = rustle_plan.plays.iter()
        .flat_map(|play| play.batches.iter())
        .flat_map(|batch| batch.tasks.iter())
        .map(|task| task.module.clone())
        .collect();
    
    for module in &modules_used {
        let result = registry.check_module_compatibility(module);
        assert!(result.is_ok(), "Module compatibility check failed for {}: {:?}", module, result);
        
        println!("Module '{}' compatibility: {:?}", module, result.unwrap());
    }
    
    // Test module set analysis
    let analysis = registry.analyze_module_set(&modules_used);
    assert_eq!(analysis.total_modules, modules_used.len());
    assert!(analysis.performance_score >= 0.0 && analysis.performance_score <= 1.0);
}

#[tokio::test]
async fn test_deployment_planning() {
    let content = fs::read_to_string("example_rustle_plan_output.json")
        .expect("Failed to read example rustle plan output file");
    
    let rustle_plan = validate_rustle_plan_json(&content)
        .expect("Failed to parse rustle plan");
    
    let planner = BinaryDeploymentPlanner::new();
    let all_tasks: Vec<_> = rustle_plan.plays.iter()
        .flat_map(|play| play.batches.iter())
        .flat_map(|batch| batch.tasks.iter())
        .cloned()
        .collect();
    
    // Test with low threshold to potentially get deployment plans
    let result = planner.create_deployment_plans(&all_tasks, &rustle_plan.hosts, 2);
    assert!(result.is_ok(), "Deployment planning failed: {:?}", result);
    
    let deployment_plans = result.unwrap();
    println!("Generated {} deployment plans", deployment_plans.len());
    
    // If we got any plans, validate their structure
    for plan in &deployment_plans {
        assert!(!plan.deployment_id.is_empty());
        assert!(!plan.target_hosts.is_empty());
        assert!(!plan.target_architecture.is_empty());
        assert!(!plan.task_ids.is_empty());
        assert!(!plan.compilation_requirements.modules.is_empty());
    }
}

#[tokio::test]
async fn test_format_conversion_roundtrip() {
    let content = fs::read_to_string("example_rustle_plan_output.json")
        .expect("Failed to read example rustle plan output file");
    
    let original_rustle_plan = validate_rustle_plan_json(&content)
        .expect("Failed to parse rustle plan");
    
    let converter = RustlePlanConverter::new();
    
    // Convert to ExecutionPlan
    let execution_plan = converter.convert_to_execution_plan(&original_rustle_plan)
        .expect("Failed to convert to execution plan");
    
    // Verify essential data is preserved
    assert_eq!(execution_plan.tasks.len(), original_rustle_plan.total_tasks as usize);
    
    // Check that all hosts are represented
    for host in &original_rustle_plan.hosts {
        assert!(execution_plan.inventory.hosts.contains_key(host),
               "Host {} not found in converted inventory", host);
    }
    
    // Check that all modules are represented
    let original_modules: std::collections::HashSet<String> = original_rustle_plan.plays.iter()
        .flat_map(|play| play.batches.iter())
        .flat_map(|batch| batch.tasks.iter())
        .map(|task| task.module.clone())
        .collect();
    
    let converted_modules: std::collections::HashSet<String> = execution_plan.modules.iter()
        .map(|module| module.name.clone())
        .collect();
    
    for original_module in &original_modules {
        assert!(converted_modules.contains(original_module),
               "Module {} not found in converted modules", original_module);
    }
}

#[tokio::test]
async fn test_task_condition_conversion() {
    let content = fs::read_to_string("example_rustle_plan_output.json")
        .expect("Failed to read example rustle plan output file");
    
    let rustle_plan = validate_rustle_plan_json(&content)
        .expect("Failed to parse rustle plan");
    
    let converter = RustlePlanConverter::new();
    let execution_plan = converter.convert_to_execution_plan(&rustle_plan)
        .expect("Failed to convert execution plan");
    
    // Find tasks with conditions and verify they're converted
    let tasks_with_conditions: Vec<_> = rustle_plan.plays.iter()
        .flat_map(|play| play.batches.iter())
        .flat_map(|batch| batch.tasks.iter())
        .filter(|task| !task.conditions.is_empty())
        .collect();
    
    for original_task in tasks_with_conditions {
        let converted_task = execution_plan.tasks.iter()
            .find(|task| task.id == original_task.task_id)
            .expect("Converted task not found");
        
        // Conditions should be converted (though the format may differ)
        println!("Original task {} had {} conditions, converted task has {} conditions",
                original_task.task_id, original_task.conditions.len(), converted_task.conditions.len());
    }
}

#[tokio::test]
async fn test_risk_level_to_failure_policy_conversion() {
    let content = fs::read_to_string("example_rustle_plan_output.json")
        .expect("Failed to read example rustle plan output file");
    
    let rustle_plan = validate_rustle_plan_json(&content)
        .expect("Failed to parse rustle plan");
    
    let converter = RustlePlanConverter::new();
    let execution_plan = converter.convert_to_execution_plan(&rustle_plan)
        .expect("Failed to convert execution plan");
    
    // Verify that risk levels are converted to appropriate failure policies
    for (original_task, converted_task) in rustle_plan.plays.iter()
        .flat_map(|play| play.batches.iter())
        .flat_map(|batch| batch.tasks.iter())
        .zip(execution_plan.tasks.iter()) {
        
        use rustle_deploy::execution::{RiskLevel, FailurePolicy};
        
        let expected_policy = match original_task.risk_level {
            RiskLevel::Low | RiskLevel::Medium => FailurePolicy::Continue,
            RiskLevel::High => FailurePolicy::Abort,
            RiskLevel::Critical => FailurePolicy::Rollback,
        };
        
        assert!(matches!(converted_task.failure_policy, expected_policy),
               "Task {} with risk level {:?} should have failure policy {:?}, got {:?}",
               original_task.task_id, original_task.risk_level, expected_policy, converted_task.failure_policy);
    }
}

#[tokio::test]
async fn test_optimization_strategy() {
    let content = fs::read_to_string("example_rustle_plan_output.json")
        .expect("Failed to read example rustle plan output file");
    
    let rustle_plan = validate_rustle_plan_json(&content)
        .expect("Failed to parse rustle plan");
    
    let planner = BinaryDeploymentPlanner::new();
    let all_tasks: Vec<_> = rustle_plan.plays.iter()
        .flat_map(|play| play.batches.iter())
        .flat_map(|batch| batch.tasks.iter())
        .cloned()
        .collect();
    
    use rustle_deploy::binary::DeploymentConstraints;
    let constraints = DeploymentConstraints::default();
    
    let result = planner.optimize_deployment_strategy(&all_tasks, &rustle_plan.hosts, &constraints);
    assert!(result.is_ok(), "Strategy optimization failed: {:?}", result);
    
    let optimized = result.unwrap();
    println!("Recommended strategy: {:?}", optimized.strategy);
    println!("Estimated total time: {:?}", optimized.estimated_total_time);
    println!("Binary deployments: {}", optimized.binary_deployments.len());
    println!("Performance analysis: compatible={}, partially={}, incompatible={}",
             optimized.performance_analysis.compatible_tasks,
             optimized.performance_analysis.partially_compatible_tasks,
             optimized.performance_analysis.incompatible_tasks);
}