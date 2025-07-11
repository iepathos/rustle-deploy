use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;

pub async fn execute(args: HashMap<String, Value>) -> Result<Value> {
    let msg = args.get("msg")
        .and_then(|v| v.as_str())
        .unwrap_or("Debug message");

    let var = args.get("var").cloned();

    println!("DEBUG: {}", msg);
    if let Some(var_value) = &var {
        println!("DEBUG var: {}", var_value);
    }

    Ok(serde_json::json!({
        "changed": false,
        "failed": false,
        "msg": msg,
        "var": var
    }))
}