# Spec 160: Archive, Git, and URL Operations Modules

## Feature Summary

Implement essential modules for handling archives (unarchive/archive), version control operations (git), and URL-based operations (uri, get_url). These modules provide critical functionality for software deployment workflows including downloading artifacts, extracting packages, managing source code repositories, and performing HTTP operations for health checks and API interactions.

## Goals & Requirements

### Functional Requirements
- **unarchive module**: Extract tar, zip, gzip, bzip2, xz archives with validation
- **archive module**: Create compressed archives in multiple formats
- **git module**: Clone, pull, checkout, and manage Git repositories
- **uri module**: Perform HTTP/HTTPS requests with comprehensive options
- **get_url module**: Download files from URLs with checksum validation

### Non-Functional Requirements
- Support for all major archive formats (tar, zip, gzip, bzip2, xz, zstd)
- Resume capability for interrupted downloads
- Progress reporting for large operations
- Secure handling of credentials and certificates
- Cross-platform compatibility
- Performance optimization for large files

### Success Criteria
- All modules handle edge cases gracefully
- Comprehensive test coverage including integration tests
- Performance benchmarks for large file operations
- Security audit compliance for credential handling
- Documentation with real-world examples

## API/Interface Design

### Unarchive Module Interface
```rust
pub struct UnarchiveArgs {
    pub src: String,                      // Required: source archive path or URL
    pub dest: String,                     // Required: destination directory
    pub remote_src: Option<bool>,         // Archive is on remote system (default: false)
    pub creates: Option<String>,          // Skip if this file/directory exists
    pub list_files: Option<bool>,         // Return list of extracted files
    pub exclude: Option<Vec<String>>,     // Files/patterns to exclude
    pub include: Option<Vec<String>>,     // Files/patterns to include
    pub keep_newer: Option<bool>,         // Don't overwrite newer files
    pub mode: Option<String>,             // Permissions for extracted files
    pub owner: Option<String>,            // Owner for extracted files
    pub group: Option<String>,            // Group for extracted files
    pub validate_certs: Option<bool>,     // Validate SSL certificates (for URLs)
    pub checksum: Option<String>,         // Expected checksum
}

#[derive(Debug, Clone)]
pub enum ArchiveFormat {
    Tar,
    TarGz,
    TarBz2,
    TarXz,
    Zip,
    SevenZ,
    Rar,
    Auto,  // Auto-detect from extension/magic bytes
}
```

### Archive Module Interface
```rust
pub struct ArchiveArgs {
    pub path: Vec<String>,                // Required: files/directories to archive
    pub dest: String,                     // Required: destination archive path
    pub format: Option<ArchiveFormat>,    // Archive format (auto-detect if not specified)
    pub exclude: Option<Vec<String>>,     // Files/patterns to exclude
    pub exclude_path: Option<Vec<String>>, // Paths to exclude
    pub compression_level: Option<u8>,    // Compression level (1-9)
    pub remove: Option<bool>,             // Remove original files after archiving
    pub mode: Option<String>,             // Permissions for created archive
    pub owner: Option<String>,            // Owner for created archive
    pub group: Option<String>,            // Group for created archive
}
```

### Git Module Interface
```rust
pub struct GitArgs {
    pub repo: String,                     // Required: Git repository URL
    pub dest: String,                     // Required: destination directory
    pub version: Option<String>,          // Branch, tag, or commit (default: HEAD)
    pub force: Option<bool>,              // Discard local changes
    pub depth: Option<u32>,               // Shallow clone depth
    pub clone: Option<bool>,              // Clone if directory doesn't exist
    pub update: Option<bool>,             // Update existing repository
    pub track_submodules: Option<bool>,   // Include submodules
    pub key_file: Option<String>,         // SSH key file path
    pub accept_hostkey: Option<bool>,     // Accept unknown host keys
    pub archive: Option<String>,          // Create archive instead of checkout
    pub separate_git_dir: Option<String>, // Separate git directory
    pub verify_commit: Option<bool>,      // Verify GPG signatures
}
```

