//! Advanced Handlebars helpers for Jinja2 compatibility

use handlebars::{
    Context, Handlebars, Helper, HelperResult, Output, RenderContext, RenderError,
    RenderErrorReason,
};
use serde_json::Value;

/// Enhanced default helper that handles various types
pub fn default_helper(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _rc: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let value_param = h.param(0);
    let default_val = h
        .param(1)
        .map(|v| match v.value() {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "".to_string(),
            _ => v.value().to_string(),
        })
        .unwrap_or_default();

    let result = if let Some(value_param) = value_param {
        match value_param.value() {
            Value::Null => &default_val,
            Value::String(s) => {
                if s.is_empty() {
                    &default_val
                } else {
                    s
                }
            }
            Value::Number(n) => {
                // For numbers, use the actual value (convert to string)
                let n_str = n.to_string();
                out.write(&n_str)?;
                return Ok(());
            }
            Value::Bool(b) => {
                // For booleans, use the actual value (convert to string)
                let b_str = b.to_string();
                out.write(&b_str)?;
                return Ok(());
            }
            _ => {
                // For other types, convert to string
                let val_str = value_param.value().to_string();
                out.write(&val_str)?;
                return Ok(());
            }
        }
    } else {
        &default_val
    };

    out.write(result)?;
    Ok(())
}

/// Quote helper for wrapping values in quotes
pub fn quote_helper(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _rc: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    if let Some(value) = h.param(0) {
        let value_str = match value.value() {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            _ => value.value().to_string(),
        };
        let quoted = format!("\"{value_str}\"");
        out.write(&quoted)?;
    }
    Ok(())
}

/// Equality comparison helper
pub fn equality_helper(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _rc: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let left = h.param(0).ok_or_else(|| {
        RenderError::from(RenderErrorReason::Other(
            "eq helper requires two parameters".to_string(),
        ))
    })?;
    let right = h.param(1).ok_or_else(|| {
        RenderError::from(RenderErrorReason::Other(
            "eq helper requires two parameters".to_string(),
        ))
    })?;

    let result = match (left.value(), right.value()) {
        (Value::String(l), Value::String(r)) => l == r,
        (Value::Number(l), Value::Number(r)) => l == r,
        (Value::Bool(l), Value::Bool(r)) => l == r,
        (Value::Null, Value::Null) => true,
        _ => {
            // Convert to strings for comparison
            let l_str = match left.value() {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                Value::Null => "".to_string(),
                _ => left.value().to_string(),
            };
            let r_str = match right.value() {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                Value::Null => "".to_string(),
                _ => right.value().to_string(),
            };
            l_str == r_str
        }
    };

    out.write(&result.to_string())?;
    Ok(())
}

/// Not equal comparison helper
pub fn not_equal_helper(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _rc: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    // Use the same logic as equality helper but invert the result
    let left = h.param(0).ok_or_else(|| {
        RenderError::from(RenderErrorReason::Other(
            "ne helper requires two parameters".to_string(),
        ))
    })?;
    let right = h.param(1).ok_or_else(|| {
        RenderError::from(RenderErrorReason::Other(
            "ne helper requires two parameters".to_string(),
        ))
    })?;

    let result = match (left.value(), right.value()) {
        (Value::String(l), Value::String(r)) => l != r,
        (Value::Number(l), Value::Number(r)) => l != r,
        (Value::Bool(l), Value::Bool(r)) => l != r,
        (Value::Null, Value::Null) => false,
        _ => {
            // Convert to strings for comparison
            let l_str = match left.value() {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                Value::Null => "".to_string(),
                _ => left.value().to_string(),
            };
            let r_str = match right.value() {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                Value::Null => "".to_string(),
                _ => right.value().to_string(),
            };
            l_str != r_str
        }
    };

    out.write(&result.to_string())?;
    Ok(())
}

/// Greater than comparison helper
pub fn greater_than_helper(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _rc: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let left = h.param(0).ok_or_else(|| {
        RenderError::from(RenderErrorReason::Other(
            "gt helper requires two parameters".to_string(),
        ))
    })?;
    let right = h.param(1).ok_or_else(|| {
        RenderError::from(RenderErrorReason::Other(
            "gt helper requires two parameters".to_string(),
        ))
    })?;

    let result = match (left.value(), right.value()) {
        (Value::Number(l), Value::Number(r)) => {
            if let (Some(l_f), Some(r_f)) = (l.as_f64(), r.as_f64()) {
                l_f > r_f
            } else {
                false
            }
        }
        _ => {
            // Try to parse as numbers for comparison
            let l_str = left.value().to_string();
            let r_str = right.value().to_string();
            if let (Ok(l_num), Ok(r_num)) = (l_str.parse::<f64>(), r_str.parse::<f64>()) {
                l_num > r_num
            } else {
                // String comparison as fallback
                l_str > r_str
            }
        }
    };

    out.write(&result.to_string())?;
    Ok(())
}

/// Less than comparison helper
pub fn less_than_helper(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _rc: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let left = h.param(0).ok_or_else(|| {
        RenderError::from(RenderErrorReason::Other(
            "lt helper requires two parameters".to_string(),
        ))
    })?;
    let right = h.param(1).ok_or_else(|| {
        RenderError::from(RenderErrorReason::Other(
            "lt helper requires two parameters".to_string(),
        ))
    })?;

    let result = match (left.value(), right.value()) {
        (Value::Number(l), Value::Number(r)) => {
            if let (Some(l_f), Some(r_f)) = (l.as_f64(), r.as_f64()) {
                l_f < r_f
            } else {
                false
            }
        }
        _ => {
            // Try to parse as numbers for comparison
            let l_str = left.value().to_string();
            let r_str = right.value().to_string();
            if let (Ok(l_num), Ok(r_num)) = (l_str.parse::<f64>(), r_str.parse::<f64>()) {
                l_num < r_num
            } else {
                // String comparison as fallback
                l_str < r_str
            }
        }
    };

    out.write(&result.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use handlebars::Handlebars;
    use serde_json::json;

    #[test]
    fn test_default_helper() {
        let mut handlebars = Handlebars::new();
        handlebars.register_helper("default", Box::new(default_helper));

        // Test with existing value
        let template = "{{default name 'unknown'}}";
        let data = json!({"name": "Alice"});
        let result = handlebars.render_template(template, &data).unwrap();
        assert_eq!(result, "Alice");

        // Test with empty value
        let data = json!({"name": ""});
        let result = handlebars.render_template(template, &data).unwrap();
        assert_eq!(result, "unknown");

        // Test with missing value
        let data = json!({});
        let result = handlebars.render_template(template, &data).unwrap();
        assert_eq!(result, "unknown");
    }

    #[test]
    fn test_equality_helper() {
        let mut handlebars = Handlebars::new();
        handlebars.register_helper("eq", Box::new(equality_helper));

        let template = "{{eq name 'Alice'}}";
        let data = json!({"name": "Alice"});
        let result = handlebars.render_template(template, &data).unwrap();
        assert_eq!(result, "true");

        let data = json!({"name": "Bob"});
        let result = handlebars.render_template(template, &data).unwrap();
        assert_eq!(result, "false");
    }

    #[test]
    fn test_quote_helper() {
        let mut handlebars = Handlebars::new();
        handlebars.register_helper("quote", Box::new(quote_helper));

        let template = "{{quote value}}";
        let data = json!({"value": "test"});
        let result = handlebars.render_template(template, &data).unwrap();
        assert_eq!(result, "\"test\"");
    }
}
