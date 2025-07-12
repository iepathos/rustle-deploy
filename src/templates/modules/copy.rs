use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub async fn execute(args: HashMap<String, Value>) -> Result<Value> {
    let src = args.get("src")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing 'src' parameter"))?;
    
    let dest = args.get("dest")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing 'dest' parameter"))?;

    let backup = args.get("backup")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let mode = args.get("mode")
        .and_then(|v| v.as_str());

    let src_path = Path::new(src);
    let dest_path = Path::new(dest);

    // Create destination directory if it doesn't exist
    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Backup existing file if requested
    let mut backup_file = None;
    if backup && dest_path.exists() {
        let backup_path = format!("{}.backup", dest);
        fs::copy(dest_path, &backup_path)?;
        backup_file = Some(backup_path);
    }

    // Copy the file
    fs::copy(src_path, dest_path)?;

    // Set permissions if specified (Unix only)
    #[cfg(unix)]
    if let Some(mode_str) = mode {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(mode_val) = u32::from_str_radix(mode_str, 8) {
            let perms = fs::Permissions::from_mode(mode_val);
            fs::set_permissions(dest_path, perms)?;
        }
    }

    Ok(serde_json::json!({
        "changed": true,
        "failed": false,
        "src": src,
        "dest": dest,
        "backup_file": backup_file,
        "msg": "File copied successfully"
    }))
}