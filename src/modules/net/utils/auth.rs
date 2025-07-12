//! Authentication utilities for network operations

use base64::Engine;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum AuthMethod {
    Basic { username: String, password: String },
    Bearer { token: String },
    ApiKey { key: String, header: String },
    Custom { headers: HashMap<String, String> },
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid credentials format: {0}")]
    InvalidFormat(String),
    #[error("Missing required authentication parameter: {0}")]
    MissingParameter(String),
    #[error("Authentication method not supported: {0}")]
    UnsupportedMethod(String),
}

pub struct AuthHandler;

impl AuthHandler {
    /// Create basic authentication header
    pub fn create_basic_auth(username: &str, password: &str) -> String {
        let credentials = format!("{}:{}", username, password);
        let encoded = base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes());
        format!("Basic {}", encoded)
    }

    /// Create bearer token authentication header
    pub fn create_bearer_auth(token: &str) -> String {
        format!("Bearer {}", token)
    }

    /// Create API key authentication header
    pub fn create_api_key_auth(key: &str, header_name: &str) -> (String, String) {
        (header_name.to_string(), key.to_string())
    }

    /// Apply authentication to headers
    pub fn apply_auth(
        headers: &mut HashMap<String, String>,
        auth_method: &AuthMethod,
    ) -> Result<(), AuthError> {
        match auth_method {
            AuthMethod::Basic { username, password } => {
                let auth_header = Self::create_basic_auth(username, password);
                headers.insert("Authorization".to_string(), auth_header);
            }
            AuthMethod::Bearer { token } => {
                let auth_header = Self::create_bearer_auth(token);
                headers.insert("Authorization".to_string(), auth_header);
            }
            AuthMethod::ApiKey { key, header } => {
                headers.insert(header.clone(), key.clone());
            }
            AuthMethod::Custom {
                headers: auth_headers,
            } => {
                for (key, value) in auth_headers {
                    headers.insert(key.clone(), value.clone());
                }
            }
        }
        Ok(())
    }

    /// Parse authentication from URL (for basic auth in URLs)
    pub fn parse_url_auth(url: &str) -> Option<(String, String, String)> {
        if let Ok(parsed_url) = url::Url::parse(url) {
            let username = parsed_url.username();
            if !username.is_empty() {
                let password = parsed_url.password().unwrap_or("");
                let clean_url = format!(
                    "{}://{}{}{}",
                    parsed_url.scheme(),
                    parsed_url.host_str().unwrap_or(""),
                    if let Some(port) = parsed_url.port() {
                        format!(":{}", port)
                    } else {
                        String::new()
                    },
                    parsed_url.path()
                );
                return Some((username.to_string(), password.to_string(), clean_url));
            }
        }
        None
    }

    /// Validate authentication parameters
    pub fn validate_auth(auth_method: &AuthMethod) -> Result<(), AuthError> {
        match auth_method {
            AuthMethod::Basic {
                username,
                password: _,
            } => {
                if username.is_empty() {
                    return Err(AuthError::MissingParameter("username".to_string()));
                }
                // Password can be empty for some token-based systems
            }
            AuthMethod::Bearer { token } => {
                if token.is_empty() {
                    return Err(AuthError::MissingParameter("token".to_string()));
                }
            }
            AuthMethod::ApiKey { key, header } => {
                if key.is_empty() {
                    return Err(AuthError::MissingParameter("key".to_string()));
                }
                if header.is_empty() {
                    return Err(AuthError::MissingParameter("header".to_string()));
                }
            }
            AuthMethod::Custom { headers } => {
                if headers.is_empty() {
                    return Err(AuthError::MissingParameter("headers".to_string()));
                }
            }
        }
        Ok(())
    }

    /// Extract bearer token from Authorization header
    pub fn extract_bearer_token(auth_header: &str) -> Option<String> {
        if auth_header.starts_with("Bearer ") {
            Some(auth_header[7..].to_string())
        } else {
            None
        }
    }

    /// Extract basic auth credentials from Authorization header
    pub fn extract_basic_auth(auth_header: &str) -> Option<(String, String)> {
        if auth_header.starts_with("Basic ") {
            let encoded = &auth_header[6..];
            if let Ok(decoded_bytes) = base64::engine::general_purpose::STANDARD.decode(encoded) {
                if let Ok(decoded_str) = String::from_utf8(decoded_bytes) {
                    if let Some(colon_pos) = decoded_str.find(':') {
                        let username = decoded_str[..colon_pos].to_string();
                        let password = decoded_str[colon_pos + 1..].to_string();
                        return Some((username, password));
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_auth_creation() {
        let auth_header = AuthHandler::create_basic_auth("user", "pass");
        assert_eq!(auth_header, "Basic dXNlcjpwYXNz");
    }

    #[test]
    fn test_bearer_auth_creation() {
        let auth_header = AuthHandler::create_bearer_auth("token123");
        assert_eq!(auth_header, "Bearer token123");
    }

    #[test]
    fn test_api_key_auth_creation() {
        let (header, value) = AuthHandler::create_api_key_auth("key123", "X-API-Key");
        assert_eq!(header, "X-API-Key");
        assert_eq!(value, "key123");
    }

    #[test]
    fn test_auth_application() {
        let mut headers = HashMap::new();

        let auth_method = AuthMethod::Basic {
            username: "user".to_string(),
            password: "pass".to_string(),
        };

        AuthHandler::apply_auth(&mut headers, &auth_method).unwrap();
        assert!(headers.contains_key("Authorization"));
        assert_eq!(headers.get("Authorization").unwrap(), "Basic dXNlcjpwYXNz");
    }

    #[test]
    fn test_url_auth_parsing() {
        let result = AuthHandler::parse_url_auth("https://user:pass@example.com/path");
        assert!(result.is_some());

        let (username, password, clean_url) = result.unwrap();
        assert_eq!(username, "user");
        assert_eq!(password, "pass");
        assert_eq!(clean_url, "https://example.com/path");
    }

    #[test]
    fn test_bearer_token_extraction() {
        let token = AuthHandler::extract_bearer_token("Bearer token123");
        assert_eq!(token, Some("token123".to_string()));

        let no_token = AuthHandler::extract_bearer_token("Basic dXNlcjpwYXNz");
        assert_eq!(no_token, None);
    }

    #[test]
    fn test_basic_auth_extraction() {
        let result = AuthHandler::extract_basic_auth("Basic dXNlcjpwYXNz");
        assert_eq!(result, Some(("user".to_string(), "pass".to_string())));

        let no_result = AuthHandler::extract_basic_auth("Bearer token123");
        assert_eq!(no_result, None);
    }

    #[test]
    fn test_auth_validation() {
        let valid_basic = AuthMethod::Basic {
            username: "user".to_string(),
            password: "pass".to_string(),
        };
        assert!(AuthHandler::validate_auth(&valid_basic).is_ok());

        let invalid_basic = AuthMethod::Basic {
            username: "".to_string(),
            password: "pass".to_string(),
        };
        assert!(AuthHandler::validate_auth(&invalid_basic).is_err());

        let valid_bearer = AuthMethod::Bearer {
            token: "token123".to_string(),
        };
        assert!(AuthHandler::validate_auth(&valid_bearer).is_ok());

        let invalid_bearer = AuthMethod::Bearer {
            token: "".to_string(),
        };
        assert!(AuthHandler::validate_auth(&invalid_bearer).is_err());
    }
}