### URI Module Interface
```rust
pub struct UriArgs {
    pub url: String,                      // Required: target URL
    pub method: Option<HttpMethod>,       // HTTP method (default: GET)
    pub body: Option<String>,             // Request body
    pub body_format: Option<BodyFormat>,  // Body format (json, form-urlencoded, raw)
    pub headers: Option<HashMap<String, String>>, // HTTP headers
    pub user: Option<String>,             // Username for authentication
    pub password: Option<String>,         // Password for authentication
    pub timeout: Option<u64>,             // Request timeout in seconds
    pub validate_certs: Option<bool>,     // Validate SSL certificates
    pub client_cert: Option<String>,      // Client certificate path
    pub client_key: Option<String>,       // Client private key path
    pub ca_path: Option<String>,          // CA certificate path
    pub follow_redirects: Option<FollowRedirects>, // Redirect handling
    pub status_code: Option<Vec<u16>>,    // Expected status codes
    pub return_content: Option<bool>,     // Return response content
}

#[derive(Debug, Clone)]
pub enum HttpMethod {
    GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS,
}

#[derive(Debug, Clone)]
pub enum BodyFormat {
    Json, FormUrlencoded, Raw,
}

#[derive(Debug, Clone)]
pub enum FollowRedirects {
    None, Safe, All,
}
```

### Get URL Module Interface
```rust
pub struct GetUrlArgs {
    pub url: String,                      // Required: source URL
    pub dest: String,                     // Required: destination file path
    pub checksum: Option<String>,         // Expected checksum (algo:hash)
    pub mode: Option<String>,             // File permissions
    pub owner: Option<String>,            // File owner
    pub group: Option<String>,            // File group
    pub backup: Option<bool>,             // Backup existing file
    pub force: Option<bool>,              // Force download even if file exists
    pub timeout: Option<u64>,             // Download timeout
    pub headers: Option<HashMap<String, String>>, // HTTP headers
    pub user: Option<String>,             // Username for authentication
    pub password: Option<String>,         // Password for authentication
    pub validate_certs: Option<bool>,     // Validate SSL certificates
    pub client_cert: Option<String>,      // Client certificate
    pub client_key: Option<String>,       // Client private key
    pub tmp_dest: Option<String>,         // Temporary download location
}
```

## File and Package Structure

### Module Organization
```
src/modules/
├── files/                    # File operations (existing)
├── net/                      # Network operations
│   ├── mod.rs               # Network module declarations
│   ├── uri.rs               # HTTP/HTTPS requests module
│   ├── get_url.rs           # URL download module
│   └── utils/
│       ├── mod.rs           # Network utilities
│       ├── http_client.rs   # HTTP client wrapper
│       ├── auth.rs          # Authentication handling
│       └── certificates.rs  # Certificate validation
├── source_control/           # Version control modules
│   ├── mod.rs               # Source control declarations
│   ├── git.rs               # Git operations module
│   └── utils/
│       ├── mod.rs           # Git utilities
│       ├── credentials.rs   # Git credential handling
│       └── ssh.rs           # SSH key management
└── archive/                  # Archive operations
    ├── mod.rs               # Archive module declarations
    ├── unarchive.rs         # Archive extraction module
    ├── archive.rs           # Archive creation module
    ├── formats/
    │   ├── mod.rs           # Format handlers
    │   ├── tar.rs           # Tar format handler
    │   ├── zip.rs           # Zip format handler
    │   ├── gzip.rs          # Gzip format handler
    │   └── detection.rs     # Format auto-detection
    └── utils/
        ├── mod.rs           # Archive utilities
        ├── compression.rs   # Compression algorithms
        └── extraction.rs    # Extraction utilities
```

## Implementation Details

### 1. Archive Format Detection and Handling
```rust
use std::io::{Read, Seek};

pub struct ArchiveDetector;

impl ArchiveDetector {
    pub fn detect_format<R: Read + Seek>(reader: &mut R) -> Result<ArchiveFormat, ArchiveError> {
        let mut buffer = [0u8; 512];
        reader.read_exact(&mut buffer)?;
        reader.seek(std::io::SeekFrom::Start(0))?;
        
        // Check magic bytes
        if buffer.starts_with(b"PK\x03\x04") || buffer.starts_with(b"PK\x05\x06") {
            return Ok(ArchiveFormat::Zip);
        }
        
        if buffer.starts_with(&[0x1f, 0x8b]) {
            return Ok(ArchiveFormat::TarGz);
        }
        
        if buffer.starts_with(b"ustar\0") {
            return Ok(ArchiveFormat::Tar);
        }
        
        // Add more format detections...
        
        Ok(ArchiveFormat::Auto)
    }
}

#[async_trait]
pub trait ArchiveHandler: Send + Sync {
    async fn extract(&self, src: &Path, dest: &Path, options: &UnarchiveArgs) -> Result<ExtractionResult, ArchiveError>;
    async fn create(&self, sources: &[PathBuf], dest: &Path, options: &ArchiveArgs) -> Result<CreationResult, ArchiveError>;
}

pub struct TarHandler;

#[async_trait]
impl ArchiveHandler for TarHandler {
    async fn extract(&self, src: &Path, dest: &Path, options: &UnarchiveArgs) -> Result<ExtractionResult, ArchiveError> {
        use flate2::read::GzDecoder;
        use tar::Archive;
        
        let file = std::fs::File::open(src)?;
        let decoder = GzDecoder::new(file);
        let mut archive = Archive::new(decoder);
        
        let mut extracted_files = Vec::new();
        
        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?;
            
            // Apply include/exclude filters
            if should_skip_entry(&path, options) {
                continue;
            }
            
            let dest_path = dest.join(&path);
            entry.unpack(&dest_path)?;
            
            // Set permissions and ownership
            if let Some(mode) = &options.mode {
                set_file_permissions(&dest_path, mode)?;
            }
            
            extracted_files.push(path.to_path_buf());
        }
        
        Ok(ExtractionResult {
            extracted_files,
            total_size: calculate_extracted_size(&extracted_files)?,
        })
    }
}
```

