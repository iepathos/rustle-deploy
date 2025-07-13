use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::net::TcpStream;
use std::time::{Duration, Instant};
use tokio::time::sleep;

pub async fn execute(args: HashMap<String, Value>) -> Result<Value> {
    let host = args.get("host")
        .and_then(|v| v.as_str())
        .unwrap_or("localhost");

    let port = args.get("port")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow::anyhow!("Missing 'port' parameter"))?;

    let timeout = args.get("timeout")
        .and_then(|v| v.as_u64())
        .unwrap_or(30);

    let delay = args.get("delay")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    // Apply initial delay if specified
    if delay > 0 {
        sleep(Duration::from_secs(delay)).await;
    }

    let start_time = Instant::now();
    let timeout_duration = Duration::from_secs(timeout);
    let address = format!("{}:{}", host, port);

    // Try to connect to the port until timeout
    loop {
        match TcpStream::connect(&address) {
            Ok(_) => {
                return Ok(serde_json::json!({
                    "changed": false,
                    "failed": false,
                    "msg": format!("Port {} on {} is available", port, host),
                    "elapsed": start_time.elapsed().as_secs_f64()
                }));
            }
            Err(_) => {
                if start_time.elapsed() >= timeout_duration {
                    return Ok(serde_json::json!({
                        "changed": false,
                        "failed": true,
                        "msg": format!("Timeout waiting for port {} on {} ({}s)", port, host, timeout),
                        "elapsed": start_time.elapsed().as_secs_f64()
                    }));
                }
                // Wait a bit before trying again
                sleep(Duration::from_millis(100)).await;
            }
        }
    }
}