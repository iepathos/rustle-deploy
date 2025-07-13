//! Git credential handling utilities

use git2::{Cred, CredentialType};
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum CredentialError {
    #[error("Git credential error: {0}")]
    Git(#[from] git2::Error),
    #[error("SSH key not found: {0}")]
    SshKeyNotFound(String),
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
}

pub struct CredentialHandler {
    ssh_key_path: Option<String>,
    username: Option<String>,
    password: Option<String>,
}

impl CredentialHandler {
    pub fn new() -> Self {
        Self {
            ssh_key_path: None,
            username: None,
            password: None,
        }
    }

    pub fn with_ssh_key<P: AsRef<Path>>(mut self, key_path: P) -> Self {
        self.ssh_key_path = Some(key_path.as_ref().to_string_lossy().to_string());
        self
    }

    pub fn with_userpass(mut self, username: String, password: String) -> Self {
        self.username = Some(username);
        self.password = Some(password);
        self
    }

    pub fn get_credentials(
        &self,
        username_from_url: Option<&str>,
        allowed_types: CredentialType,
    ) -> Result<Cred, CredentialError> {
        // Try SSH key authentication first
        if allowed_types.contains(CredentialType::SSH_KEY) {
            if let Some(key_path) = &self.ssh_key_path {
                let username = username_from_url.unwrap_or("git");
                return Ok(Cred::ssh_key(
                    username,
                    None, // No public key file
                    Path::new(key_path),
                    None, // No passphrase for now
                )?);
            }

            // Try SSH agent
            if let Ok(cred) = Cred::ssh_key_from_agent(username_from_url.unwrap_or("git")) {
                return Ok(cred);
            }

            // Try default SSH key locations
            let username = username_from_url.unwrap_or("git");
            let home_dir = dirs::home_dir().ok_or_else(|| {
                CredentialError::SshKeyNotFound("Unable to find home directory".to_string())
            })?;

            let default_keys = [
                home_dir.join(".ssh/id_rsa"),
                home_dir.join(".ssh/id_ed25519"),
                home_dir.join(".ssh/id_ecdsa"),
            ];

            for key_path in &default_keys {
                if key_path.exists() {
                    if let Ok(cred) = Cred::ssh_key(username, None, key_path, None) {
                        return Ok(cred);
                    }
                }
            }
        }

        // Try username/password authentication
        if allowed_types.contains(CredentialType::USER_PASS_PLAINTEXT) {
            if let (Some(username), Some(password)) = (&self.username, &self.password) {
                return Ok(Cred::userpass_plaintext(username, password)?);
            }

            // For HTTPS URLs, try without password (for token-based auth)
            if let Some(username) = &self.username {
                return Ok(Cred::userpass_plaintext(username, "")?);
            }
        }

        Err(CredentialError::AuthenticationFailed(
            "No suitable authentication method found".to_string(),
        ))
    }

    /// Check if SSH key exists and is readable
    pub fn validate_ssh_key(&self) -> Result<(), CredentialError> {
        if let Some(key_path) = &self.ssh_key_path {
            let path = Path::new(key_path);
            if !path.exists() {
                return Err(CredentialError::SshKeyNotFound(format!(
                    "SSH key not found: {key_path}"
                )));
            }

            // Check if file is readable
            std::fs::metadata(path).map_err(|e| {
                CredentialError::SshKeyNotFound(format!("Cannot read SSH key {key_path}: {e}"))
            })?;
        }
        Ok(())
    }
}

impl Default for CredentialHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_credential_handler_creation() {
        let handler = CredentialHandler::new();
        assert!(handler.ssh_key_path.is_none());
        assert!(handler.username.is_none());
        assert!(handler.password.is_none());
    }

    #[test]
    fn test_credential_handler_with_ssh_key() {
        let temp_file = NamedTempFile::new().unwrap();
        let handler = CredentialHandler::new().with_ssh_key(temp_file.path());

        assert!(handler.ssh_key_path.is_some());
        assert!(handler.validate_ssh_key().is_ok());
    }

    #[test]
    fn test_credential_handler_with_userpass() {
        let handler =
            CredentialHandler::new().with_userpass("testuser".to_string(), "testpass".to_string());

        assert_eq!(handler.username, Some("testuser".to_string()));
        assert_eq!(handler.password, Some("testpass".to_string()));
    }

    #[test]
    fn test_ssh_key_validation_failure() {
        let handler = CredentialHandler::new().with_ssh_key("/nonexistent/key");

        assert!(handler.validate_ssh_key().is_err());
    }
}
