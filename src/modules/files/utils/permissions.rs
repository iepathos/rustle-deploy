//! Cross-platform file permission utilities

use std::path::Path;

use super::FileError;

/// Set file permissions using string format (e.g., "0644", "u+rwx", etc.)
pub async fn set_permissions(path: &Path, mode: &str) -> Result<(), FileError> {
    let parsed_mode = parse_mode(mode)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = tokio::fs::metadata(path).await?;
        let mut permissions = metadata.permissions();
        permissions.set_mode(parsed_mode);
        tokio::fs::set_permissions(path, permissions).await?;
    }

    #[cfg(windows)]
    {
        // Windows has limited permission support
        // We'll handle basic read-only vs read-write
        let mut permissions = tokio::fs::metadata(path).await?.permissions();

        // If mode suggests read-only (no write bits set)
        if parsed_mode & 0o200 == 0 {
            permissions.set_readonly(true);
        } else {
            permissions.set_readonly(false);
        }

        tokio::fs::set_permissions(path, permissions).await?;
    }

    Ok(())
}

/// Parse permission mode string into numeric format
fn parse_mode(mode: &str) -> Result<u32, FileError> {
    if mode.starts_with("0") || mode.chars().all(|c| c.is_ascii_digit()) {
        // Octal format (e.g., "0644", "644")
        let mode_str = mode.trim_start_matches('0');
        u32::from_str_radix(mode_str, 8).map_err(|_| FileError::InvalidPermissions {
            mode: mode.to_string(),
        })
    } else if mode.contains('+') || mode.contains('-') || mode.contains('=') {
        // Symbolic format (e.g., "u+rwx", "go-w")
        parse_symbolic_mode(mode)
    } else {
        Err(FileError::InvalidPermissions {
            mode: mode.to_string(),
        })
    }
}

/// Parse symbolic permission mode (u+rwx, go-w, etc.)
fn parse_symbolic_mode(mode: &str) -> Result<u32, FileError> {
    let mut result = 0o644; // Default permissions

    for part in mode.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        let (who, op_and_perms) = if let Some(pos) = part.find(|c| c == '+' || c == '-' || c == '=')
        {
            (&part[..pos], &part[pos..])
        } else {
            return Err(FileError::InvalidPermissions {
                mode: mode.to_string(),
            });
        };

        let op = op_and_perms.chars().next().unwrap();
        let perms = &op_and_perms[1..];

        let who_mask = parse_who(who)?;
        let perm_bits = parse_permissions(perms)?;

        match op {
            '+' => result |= who_mask & perm_bits,
            '-' => result &= !(who_mask & perm_bits),
            '=' => {
                // Clear existing bits for this who, then set new ones
                result &= !who_mask;
                result |= who_mask & perm_bits;
            }
            _ => {
                return Err(FileError::InvalidPermissions {
                    mode: mode.to_string(),
                });
            }
        }
    }

    Ok(result)
}

/// Parse "who" part of symbolic notation (u, g, o, a)
fn parse_who(who: &str) -> Result<u32, FileError> {
    let mut mask = 0;

    for c in who.chars() {
        match c {
            'u' => mask |= 0o700, // User/owner
            'g' => mask |= 0o070, // Group
            'o' => mask |= 0o007, // Other
            'a' => mask |= 0o777, // All
            _ => {
                return Err(FileError::InvalidPermissions {
                    mode: who.to_string(),
                });
            }
        }
    }

    if mask == 0 {
        mask = 0o777; // Default to all if no who specified
    }

    Ok(mask)
}

/// Parse permission bits (r, w, x)
fn parse_permissions(perms: &str) -> Result<u32, FileError> {
    let mut bits = 0;

    for c in perms.chars() {
        match c {
            'r' => bits |= 0o444, // Read for all
            'w' => bits |= 0o222, // Write for all
            'x' => bits |= 0o111, // Execute for all
            _ => {
                return Err(FileError::InvalidPermissions {
                    mode: perms.to_string(),
                });
            }
        }
    }

    Ok(bits)
}

/// Get current file permissions as octal string
pub async fn get_permissions(path: &Path) -> Result<String, FileError> {
    let metadata = tokio::fs::metadata(path).await?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = metadata.permissions().mode() & 0o777;
        Ok(format!("{:o}", mode))
    }

    #[cfg(windows)]
    {
        // Windows simplified representation
        if metadata.permissions().readonly() {
            Ok("444".to_string()) // Read-only
        } else {
            Ok("644".to_string()) // Read-write
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_octal_mode() {
        assert_eq!(parse_mode("644").unwrap(), 0o644);
        assert_eq!(parse_mode("0755").unwrap(), 0o755);
        assert_eq!(parse_mode("777").unwrap(), 0o777);
    }

    #[test]
    fn test_parse_symbolic_mode() {
        assert_eq!(parse_mode("u+x").unwrap() & 0o100, 0o100);
        assert_eq!(parse_mode("go-w").unwrap() & 0o022, 0);
    }

    #[test]
    fn test_parse_who() {
        assert_eq!(parse_who("u").unwrap(), 0o700);
        assert_eq!(parse_who("g").unwrap(), 0o070);
        assert_eq!(parse_who("o").unwrap(), 0o007);
        assert_eq!(parse_who("a").unwrap(), 0o777);
        assert_eq!(parse_who("ug").unwrap(), 0o770);
    }

    #[test]
    fn test_parse_permissions() {
        assert_eq!(parse_permissions("r").unwrap(), 0o444);
        assert_eq!(parse_permissions("w").unwrap(), 0o222);
        assert_eq!(parse_permissions("x").unwrap(), 0o111);
        assert_eq!(parse_permissions("rw").unwrap(), 0o666);
        assert_eq!(parse_permissions("rwx").unwrap(), 0o777);
    }
}
