//! Certificate validation and management utilities

use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum CertificateError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Certificate parsing error: {0}")]
    Parse(String),
    #[error("Certificate validation error: {0}")]
    Validation(String),
    #[error("Certificate not found: {0}")]
    NotFound(String),
}

pub struct CertificateManager;

impl CertificateManager {
    /// Validate that a certificate file exists and is readable
    pub fn validate_certificate_file(cert_path: &Path) -> Result<Vec<u8>, CertificateError> {
        if !cert_path.exists() {
            return Err(CertificateError::NotFound(
                cert_path.to_string_lossy().to_string(),
            ));
        }

        let cert_data = std::fs::read(cert_path)?;

        // Basic validation - check if it looks like a PEM certificate
        let cert_str = String::from_utf8_lossy(&cert_data);
        if !cert_str.contains("-----BEGIN CERTIFICATE-----") {
            return Err(CertificateError::Parse(
                "File does not appear to contain a PEM certificate".to_string(),
            ));
        }

        Ok(cert_data)
    }

    /// Validate that a private key file exists and is readable
    pub fn validate_private_key_file(key_path: &Path) -> Result<Vec<u8>, CertificateError> {
        if !key_path.exists() {
            return Err(CertificateError::NotFound(
                key_path.to_string_lossy().to_string(),
            ));
        }

        let key_data = std::fs::read(key_path)?;

        // Basic validation - check if it looks like a PEM private key
        let key_str = String::from_utf8_lossy(&key_data);
        if !key_str.contains("-----BEGIN PRIVATE KEY-----")
            && !key_str.contains("-----BEGIN RSA PRIVATE KEY-----")
            && !key_str.contains("-----BEGIN EC PRIVATE KEY-----")
        {
            return Err(CertificateError::Parse(
                "File does not appear to contain a PEM private key".to_string(),
            ));
        }

        // Check permissions on Unix systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = std::fs::metadata(key_path)?;
            let mode = metadata.permissions().mode();

            // Private keys should be readable only by owner (600 or 400)
            if mode & 0o077 != 0 {
                tracing::warn!(
                    "Private key {} has overly permissive permissions: {:o}",
                    key_path.display(),
                    mode
                );
            }
        }

        Ok(key_data)
    }

    /// Load a certificate and private key pair
    pub fn load_certificate_pair(
        cert_path: &Path,
        key_path: &Path,
    ) -> Result<(Vec<u8>, Vec<u8>), CertificateError> {
        let cert_data = Self::validate_certificate_file(cert_path)?;
        let key_data = Self::validate_private_key_file(key_path)?;

        Ok((cert_data, key_data))
    }

    /// Find system CA certificate bundle
    pub fn find_ca_bundle() -> Option<std::path::PathBuf> {
        let ca_paths = [
            // Common CA bundle locations
            "/etc/ssl/certs/ca-certificates.crt", // Debian/Ubuntu
            "/etc/pki/tls/certs/ca-bundle.crt",   // RHEL/CentOS
            "/etc/ssl/ca-bundle.pem",             // OpenSUSE
            "/etc/ssl/cert.pem",                  // OpenBSD
            "/usr/local/share/certs/ca-root-nss.crt", // FreeBSD
            "/etc/pki/tls/cert.pem",              // CentOS/RHEL alternative
        ];

        for path in &ca_paths {
            let path_buf = std::path::PathBuf::from(path);
            if path_buf.exists() {
                return Some(path_buf);
            }
        }

        None
    }

    /// Extract certificate information (basic parsing)
    pub fn extract_certificate_info(cert_data: &[u8]) -> Result<CertificateInfo, CertificateError> {
        let cert_str = String::from_utf8_lossy(cert_data);

        // Very basic parsing - in a real implementation, you'd use a proper certificate parsing library
        let mut info = CertificateInfo {
            subject: None,
            issuer: None,
            serial_number: None,
            not_before: None,
            not_after: None,
        };

        // This is a simplified parser - for production use, consider using a library like 'x509-parser'
        if cert_str.contains("-----BEGIN CERTIFICATE-----") {
            // Extract base64 content (simplified)
            let lines: Vec<&str> = cert_str.lines().collect();
            let start_idx = lines
                .iter()
                .position(|&line| line == "-----BEGIN CERTIFICATE-----");
            let end_idx = lines
                .iter()
                .position(|&line| line == "-----END CERTIFICATE-----");

            if let (Some(start), Some(end)) = (start_idx, end_idx) {
                if start + 1 < end {
                    // Certificate data exists
                    info.subject = Some("Certificate present".to_string());
                }
            }
        }

        Ok(info)
    }

    /// Verify certificate chain (basic check)
    pub fn verify_certificate_chain(
        cert_data: &[u8],
        ca_data: Option<&[u8]>,
    ) -> Result<bool, CertificateError> {
        // This is a placeholder implementation
        // In a real scenario, you'd use a proper TLS/certificate library

        let cert_str = String::from_utf8_lossy(cert_data);
        if !cert_str.contains("-----BEGIN CERTIFICATE-----") {
            return Err(CertificateError::Validation(
                "Invalid certificate format".to_string(),
            ));
        }

        if let Some(ca_cert) = ca_data {
            let ca_str = String::from_utf8_lossy(ca_cert);
            if !ca_str.contains("-----BEGIN CERTIFICATE-----") {
                return Err(CertificateError::Validation(
                    "Invalid CA certificate format".to_string(),
                ));
            }
        }

        // Basic validation passed
        Ok(true)
    }
}