### 2. Git Operations with Authentication
```rust
use git2::{Repository, Cred, RemoteCallbacks, FetchOptions, Progress};

pub struct GitOperations {
    auth_handler: AuthHandler,
}

impl GitOperations {
    pub async fn clone_repository(&self, args: &GitArgs) -> Result<GitResult, GitError> {
        let dest_path = Path::new(&args.dest);
        
        // Set up authentication
        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_url, username_from_url, allowed_types| {
            self.auth_handler.get_credentials(username_from_url, allowed_types, args)
        });
        
        // Set up progress reporting
        callbacks.transfer_progress(|stats| {
            self.report_progress(stats);
            true
        });
        
        // Configure fetch options
        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);
        
        // Perform clone
        let mut builder = git2::build::RepoBuilder::new();
        builder.fetch_options(fetch_options);
        
        if let Some(depth) = args.depth {
            builder.clone_recurse_submodules(args.track_submodules.unwrap_or(false));
        }
        
        let repo = builder.clone(&args.repo, dest_path)?;
        
        // Checkout specific version if requested
        if let Some(version) = &args.version {
            self.checkout_version(&repo, version)?;
        }
        
        Ok(GitResult {
            changed: true,
            before: None,
            after: Some(repo.head()?.target().unwrap().to_string()),
        })
    }
    
    fn checkout_version(&self, repo: &Repository, version: &str) -> Result<(), GitError> {
        // Try to resolve as branch, tag, or commit
        let oid = if let Ok(reference) = repo.resolve_reference_from_short_name(version) {
            reference.target().unwrap()
        } else if let Ok(oid) = git2::Oid::from_str(version) {
            oid
        } else {
            return Err(GitError::InvalidVersion(version.to_string()));
        };
        
        let object = repo.find_object(oid, None)?;
        repo.checkout_tree(&object, None)?;
        repo.set_head_detached(oid)?;
        
        Ok(())
    }
}

pub struct AuthHandler;

impl AuthHandler {
    fn get_credentials(&self, username: Option<&str>, allowed: git2::CredentialType, args: &GitArgs) -> Result<Cred, git2::Error> {
        if allowed.contains(git2::CredentialType::SSH_KEY) {
            if let Some(key_file) = &args.key_file {
                return Cred::ssh_key(
                    username.unwrap_or("git"),
                    None,
                    Path::new(key_file),
                    None,
                );
            }
            
            // Try default SSH key locations
            return Cred::ssh_key_from_agent(username.unwrap_or("git"));
        }
        
        if allowed.contains(git2::CredentialType::USER_PASS_PLAINTEXT) {
            // For HTTPS authentication
            return Cred::userpass_plaintext("", ""); // Will prompt for credentials
        }
        
        Err(git2::Error::from_str("No suitable authentication method"))
    }
}
```

