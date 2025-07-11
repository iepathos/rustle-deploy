use rustle_deploy::execution::{Condition, ConditionOperator};
use rustle_deploy::runtime::{ConditionEvaluator, ConditionContext, TaskResult};
use serde_json::json;
use std::collections::HashMap;

#[test]
fn test_equals_condition() {
    let context = create_test_context();
    
    let condition = Condition {
        variable: "test_var".to_string(),
        operator: ConditionOperator::Equals,
        value: json!("test_value"),
    };
    
    assert!(ConditionEvaluator::evaluate_condition(&condition, &context).unwrap());
    
    let false_condition = Condition {
        variable: "test_var".to_string(),
        operator: ConditionOperator::Equals,
        value: json!("wrong_value"),
    };
    
    assert!(!ConditionEvaluator::evaluate_condition(&false_condition, &context).unwrap());
}

#[test]
fn test_not_equals_condition() {
    let context = create_test_context();
    
    let condition = Condition {
        variable: "test_var".to_string(),
        operator: ConditionOperator::NotEquals,
        value: json!("wrong_value"),
    };
    
    assert!(ConditionEvaluator::evaluate_condition(&condition, &context).unwrap());
    
    let false_condition = Condition {
        variable: "test_var".to_string(),
        operator: ConditionOperator::NotEquals,
        value: json!("test_value"),
    };
    
    assert!(!ConditionEvaluator::evaluate_condition(&false_condition, &context).unwrap());
}

#[test]
fn test_contains_condition() {
    let context = create_test_context();
    
    let condition = Condition {
        variable: "test_string".to_string(),
        operator: ConditionOperator::Contains,
        value: json!("hello"),
    };
    
    assert!(ConditionEvaluator::evaluate_condition(&condition, &context).unwrap());
    
    let false_condition = Condition {
        variable: "test_string".to_string(),
        operator: ConditionOperator::Contains,
        value: json!("xyz"),
    };
    
    assert!(!ConditionEvaluator::evaluate_condition(&false_condition, &context).unwrap());
}

#[test]
fn test_array_contains_condition() {
    let context = create_test_context();
    
    let condition = Condition {
        variable: "test_array".to_string(),
        operator: ConditionOperator::Contains,
        value: json!("item2"),
    };
    
    assert!(ConditionEvaluator::evaluate_condition(&condition, &context).unwrap());
    
    let false_condition = Condition {
        variable: "test_array".to_string(),
        operator: ConditionOperator::Contains,
        value: json!("missing_item"),
    };
    
    assert!(!ConditionEvaluator::evaluate_condition(&false_condition, &context).unwrap());
}

#[test]
fn test_starts_with_condition() {
    let context = create_test_context();
    
    let condition = Condition {
        variable: "test_string".to_string(),
        operator: ConditionOperator::StartsWith,
        value: json!("hello"),
    };
    
    assert!(ConditionEvaluator::evaluate_condition(&condition, &context).unwrap());
    
    let false_condition = Condition {
        variable: "test_string".to_string(),
        operator: ConditionOperator::StartsWith,
        value: json!("world"),
    };
    
    assert!(!ConditionEvaluator::evaluate_condition(&false_condition, &context).unwrap());
}

#[test]
fn test_ends_with_condition() {
    let context = create_test_context();
    
    let condition = Condition {
        variable: "test_string".to_string(),
        operator: ConditionOperator::EndsWith,
        value: json!("world"),
    };
    
    assert!(ConditionEvaluator::evaluate_condition(&condition, &context).unwrap());
    
    let false_condition = Condition {
        variable: "test_string".to_string(),
        operator: ConditionOperator::EndsWith,
        value: json!("hello"),
    };
    
    assert!(!ConditionEvaluator::evaluate_condition(&false_condition, &context).unwrap());
}

#[test]
fn test_greater_than_condition() {
    let context = create_test_context();
    
    let condition = Condition {
        variable: "test_number".to_string(),
        operator: ConditionOperator::GreaterThan,
        value: json!(5),
    };
    
    assert!(ConditionEvaluator::evaluate_condition(&condition, &context).unwrap());
    
    let false_condition = Condition {
        variable: "test_number".to_string(),
        operator: ConditionOperator::GreaterThan,
        value: json!(15),
    };
    
    assert!(!ConditionEvaluator::evaluate_condition(&false_condition, &context).unwrap());
}