#[derive(Debug, Clone)]
pub struct CertificateInfo {
    pub subject: Option<String>,
    pub issuer: Option<String>,
    pub serial_number: Option<String>,
    pub not_before: Option<String>,
    pub not_after: Option<String>,
}

impl CertificateInfo {
    pub fn new() -> Self {
        Self {
            subject: None,
            issuer: None,
            serial_number: None,
            not_before: None,
            not_after: None,
        }
    }
}

impl Default for CertificateInfo {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_certificate_validation() {
        // Create a temporary file with certificate-like content
        let mut cert_file = NamedTempFile::new().unwrap();
        writeln!(cert_file, "-----BEGIN CERTIFICATE-----").unwrap();
        writeln!(
            cert_file,
            "MIICljCCAX4CCQDAOYKnVgWRFjANBgkqhkiG9w0BAQsFADA..."
        )
        .unwrap();
        writeln!(cert_file, "-----END CERTIFICATE-----").unwrap();
        cert_file.flush().unwrap();

        let result = CertificateManager::validate_certificate_file(cert_file.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_private_key_validation() {
        // Create a temporary file with private key-like content
        let mut key_file = NamedTempFile::new().unwrap();
        writeln!(key_file, "-----BEGIN PRIVATE KEY-----").unwrap();
        writeln!(
            key_file,
            "MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQC..."
        )
        .unwrap();
        writeln!(key_file, "-----END PRIVATE KEY-----").unwrap();
        key_file.flush().unwrap();

        let result = CertificateManager::validate_private_key_file(key_file.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_certificate_info_extraction() {
        let cert_data = b"-----BEGIN CERTIFICATE-----\nMIICljCCAX4CCQDAOYKnVgWRFjANBgkqhkiG9w0BAQsFADA...\n-----END CERTIFICATE-----";

        let result = CertificateManager::extract_certificate_info(cert_data);
        assert!(result.is_ok());

        let info = result.unwrap();
        assert!(info.subject.is_some());
    }

    #[test]
    fn test_invalid_certificate() {
        // Create a temporary file with invalid content
        let mut invalid_file = NamedTempFile::new().unwrap();
        writeln!(invalid_file, "This is not a certificate").unwrap();
        invalid_file.flush().unwrap();

        let result = CertificateManager::validate_certificate_file(invalid_file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_nonexistent_certificate() {
        let nonexistent_path = Path::new("/nonexistent/certificate.pem");
        let result = CertificateManager::validate_certificate_file(nonexistent_path);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CertificateError::NotFound(_)));
    }

    #[test]
    fn test_ca_bundle_detection() {
        // This test just ensures the function doesn't panic
        let _ca_bundle = CertificateManager::find_ca_bundle();
        // We can't assert the result since it depends on the system
    }
}
