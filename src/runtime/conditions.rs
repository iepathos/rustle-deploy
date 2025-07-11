use crate::execution::{Condition, ConditionOperator};
use crate::runtime::ExecutionError;
use serde_json::Value;
use std::collections::HashMap;

/// Condition evaluator for task execution
pub struct ConditionEvaluator;

impl ConditionEvaluator {
    /// Evaluate a list of conditions (all must be true)
    pub fn evaluate_conditions(
        conditions: &[Condition],
        context: &ConditionContext,
    ) -> Result<bool, ExecutionError> {
        if conditions.is_empty() {
            return Ok(true);
        }

        for condition in conditions {
            if !Self::evaluate_condition(condition, context)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Evaluate a single condition
    pub fn evaluate_condition(
        condition: &Condition,
        context: &ConditionContext,
    ) -> Result<bool, ExecutionError> {
        let variable_value = Self::resolve_variable(&condition.variable, context)?;

        match condition.operator {
            ConditionOperator::Equals => Ok(Self::values_equal(&variable_value, &condition.value)),
            ConditionOperator::NotEquals => {
                Ok(!Self::values_equal(&variable_value, &condition.value))
            }
            ConditionOperator::Contains => {
                Self::evaluate_contains(&variable_value, &condition.value)
            }
            ConditionOperator::StartsWith => {
                Self::evaluate_starts_with(&variable_value, &condition.value)
            }
            ConditionOperator::EndsWith => {
                Self::evaluate_ends_with(&variable_value, &condition.value)
            }
            ConditionOperator::GreaterThan => {
                Self::evaluate_greater_than(&variable_value, &condition.value)
            }
            ConditionOperator::LessThan => {
                Self::evaluate_less_than(&variable_value, &condition.value)
            }
            ConditionOperator::Exists => Ok(!variable_value.is_null()),
            ConditionOperator::NotExists => Ok(variable_value.is_null()),
        }
    }

    fn resolve_variable(
        variable_name: &str,
        context: &ConditionContext,
    ) -> Result<Value, ExecutionError> {
        // Handle nested variable access (e.g., "ansible_facts.hostname")
        let parts: Vec<&str> = variable_name.split('.').collect();

        // Look in variables first, then facts
        let mut current_value = context
            .variables
            .get(parts[0])
            .or_else(|| context.facts.get(parts[0]))
            .cloned()
            .unwrap_or(Value::Null);

        // Navigate nested properties
        for part in &parts[1..] {
            if let Value::Object(map) = current_value {
                current_value = map.get(*part).cloned().unwrap_or(Value::Null);
            } else {
                current_value = Value::Null;
                break;
            }
        }

        Ok(current_value)
    }

    fn values_equal(a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Null, Value::Null) => true,
            // Try string comparison for mixed types
            _ => {
                let a_str = Self::value_to_string(a);
                let b_str = Self::value_to_string(b);
                a_str == b_str
            }
        }
    }

    fn evaluate_contains(haystack: &Value, needle: &Value) -> Result<bool, ExecutionError> {
        match (haystack, needle) {
            (Value::String(haystack), Value::String(needle)) => Ok(haystack.contains(needle)),
            (Value::Array(haystack), needle) => {
                Ok(haystack.iter().any(|item| Self::values_equal(item, needle)))
            }
            (Value::Object(haystack), Value::String(key)) => Ok(haystack.contains_key(key)),
            _ => Err(ExecutionError::ConditionFailed {
                condition: format!(
                    "contains operation not supported for types: {} contains {}",
                    Self::type_name(haystack),
                    Self::type_name(needle)
                ),
            }),
        }
    }

    fn evaluate_starts_with(value: &Value, prefix: &Value) -> Result<bool, ExecutionError> {
        match (value, prefix) {
            (Value::String(value), Value::String(prefix)) => Ok(value.starts_with(prefix)),
            _ => {
                let value_str = Self::value_to_string(value);
                let prefix_str = Self::value_to_string(prefix);
                Ok(value_str.starts_with(&prefix_str))
            }
        }
    }