### 3. HTTP Client with Comprehensive Options
```rust
use reqwest::{Client, ClientBuilder, Method, RequestBuilder};
use std::time::Duration;

pub struct HttpClientWrapper {
    client: Client,
}

impl HttpClientWrapper {
    pub fn new(args: &UriArgs) -> Result<Self, UriError> {
        let mut builder = ClientBuilder::new();
        
        // Configure timeout
        if let Some(timeout) = args.timeout {
            builder = builder.timeout(Duration::from_secs(timeout));
        }
        
        // Configure certificate validation
        if let Some(validate_certs) = args.validate_certs {
            builder = builder.danger_accept_invalid_certs(!validate_certs);
        }
        
        // Configure client certificates
        if let (Some(cert_path), Some(key_path)) = (&args.client_cert, &args.client_key) {
            let cert = std::fs::read(cert_path)?;
            let key = std::fs::read(key_path)?;
            let identity = reqwest::Identity::from_pkcs8_pem(&cert, &key)?;
            builder = builder.identity(identity);
        }
        
        // Configure redirects
        match args.follow_redirects.as_ref().unwrap_or(&FollowRedirects::Safe) {
            FollowRedirects::None => builder = builder.redirect(reqwest::redirect::Policy::none()),
            FollowRedirects::Safe => builder = builder.redirect(reqwest::redirect::Policy::limited(10)),
            FollowRedirects::All => builder = builder.redirect(reqwest::redirect::Policy::limited(20)),
        }
        
        let client = builder.build()?;
        Ok(Self { client })
    }
    
    pub async fn execute_request(&self, args: &UriArgs) -> Result<UriResult, UriError> {
        let method = match args.method.as_ref().unwrap_or(&HttpMethod::GET) {
            HttpMethod::GET => Method::GET,
            HttpMethod::POST => Method::POST,
            HttpMethod::PUT => Method::PUT,
            HttpMethod::DELETE => Method::DELETE,
            HttpMethod::PATCH => Method::PATCH,
            HttpMethod::HEAD => Method::HEAD,
            HttpMethod::OPTIONS => Method::OPTIONS,
        };
        
        let mut request = self.client.request(method, &args.url);
        
        // Add headers
        if let Some(headers) = &args.headers {
            for (key, value) in headers {
                request = request.header(key, value);
            }
        }
        
        // Add authentication
        if let (Some(user), Some(password)) = (&args.user, &args.password) {
            request = request.basic_auth(user, Some(password));
        }
        
        // Add body
        if let Some(body) = &args.body {
            request = match args.body_format.as_ref().unwrap_or(&BodyFormat::Raw) {
                BodyFormat::Json => {
                    request.header("Content-Type", "application/json").body(body.clone())
                }
                BodyFormat::FormUrlencoded => {
                    request.header("Content-Type", "application/x-www-form-urlencoded").body(body.clone())
                }
                BodyFormat::Raw => request.body(body.clone()),
            };
        }
        
        let response = request.send().await?;
        
        // Validate status code
        if let Some(expected_codes) = &args.status_code {
            if !expected_codes.contains(&response.status().as_u16()) {
                return Err(UriError::UnexpectedStatusCode {
                    expected: expected_codes.clone(),
                    actual: response.status().as_u16(),
                });
            }
        }
        
        let status = response.status().as_u16();
        let headers = response.headers().clone();
        let content = if args.return_content.unwrap_or(true) {
            Some(response.text().await?)
        } else {
            None
        };
        
        Ok(UriResult {
            status,
            headers: headers.iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                .collect(),
            content,
        })
    }
}
```

### 4. File Download with Resume Support
```rust
pub struct FileDownloader {
    client: Client,
}

impl FileDownloader {
    pub async fn download_file(&self, args: &GetUrlArgs) -> Result<GetUrlResult, GetUrlError> {
        let dest_path = Path::new(&args.dest);
        let temp_path = self.get_temp_path(dest_path, args)?;
        
        // Check if we can resume
        let resume_from = if temp_path.exists() && !args.force.unwrap_or(false) {
            Some(temp_path.metadata()?.len())
        } else {
            None
        };
        
        let mut request = self.client.get(&args.url);
        
        // Add range header for resume
        if let Some(offset) = resume_from {
            request = request.header("Range", format!("bytes={}-", offset));
        }
        
        // Add authentication and headers
        if let (Some(user), Some(password)) = (&args.user, &args.password) {
            request = request.basic_auth(user, Some(password));
        }
        
        if let Some(headers) = &args.headers {
            for (key, value) in headers {
                request = request.header(key, value);
            }
        }
        
        let response = request.send().await?;
        
        // Create or append to temp file
        let mut file = if resume_from.is_some() {
            OpenOptions::new().append(true).open(&temp_path)?
        } else {
            File::create(&temp_path)?
        };
        
        // Stream download with progress reporting
        let mut stream = response.bytes_stream();
        let mut downloaded = resume_from.unwrap_or(0);
        
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk)?;
            downloaded += chunk.len() as u64;
            
            // Report progress if callback provided
            self.report_download_progress(downloaded, None);
        }
        
        file.flush()?;
        drop(file);
        
        // Validate checksum if provided
        if let Some(checksum) = &args.checksum {
            self.validate_checksum(&temp_path, checksum).await?;
        }
        
        // Atomic move to final destination
        tokio::fs::rename(&temp_path, dest_path).await?;
        
        // Set permissions and ownership
        self.set_file_attributes(dest_path, args).await?;
        
        Ok(GetUrlResult {
            changed: true,
            dest: dest_path.to_string_lossy().to_string(),
            size: downloaded,
            checksum: self.calculate_checksum(dest_path).await?,
        })
    }
}
```

