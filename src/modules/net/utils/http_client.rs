//! HTTP client wrapper with comprehensive options

use reqwest::{Client, ClientBuilder, Method};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    HEAD,
    OPTIONS,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BodyFormat {
    Json,
    FormUrlencoded,
    Raw,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FollowRedirects {
    None,
    Safe,
    All,
}

#[derive(Debug, thiserror::Error)]
pub enum HttpClientError {
    #[error("HTTP request error: {0}")]
    Request(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid certificate: {0}")]
    Certificate(String),
    #[error("Authentication failed: {0}")]
    Authentication(String),
    #[error("Unexpected status code: expected {expected:?}, got {actual}")]
    UnexpectedStatusCode { expected: Vec<u16>, actual: u16 },
}

pub struct HttpClientWrapper {
    client: Client,
}

impl HttpClientWrapper {
    pub fn new(
        timeout: Option<u64>,
        validate_certs: Option<bool>,
        client_cert: Option<&str>,
        client_key: Option<&str>,
        ca_path: Option<&str>,
        follow_redirects: Option<&FollowRedirects>,
    ) -> Result<Self, HttpClientError> {
        let mut builder = ClientBuilder::new();

        // Configure timeout
        if let Some(timeout_secs) = timeout {
            builder = builder.timeout(Duration::from_secs(timeout_secs));
        }

        // Configure certificate validation
        if let Some(validate) = validate_certs {
            builder = builder.danger_accept_invalid_certs(!validate);
            if !validate {
                builder = builder.danger_accept_invalid_hostnames(true);
            }
        }

        // Configure client certificates
        if let (Some(cert_path), Some(key_path)) = (client_cert, client_key) {
            let _cert_data = std::fs::read(cert_path)?;
            let _key_data = std::fs::read(key_path)?;

            // Client certificate support is not available in this version of reqwest
            tracing::warn!("Client certificate authentication not yet fully implemented");
        }

        // Configure CA certificate
        if let Some(ca_cert_path) = ca_path {
            let ca_cert = std::fs::read(ca_cert_path)?;
            let cert = reqwest::Certificate::from_pem(&ca_cert).map_err(|e| {
                HttpClientError::Certificate(format!("Failed to load CA certificate: {e}"))
            })?;

            builder = builder.add_root_certificate(cert);
        }

        // Configure redirects
        match follow_redirects.unwrap_or(&FollowRedirects::Safe) {
            FollowRedirects::None => {
                builder = builder.redirect(reqwest::redirect::Policy::none());
            }
            FollowRedirects::Safe => {
                builder = builder.redirect(reqwest::redirect::Policy::limited(10));
            }
            FollowRedirects::All => {
                builder = builder.redirect(reqwest::redirect::Policy::limited(20));
            }
        }

        // Set a reasonable user agent
        builder = builder.user_agent("rustle-deploy/1.0");

        let client = builder.build()?;
        Ok(Self { client })
    }

    pub async fn execute_request(
        &self,
        url: &str,
        method: &HttpMethod,
        headers: Option<&HashMap<String, String>>,
        body: Option<&str>,
        body_format: Option<&BodyFormat>,
        user: Option<&str>,
        password: Option<&str>,
        expected_status: Option<&[u16]>,
    ) -> Result<HttpResponse, HttpClientError> {
        let http_method = match method {
            HttpMethod::GET => Method::GET,
            HttpMethod::POST => Method::POST,
            HttpMethod::PUT => Method::PUT,
            HttpMethod::DELETE => Method::DELETE,
            HttpMethod::PATCH => Method::PATCH,
            HttpMethod::HEAD => Method::HEAD,
            HttpMethod::OPTIONS => Method::OPTIONS,
        };

        let mut request = self.client.request(http_method, url);

        // Add headers
        if let Some(headers) = headers {
            for (key, value) in headers {
                request = request.header(key, value);
            }
        }

        // Add authentication
        if let (Some(username), Some(pass)) = (user, password) {
            request = request.basic_auth(username, Some(pass));
        }

        // Add body
        if let Some(body_content) = body {
            request = match body_format.unwrap_or(&BodyFormat::Raw) {
                BodyFormat::Json => request
                    .header("Content-Type", "application/json")
                    .body(body_content.to_string()),
                BodyFormat::FormUrlencoded => request
                    .header("Content-Type", "application/x-www-form-urlencoded")
                    .body(body_content.to_string()),
                BodyFormat::Raw => request.body(body_content.to_string()),
            };
        }

        let response = request.send().await?;

        // Validate status code
        let status_code = response.status().as_u16();
        if let Some(expected_codes) = expected_status {
            if !expected_codes.contains(&status_code) {
                return Err(HttpClientError::UnexpectedStatusCode {
                    expected: expected_codes.to_vec(),
                    actual: status_code,
                });
            }
        }

        // Extract response data
        let status = status_code;
        let headers: HashMap<String, String> = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        let content = response.text().await?;

        Ok(HttpResponse {
            status,
            headers,
            content,
        })
    }

    pub async fn download_file(
        &self,
        url: &str,
        headers: Option<&HashMap<String, String>>,
        user: Option<&str>,
        password: Option<&str>,
        resume_from: Option<u64>,
    ) -> Result<reqwest::Response, HttpClientError> {
        let mut request = self.client.get(url);

        // Add headers
        if let Some(headers) = headers {
            for (key, value) in headers {
                request = request.header(key, value);
            }
        }

        // Add range header for resume
        if let Some(offset) = resume_from {
            request = request.header("Range", format!("bytes={offset}-"));
        }

        // Add authentication
        if let (Some(username), Some(pass)) = (user, password) {
            request = request.basic_auth(username, Some(pass));
        }

        let response = request.send().await?;

        if !response.status().is_success()
            && response.status() != reqwest::StatusCode::PARTIAL_CONTENT
        {
            return Err(HttpClientError::Request(
                response.error_for_status().unwrap_err(),
            ));
        }

        Ok(response)
    }
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub content: String,
}

impl HttpResponse {
    pub fn is_success(&self) -> bool {
        self.status >= 200 && self.status < 300
    }

    pub fn is_redirect(&self) -> bool {
        self.status >= 300 && self.status < 400
    }

    pub fn is_client_error(&self) -> bool {
        self.status >= 400 && self.status < 500
    }

    pub fn is_server_error(&self) -> bool {
        self.status >= 500
    }

    pub fn get_header(&self, name: &str) -> Option<&String> {
        self.headers.get(&name.to_lowercase())
    }

    pub fn content_type(&self) -> Option<&String> {
        self.get_header("content-type")
    }

    pub fn content_length(&self) -> Option<u64> {
        self.get_header("content-length")
            .and_then(|s| s.parse().ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_response_status_checks() {
        let response = HttpResponse {
            status: 200,
            headers: HashMap::new(),
            content: "OK".to_string(),
        };
        assert!(response.is_success());
        assert!(!response.is_redirect());
        assert!(!response.is_client_error());
        assert!(!response.is_server_error());

        let response = HttpResponse {
            status: 404,
            headers: HashMap::new(),
            content: "Not Found".to_string(),
        };
        assert!(!response.is_success());
        assert!(response.is_client_error());
    }

    #[test]
    fn test_header_access() {
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());
        headers.insert("content-length".to_string(), "123".to_string());

        let response = HttpResponse {
            status: 200,
            headers,
            content: "{}".to_string(),
        };

        assert_eq!(
            response.content_type(),
            Some(&"application/json".to_string())
        );
        assert_eq!(response.content_length(), Some(123));
    }

    #[test]
    fn test_http_method_conversion() {
        // This test just ensures the enum values exist and can be matched
        match HttpMethod::GET {
            HttpMethod::GET => {}
            _ => panic!("Unexpected HTTP method"),
        }
    }
}
