use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::process::Command;

pub async fn execute(args: HashMap<String, Value>) -> Result<Value> {
    let cmd = args.get("cmd")
        .or_else(|| args.get("command"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing 'cmd' or 'command' parameter"))?;

    let output = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(&["/C", cmd])
            .output()?
    } else {
        Command::new("sh")
            .args(&["-c", cmd])
            .output()?
    };

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let rc = output.status.code().unwrap_or(-1);

    Ok(serde_json::json!({
        "changed": rc == 0,
        "failed": rc != 0,
        "rc": rc,
        "stdout": stdout,
        "stderr": stderr,
        "msg": if rc == 0 { "Command executed successfully" } else { "Command failed" }
    }))
}