## Testing Strategy

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_tar_extraction() {
        let temp_dir = TempDir::new().unwrap();
        let handler = TarHandler;
        
        // Create test tar file
        let tar_path = create_test_tar().await;
        
        let args = UnarchiveArgs {
            src: tar_path.to_string_lossy().to_string(),
            dest: temp_dir.path().to_string_lossy().to_string(),
            ..Default::default()
        };
        
        let result = handler.extract(&tar_path, temp_dir.path(), &args).await.unwrap();
        assert!(!result.extracted_files.is_empty());
    }
    
    #[tokio::test]
    async fn test_git_clone() {
        let temp_dir = TempDir::new().unwrap();
        let git_ops = GitOperations::new();
        
        let args = GitArgs {
            repo: "https://github.com/git/git.git".to_string(),
            dest: temp_dir.path().join("git").to_string_lossy().to_string(),
            depth: Some(1),
            ..Default::default()
        };
        
        let result = git_ops.clone_repository(&args).await.unwrap();
        assert!(result.changed);
    }
    
    #[tokio::test]
    async fn test_http_request() {
        let client = HttpClientWrapper::new(&UriArgs {
            url: "https://httpbin.org/get".to_string(),
            ..Default::default()
        }).unwrap();
        
        let result = client.execute_request(&UriArgs {
            url: "https://httpbin.org/get".to_string(),
            ..Default::default()
        }).await.unwrap();
        
        assert_eq!(result.status, 200);
        assert!(result.content.is_some());
    }
}
```

### Integration Tests
```rust
// tests/modules/archive_integration_tests.rs
#[tokio::test]
async fn test_archive_roundtrip() {
    // Test creating and extracting archives
}

// tests/modules/git_integration_tests.rs
#[tokio::test]
async fn test_git_workflow() {
    // Test clone, checkout, pull operations
}
```

## Edge Cases & Error Handling

### Archive Operations
- Handle corrupted archives gracefully
- Manage disk space during extraction
- Handle path traversal security issues
- Deal with file permission conflicts

### Git Operations
- Handle authentication failures
- Manage network connectivity issues
- Handle repository conflicts and merge issues
- Deal with submodule complexities

### Network Operations
- Handle network timeouts and retries
- Manage certificate validation issues
- Handle authentication challenges
- Deal with proxy configurations

## Dependencies

### External Crates
- `tar = "0.4"` (already available) - Tar archive handling
- `flate2 = "1"` (already available) - Gzip compression
- `git2 = "0.18"` (already available) - Git operations
- `reqwest = "0.12"` (already available) - HTTP client
- `sha2 = "0.10"` (already available) - Checksum validation
- `zip = "0.6"` - Zip archive handling (new dependency)
- `xz2 = "0.1"` - XZ compression (new dependency)
- `bzip2 = "0.4"` - Bzip2 compression (new dependency)

### Internal Dependencies
- `crate::modules::files` - File operations utilities
- `crate::modules::interface` - Module interface
- `crate::types::platform` - Platform detection

## Configuration

### Module Configuration
```rust
pub struct ArchiveConfig {
    pub max_archive_size: Option<u64>,     // Maximum archive size to process
    pub extraction_timeout: Duration,      // Timeout for extraction operations
    pub compression_level: u8,             // Default compression level
}

pub struct GitConfig {
    pub default_timeout: Duration,         // Default Git operation timeout
    pub max_clone_depth: Option<u32>,      // Maximum shallow clone depth
    pub credential_cache_ttl: Duration,    // Credential cache duration
}

pub struct NetworkConfig {
    pub default_timeout: Duration,         // Default request timeout
    pub max_redirects: u32,                // Maximum redirect follow count
    pub user_agent: String,                // Default User-Agent header
    pub max_download_size: Option<u64>,    // Maximum download size
}
```

This specification provides comprehensive archive, version control, and network operation capabilities essential for modern deployment automation workflows.