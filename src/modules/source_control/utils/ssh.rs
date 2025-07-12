//! SSH utilities for Git operations

use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum SshError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("SSH key error: {0}")]
    SshKey(String),
    #[error("Known hosts error: {0}")]
    KnownHosts(String),
}

pub struct SshManager {
    ssh_dir: PathBuf,
    known_hosts: HashMap<String, String>,
}

impl SshManager {
    pub fn new() -> Result<Self, SshError> {
        let home_dir = dirs::home_dir()
            .ok_or_else(|| SshError::SshKey("Unable to find home directory".to_string()))?;

        let ssh_dir = home_dir.join(".ssh");

        Ok(Self {
            ssh_dir,
            known_hosts: HashMap::new(),
        })
    }

    /// Find available SSH keys in the default SSH directory
    pub fn find_available_keys(&self) -> Vec<PathBuf> {
        let mut keys = Vec::new();

        let key_files = ["id_rsa", "id_ed25519", "id_ecdsa", "id_dsa"];

        for key_file in &key_files {
            let key_path = self.ssh_dir.join(key_file);
            if key_path.exists() {
                keys.push(key_path);
            }
        }

        keys
    }

    /// Get the default SSH key (first available key)
    pub fn get_default_key(&self) -> Option<PathBuf> {
        self.find_available_keys().into_iter().next()
    }

    /// Validate that an SSH key exists and has correct permissions
    pub fn validate_key(&self, key_path: &Path) -> Result<(), SshError> {
        if !key_path.exists() {
            return Err(SshError::SshKey(format!(
                "SSH key does not exist: {}",
                key_path.display()
            )));
        }

        let metadata = std::fs::metadata(key_path)?;

        // Check permissions on Unix systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = metadata.permissions().mode();

            // SSH keys should be readable only by owner (600 or 400)
            if mode & 0o077 != 0 {
                tracing::warn!(
                    "SSH key {} has overly permissive permissions: {:o}",
                    key_path.display(),
                    mode
                );
            }
        }

        Ok(())
    }

    /// Add a host to known hosts
    pub fn add_known_host(&mut self, hostname: &str, key: &str) {
        self.known_hosts
            .insert(hostname.to_string(), key.to_string());
    }

    /// Check if a host is in known hosts
    pub fn is_known_host(&self, hostname: &str) -> bool {
        self.known_hosts.contains_key(hostname) || self.is_in_known_hosts_file(hostname)
    }

    /// Check if a host is in the system known_hosts file
    fn is_in_known_hosts_file(&self, hostname: &str) -> bool {
        let known_hosts_files = [
            self.ssh_dir.join("known_hosts"),
            PathBuf::from("/etc/ssh/ssh_known_hosts"),
        ];

        for file_path in &known_hosts_files {
            if file_path.exists() {
                if let Ok(content) = std::fs::read_to_string(file_path) {
                    for line in content.lines() {
                        if line.starts_with(hostname) || line.contains(&format!(" {} ", hostname)) {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    /// Extract hostname from Git URL
    pub fn extract_hostname(url: &str) -> Option<String> {
        if url.starts_with("git@") {
            // SSH format: git@github.com:user/repo.git
            if let Some(colon_pos) = url.find(':') {
                let host_part = &url[4..colon_pos]; // Skip "git@"
                return Some(host_part.to_string());
            }
        } else if url.starts_with("ssh://") {
            // SSH URL format: ssh://git@github.com/user/repo.git
            if let Ok(parsed_url) = url::Url::parse(url) {
                return parsed_url.host_str().map(|h| h.to_string());
            }
        } else if url.starts_with("https://") || url.starts_with("http://") {
            // HTTPS/HTTP format
            if let Ok(parsed_url) = url::Url::parse(url) {
                return parsed_url.host_str().map(|h| h.to_string());
            }
        }

        None
    }

    /// Generate SSH config for a specific host
    pub fn generate_ssh_config(&self, hostname: &str, key_path: &Path) -> String {
        format!(
            "Host {}\n    HostName {}\n    User git\n    IdentityFile {}\n    StrictHostKeyChecking no\n",
            hostname,
            hostname,
            key_path.display()
        )
    }

    /// Check if SSH agent is available
    pub fn is_ssh_agent_available(&self) -> bool {
        std::env::var("SSH_AUTH_SOCK").is_ok()
    }
}

impl Default for SshManager {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| {
            // Fallback for systems without home directory
            Self {
                ssh_dir: PathBuf::from(".ssh"),
                known_hosts: HashMap::new(),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hostname_extraction() {
        assert_eq!(
            SshManager::extract_hostname("git@github.com:user/repo.git"),
            Some("github.com".to_string())
        );

        assert_eq!(
            SshManager::extract_hostname("https://github.com/user/repo.git"),
            Some("github.com".to_string())
        );

        assert_eq!(
            SshManager::extract_hostname("ssh://git@gitlab.com/user/repo.git"),
            Some("gitlab.com".to_string())
        );

        assert_eq!(SshManager::extract_hostname("invalid-url"), None);
    }

    #[test]
    fn test_ssh_config_generation() {
        let ssh_manager = SshManager::default();
        let config =
            ssh_manager.generate_ssh_config("github.com", Path::new("/home/user/.ssh/id_rsa"));

        assert!(config.contains("Host github.com"));
        assert!(config.contains("HostName github.com"));
        assert!(config.contains("User git"));
        assert!(config.contains("IdentityFile /home/user/.ssh/id_rsa"));
    }

    #[test]
    fn test_known_host_management() {
        let mut ssh_manager = SshManager::default();

        assert!(!ssh_manager.is_known_host("example.com"));

        ssh_manager.add_known_host("example.com", "ssh-rsa AAAAB3...");
        assert!(ssh_manager.is_known_host("example.com"));
    }

    #[test]
    fn test_ssh_agent_detection() {
        let ssh_manager = SshManager::default();

        // This test depends on the environment, so we just check that it doesn't panic
        let _agent_available = ssh_manager.is_ssh_agent_available();
    }
}
