//! File ownership utilities

use std::path::Path;

use super::FileError;

/// Set file owner and group
pub async fn set_ownership(
    path: &Path,
    owner: Option<&str>,
    group: Option<&str>,
) -> Result<(), FileError> {
    #[cfg(unix)]
    {
        use nix::unistd::chown;

        let uid = if let Some(owner) = owner {
            Some(resolve_user(owner)?)
        } else {
            None
        };

        let gid = if let Some(group) = group {
            Some(resolve_group(group)?)
        } else {
            None
        };

        chown(path, uid, gid).map_err(|_e| FileError::PermissionDenied {
            path: path.display().to_string(),
        })?;
    }

    #[cfg(windows)]
    {
        // Windows doesn't have the same ownership model
        // This is a no-op for now, but could be extended to handle Windows ACLs
        tracing::warn!("File ownership changes are not supported on Windows");
    }

    Ok(())
}

/// Get file owner and group information
pub async fn get_ownership(path: &Path) -> Result<(String, String), FileError> {
    let metadata = tokio::fs::metadata(path).await?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;

        let uid = metadata.uid();
        let gid = metadata.gid();

        let owner = get_username_by_uid(uid).unwrap_or_else(|| uid.to_string());
        let group = get_groupname_by_gid(gid).unwrap_or_else(|| gid.to_string());

        Ok((owner, group))
    }

    #[cfg(windows)]
    {
        // On Windows, return generic values
        Ok(("Administrator".to_string(), "Administrators".to_string()))
    }
}

#[cfg(unix)]
fn resolve_user(user: &str) -> Result<nix::unistd::Uid, FileError> {
    use nix::unistd::Uid;

    // Try parsing as UID first
    if let Ok(uid) = user.parse::<u32>() {
        return Ok(Uid::from_raw(uid));
    }

    // Try resolving username
    if let Some(uid) = get_uid_by_username(user) {
        Ok(Uid::from_raw(uid))
    } else {
        Err(FileError::NotFound {
            path: format!("User: {user}"),
        })
    }
}

#[cfg(unix)]
fn resolve_group(group: &str) -> Result<nix::unistd::Gid, FileError> {
    use nix::unistd::Gid;

    // Try parsing as GID first
    if let Ok(gid) = group.parse::<u32>() {
        return Ok(Gid::from_raw(gid));
    }

    // Try resolving group name
    if let Some(gid) = get_gid_by_groupname(group) {
        Ok(Gid::from_raw(gid))
    } else {
        Err(FileError::NotFound {
            path: format!("Group: {group}"),
        })
    }
}

#[cfg(unix)]
fn get_uid_by_username(username: &str) -> Option<u32> {
    use std::ffi::CString;

    let c_username = CString::new(username).ok()?;

    unsafe {
        let passwd = libc::getpwnam(c_username.as_ptr());
        if !passwd.is_null() {
            Some((*passwd).pw_uid)
        } else {
            None
        }
    }
}

#[cfg(unix)]
fn get_gid_by_groupname(groupname: &str) -> Option<u32> {
    use std::ffi::CString;

    let c_groupname = CString::new(groupname).ok()?;

    unsafe {
        let group = libc::getgrnam(c_groupname.as_ptr());
        if !group.is_null() {
            Some((*group).gr_gid)
        } else {
            None
        }
    }
}

#[cfg(unix)]
fn get_username_by_uid(uid: u32) -> Option<String> {
    use std::ffi::CStr;

    unsafe {
        let passwd = libc::getpwuid(uid);
        if !passwd.is_null() {
            let c_str = CStr::from_ptr((*passwd).pw_name);
            c_str.to_str().ok().map(|s| s.to_string())
        } else {
            None
        }
    }
}

#[cfg(unix)]
fn get_groupname_by_gid(gid: u32) -> Option<String> {
    use std::ffi::CStr;

    unsafe {
        let group = libc::getgrgid(gid);
        if !group.is_null() {
            let c_str = CStr::from_ptr((*group).gr_name);
            c_str.to_str().ok().map(|s| s.to_string())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[tokio::test]
    async fn test_get_ownership() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let (owner, group) = get_ownership(temp_file.path()).await.unwrap();

        // Should be able to get some owner/group (exact values depend on system)
        assert!(!owner.is_empty());
        assert!(!group.is_empty());
    }
}
