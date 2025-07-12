use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub async fn execute(args: HashMap<String, Value>) -> Result<Value> {
    let path = args.get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing 'path' parameter"))?;
    
    let state = args.get("state")
        .and_then(|v| v.as_str())
        .unwrap_or("file");

    let mode = args.get("mode")
        .and_then(|v| v.as_str());

    let recurse = args.get("recurse")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let file_path = Path::new(path);
    let mut changed = false;

    match state {
        "directory" => {
            if !file_path.exists() {
                if recurse {
                    fs::create_dir_all(file_path)?;
                } else {
                    fs::create_dir(file_path)?;
                }
                changed = true;
            }
        },
        "file" => {
            if !file_path.exists() {
                if let Some(parent) = file_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::File::create(file_path)?;
                changed = true;
            }
        },
        "touch" => {
            if !file_path.exists() {
                if let Some(parent) = file_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::File::create(file_path)?;
                changed = true;
            } else {
                // Update timestamp
                let metadata = fs::metadata(file_path)?;
                let now = std::time::SystemTime::now();
                // Note: Setting access time requires platform-specific code
                // For now, we'll just mark as changed if file exists
                changed = true;
            }
        },
        "link" => {
            let src = args.get("src")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing 'src' parameter for link state"))?;
            
            if !file_path.exists() {
                #[cfg(unix)]
                {
                    std::os::unix::fs::symlink(src, file_path)?;
                    changed = true;
                }
                #[cfg(windows)]
                {
                    if Path::new(src).is_dir() {
                        std::os::windows::fs::symlink_dir(src, file_path)?;
                    } else {
                        std::os::windows::fs::symlink_file(src, file_path)?;
                    }
                    changed = true;
                }
            }
        },
        "absent" => {
            if file_path.exists() {
                if file_path.is_dir() {
                    if recurse {
                        fs::remove_dir_all(file_path)?;
                    } else {
                        fs::remove_dir(file_path)?;
                    }
                } else {
                    fs::remove_file(file_path)?;
                }
                changed = true;
            }
        },
        _ => {
            return Err(anyhow::anyhow!("Invalid state: {}", state));
        }
    }

    // Set permissions if specified (Unix only)
    #[cfg(unix)]
    if let Some(mode_str) = mode {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(mode_val) = u32::from_str_radix(mode_str, 8) {
            let perms = fs::Permissions::from_mode(mode_val);
            fs::set_permissions(file_path, perms)?;
            changed = true;
        }
    }

    Ok(serde_json::json!({
        "changed": changed,
        "failed": false,
        "path": path,
        "state": state,
        "msg": format!("File operation '{}' completed successfully", state)
    }))
}