#[test]
fn test_less_than_condition() {
    let context = create_test_context();
    
    let condition = Condition {
        variable: "test_number".to_string(),
        operator: ConditionOperator::LessThan,
        value: json!(15),
    };
    
    assert!(ConditionEvaluator::evaluate_condition(&condition, &context).unwrap());
    
    let false_condition = Condition {
        variable: "test_number".to_string(),
        operator: ConditionOperator::LessThan,
        value: json!(5),
    };
    
    assert!(!ConditionEvaluator::evaluate_condition(&false_condition, &context).unwrap());
}

#[test]
fn test_exists_condition() {
    let context = create_test_context();
    
    let condition = Condition {
        variable: "test_var".to_string(),
        operator: ConditionOperator::Exists,
        value: json!(null),
    };
    
    assert!(ConditionEvaluator::evaluate_condition(&condition, &context).unwrap());
    
    let false_condition = Condition {
        variable: "nonexistent_var".to_string(),
        operator: ConditionOperator::Exists,
        value: json!(null),
    };
    
    assert!(!ConditionEvaluator::evaluate_condition(&false_condition, &context).unwrap());
}

#[test]
fn test_not_exists_condition() {
    let context = create_test_context();
    
    let condition = Condition {
        variable: "nonexistent_var".to_string(),
        operator: ConditionOperator::NotExists,
        value: json!(null),
    };
    
    assert!(ConditionEvaluator::evaluate_condition(&condition, &context).unwrap());
    
    let false_condition = Condition {
        variable: "test_var".to_string(),
        operator: ConditionOperator::NotExists,
        value: json!(null),
    };
    
    assert!(!ConditionEvaluator::evaluate_condition(&false_condition, &context).unwrap());
}

#[test]
fn test_nested_variable_access() {
    let context = create_test_context();
    
    let condition = Condition {
        variable: "nested.inner.value".to_string(),
        operator: ConditionOperator::Equals,
        value: json!("nested_value"),
    };
    
    assert!(ConditionEvaluator::evaluate_condition(&condition, &context).unwrap());
    
    let false_condition = Condition {
        variable: "nested.inner.nonexistent".to_string(),
        operator: ConditionOperator::Exists,
        value: json!(null),
    };
    
    assert!(!ConditionEvaluator::evaluate_condition(&false_condition, &context).unwrap());
}

#[test]
fn test_multiple_conditions() {
    let context = create_test_context();
    
    let conditions = vec![
        Condition {
            variable: "test_var".to_string(),
            operator: ConditionOperator::Equals,
            value: json!("test_value"),
        },
        Condition {
            variable: "test_number".to_string(),
            operator: ConditionOperator::GreaterThan,
            value: json!(5),
        },
    ];
    
    assert!(ConditionEvaluator::evaluate_conditions(&conditions, &context).unwrap());
    
    let conditions_with_false = vec![
        Condition {
            variable: "test_var".to_string(),
            operator: ConditionOperator::Equals,
            value: json!("test_value"),
        },
        Condition {
            variable: "test_number".to_string(),
            operator: ConditionOperator::LessThan,
            value: json!(5),
        },
    ];
    
    assert!(!ConditionEvaluator::evaluate_conditions(&conditions_with_false, &context).unwrap());
}

#[test]
fn test_empty_conditions() {
    let context = create_test_context();
    let conditions = vec![];
    
    // Empty conditions should always evaluate to true
    assert!(ConditionEvaluator::evaluate_conditions(&conditions, &context).unwrap());
}

fn create_test_context() -> ConditionContext {
    let mut facts = HashMap::new();
    facts.insert("test_var".to_string(), json!("test_value"));
    facts.insert("test_string".to_string(), json!("hello world"));
    facts.insert("test_number".to_string(), json!(10));
    facts.insert("test_array".to_string(), json!(["item1", "item2", "item3"]));
    facts.insert("nested".to_string(), json!({
        "inner": {
            "value": "nested_value"
        }
    }));
    
    let variables = HashMap::new();
    let task_results = HashMap::new();
    
    ConditionContext::new(facts, variables, task_results)
}