    fn evaluate_ends_with(value: &Value, suffix: &Value) -> Result<bool, ExecutionError> {
        match (value, suffix) {
            (Value::String(value), Value::String(suffix)) => Ok(value.ends_with(suffix)),
            _ => {
                let value_str = Self::value_to_string(value);
                let suffix_str = Self::value_to_string(suffix);
                Ok(value_str.ends_with(&suffix_str))
            }
        }
    }

    fn evaluate_greater_than(a: &Value, b: &Value) -> Result<bool, ExecutionError> {
        match (a, b) {
            (Value::Number(a), Value::Number(b)) => {
                if let (Some(a_f), Some(b_f)) = (a.as_f64(), b.as_f64()) {
                    Ok(a_f > b_f)
                } else {
                    Err(ExecutionError::ConditionFailed {
                        condition: "Cannot compare non-numeric values with >".to_string(),
                    })
                }
            }
            _ => Err(ExecutionError::ConditionFailed {
                condition: format!(
                    "Cannot compare {} > {} (unsupported types)",
                    Self::type_name(a),
                    Self::type_name(b)
                ),
            }),
        }
    }

    fn evaluate_less_than(a: &Value, b: &Value) -> Result<bool, ExecutionError> {
        match (a, b) {
            (Value::Number(a), Value::Number(b)) => {
                if let (Some(a_f), Some(b_f)) = (a.as_f64(), b.as_f64()) {
                    Ok(a_f < b_f)
                } else {
                    Err(ExecutionError::ConditionFailed {
                        condition: "Cannot compare non-numeric values with <".to_string(),
                    })
                }
            }
            _ => Err(ExecutionError::ConditionFailed {
                condition: format!(
                    "Cannot compare {} < {} (unsupported types)",
                    Self::type_name(a),
                    Self::type_name(b)
                ),
            }),
        }
    }

    fn value_to_string(value: &Value) -> String {
        match value {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            _ => value.to_string(),
        }
    }

    fn type_name(value: &Value) -> &'static str {
        match value {
            Value::String(_) => "string",
            Value::Number(_) => "number",
            Value::Bool(_) => "boolean",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
            Value::Null => "null",
        }
    }
}

/// Context for condition evaluation
pub struct ConditionContext {
    pub facts: HashMap<String, Value>,
    pub variables: HashMap<String, Value>,
    pub task_results: HashMap<String, crate::runtime::TaskResult>,
}

impl ConditionContext {
    pub fn new(
        facts: HashMap<String, Value>,
        variables: HashMap<String, Value>,
        task_results: HashMap<String, crate::runtime::TaskResult>,
    ) -> Self {
        Self {
            facts,
            variables,
            task_results,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_equals_condition() {
        let context = ConditionContext::new(
            [(String::from("hostname"), json!("test-host"))].into(),
            HashMap::new(),
            HashMap::new(),
        );

        let condition = Condition {
            variable: "hostname".to_string(),
            operator: ConditionOperator::Equals,
            value: json!("test-host"),
        };

        assert!(ConditionEvaluator::evaluate_condition(&condition, &context).unwrap());
    }

    #[test]
    fn test_contains_condition() {
        let context = ConditionContext::new(
            [(String::from("os_family"), json!("RedHat"))].into(),
            HashMap::new(),
            HashMap::new(),
        );

        let condition = Condition {
            variable: "os_family".to_string(),
            operator: ConditionOperator::Contains,
            value: json!("Red"),
        };

        assert!(ConditionEvaluator::evaluate_condition(&condition, &context).unwrap());
    }

    #[test]
    fn test_nested_variable_access() {
        let context = ConditionContext::new(
            [(
                String::from("system"),
                json!({"kernel": {"version": "5.4.0"}}),
            )]
            .into(),
            HashMap::new(),
            HashMap::new(),
        );

        let condition = Condition {
            variable: "system.kernel.version".to_string(),
            operator: ConditionOperator::Equals,
            value: json!("5.4.0"),
        };

        assert!(ConditionEvaluator::evaluate_condition(&condition, &context).unwrap());
    }
}
