use crate::execution::plan::ModuleSource;
use crate::modules::error::ResolveError;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info};
use walkdir::WalkDir;

/// Trait for resolving different module sources
#[async_trait]
pub trait ModuleSourceResolver: Send + Sync {
    fn can_resolve(&self, source: &ModuleSource) -> bool;

    async fn resolve(&self, source: &ModuleSource) -> Result<ModuleSourceCode, ResolveError>;

    fn cache_key(&self, source: &ModuleSource) -> String;
}

#[derive(Debug, Clone)]
pub struct ModuleSourceCode {
    pub main_file: String,
    pub additional_files: HashMap<String, String>,
    pub cargo_toml: Option<String>,
}

/// File system module resolver
pub struct FileSystemResolver {
    base_paths: Vec<PathBuf>,
}

impl FileSystemResolver {
    pub fn new(base_paths: Vec<PathBuf>) -> Self {
        let mut paths = base_paths;
        // Add default module search paths
        if let Ok(home) = std::env::var("HOME") {
            paths.push(PathBuf::from(home).join(".rustle").join("modules"));
        }
        paths.push(PathBuf::from("/usr/local/share/rustle/modules"));
        paths.push(PathBuf::from("./modules"));

        Self { base_paths: paths }
    }

    fn resolve_path(&self, path: &str) -> Result<PathBuf, ResolveError> {
        let path = Path::new(path);

        // If absolute path, use it directly
        if path.is_absolute() {
            if path.exists() {
                return Ok(path.to_path_buf());
            } else {
                return Err(ResolveError::FileNotFound {
                    path: path.display().to_string(),
                });
            }
        }

        // Try each base path
        for base in &self.base_paths {
            let full_path = base.join(path);
            if full_path.exists() {
                return Ok(full_path);
            }
        }

        Err(ResolveError::FileNotFound {
            path: path.display().to_string(),
        })
    }

    async fn scan_directory(&self, dir: &Path) -> Result<HashMap<String, String>, ResolveError> {
        let mut files = HashMap::new();

        for entry in WalkDir::new(dir).max_depth(3) {
            let entry = entry.map_err(|e| ResolveError::IoError {
                operation: "scan directory".to_string(),
                error: e.to_string(),
            })?;

            if entry.file_type().is_file() {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    if ext == "rs" || ext == "toml" {
                        let relative = path.strip_prefix(dir).unwrap();
                        let content = tokio::fs::read_to_string(path).await.map_err(|e| {
                            ResolveError::IoError {
                                operation: "read file".to_string(),
                                error: e.to_string(),
                            }
                        })?;
                        files.insert(relative.display().to_string(), content);
                    }
                }
            }
        }

        Ok(files)
    }

    async fn load_cargo_toml(&self, dir: &Path) -> Result<Option<String>, ResolveError> {
        let cargo_path = dir.join("Cargo.toml");
        if cargo_path.exists() {
            let content = tokio::fs::read_to_string(&cargo_path).await.map_err(|e| {
                ResolveError::IoError {
                    operation: "read Cargo.toml".to_string(),
                    error: e.to_string(),
                }
            })?;
            Ok(Some(content))
        } else {
            Ok(None)
        }
    }
}

#[async_trait]
impl ModuleSourceResolver for FileSystemResolver {
    fn can_resolve(&self, source: &ModuleSource) -> bool {
        matches!(source, ModuleSource::File { .. })
    }

    async fn resolve(&self, source: &ModuleSource) -> Result<ModuleSourceCode, ResolveError> {
        if let ModuleSource::File { path } = source {
            debug!("Resolving file module from path: {}", path);
            let full_path = self.resolve_path(path)?;

            // Check if it's a directory or a file
            if full_path.is_dir() {
                // Look for main module file
                let main_candidates = ["mod.rs", "lib.rs", "main.rs"];
                let mut main_file = None;

                for candidate in &main_candidates {
                    let candidate_path = full_path.join(candidate);
                    if candidate_path.exists() {
                        main_file = Some(candidate_path);
                        break;
                    }
                }

                let main_path = main_file.ok_or_else(|| ResolveError::InvalidModule {
                    reason: "No main module file found (mod.rs, lib.rs, or main.rs)".to_string(),
                })?;

                let source_code = tokio::fs::read_to_string(&main_path).await.map_err(|e| {
                    ResolveError::IoError {
                        operation: "read main file".to_string(),
                        error: e.to_string(),
                    }
                })?;

                let additional_files = self.scan_directory(&full_path).await?;
                let cargo_toml = self.load_cargo_toml(&full_path).await?;

                Ok(ModuleSourceCode {
                    main_file: source_code,
                    additional_files,
                    cargo_toml,
                })
            } else {
                // Single file module
                let source_code = tokio::fs::read_to_string(&full_path).await.map_err(|e| {
                    ResolveError::IoError {
                        operation: "read module file".to_string(),
                        error: e.to_string(),
                    }
                })?;

                // Look for additional files in the same directory
                let directory = full_path.parent().unwrap();
                let additional_files = self.scan_directory(directory).await?;
                let cargo_toml = self.load_cargo_toml(directory).await?;

                Ok(ModuleSourceCode {
                    main_file: source_code,
                    additional_files,
                    cargo_toml,
                })
            }
        } else {
            Err(ResolveError::IncompatibleSource)
        }
    }

    fn cache_key(&self, source: &ModuleSource) -> String {
        match source {
            ModuleSource::File { path } => format!("file:{}", path),
            _ => unreachable!(),
        }
    }
}

/// Git repository module resolver
pub struct GitResolver {
    cache_dir: PathBuf,
}

impl GitResolver {
    pub fn new() -> Self {
        let cache_dir = if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home)
                .join(".rustle")
                .join("cache")
                .join("git")
        } else {
            PathBuf::from("/tmp/rustle/cache/git")
        };

        Self { cache_dir }
    }

    async fn clone_or_update_repo(
        &self,
        repository: &str,
        reference: &str,
        cache_path: &Path,
    ) -> Result<(), ResolveError> {
        use tokio::process::Command;

        if !cache_path.exists() {
            info!("Cloning repository {} to {:?}", repository, cache_path);
            tokio::fs::create_dir_all(cache_path.parent().unwrap())
                .await
                .map_err(|e| ResolveError::IoError {
                    operation: "create cache directory".to_string(),
                    error: e.to_string(),
                })?;

            let output = Command::new("git")
                .args(&["clone", repository, &cache_path.display().to_string()])
                .output()
                .await
                .map_err(|e| ResolveError::GitError {
                    operation: "clone".to_string(),
                    error: e.to_string(),
                })?;

            if !output.status.success() {
                return Err(ResolveError::GitError {
                    operation: "clone".to_string(),
                    error: String::from_utf8_lossy(&output.stderr).to_string(),
                });
            }
        }

        // Checkout the specified reference
        let output = Command::new("git")
            .current_dir(cache_path)
            .args(&["checkout", reference])
            .output()
            .await
            .map_err(|e| ResolveError::GitError {
                operation: "checkout".to_string(),
                error: e.to_string(),
            })?;

        if !output.status.success() {
            return Err(ResolveError::GitError {
                operation: "checkout".to_string(),
                error: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        Ok(())
    }

    async fn load_from_directory(&self, dir: &Path) -> Result<ModuleSourceCode, ResolveError> {
        // Similar to FileSystemResolver's directory loading
        let main_candidates = ["mod.rs", "lib.rs", "main.rs"];
        let mut main_file = None;

        for candidate in &main_candidates {
            let candidate_path = dir.join(candidate);
            if candidate_path.exists() {
                main_file = Some(candidate_path);
                break;
            }
        }

        let main_path = main_file.ok_or_else(|| ResolveError::InvalidModule {
            reason: "No main module file found in repository".to_string(),
        })?;

        let source_code =
            tokio::fs::read_to_string(&main_path)
                .await
                .map_err(|e| ResolveError::IoError {
                    operation: "read main file".to_string(),
                    error: e.to_string(),
                })?;

        let mut additional_files = HashMap::new();
        for entry in WalkDir::new(dir).max_depth(3) {
            let entry = entry.map_err(|e| ResolveError::IoError {
                operation: "scan directory".to_string(),
                error: e.to_string(),
            })?;

            if entry.file_type().is_file() {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    if ext == "rs" && path != main_path {
                        let relative = path.strip_prefix(dir).unwrap();
                        let content = tokio::fs::read_to_string(path).await.map_err(|e| {
                            ResolveError::IoError {
                                operation: "read file".to_string(),
                                error: e.to_string(),
                            }
                        })?;
                        additional_files.insert(relative.display().to_string(), content);
                    }
                }
            }
        }

        let cargo_toml = {
            let cargo_path = dir.join("Cargo.toml");
            if cargo_path.exists() {
                Some(tokio::fs::read_to_string(&cargo_path).await.map_err(|e| {
                    ResolveError::IoError {
                        operation: "read Cargo.toml".to_string(),
                        error: e.to_string(),
                    }
                })?)
            } else {
                None
            }
        };

        Ok(ModuleSourceCode {
            main_file: source_code,
            additional_files,
            cargo_toml,
        })
    }
}

#[async_trait]
impl ModuleSourceResolver for GitResolver {
    fn can_resolve(&self, source: &ModuleSource) -> bool {
        matches!(source, ModuleSource::Git { .. })
    }

    async fn resolve(&self, source: &ModuleSource) -> Result<ModuleSourceCode, ResolveError> {
        if let ModuleSource::Git {
            repository,
            reference,
        } = source
        {
            let cache_key = self.cache_key(source);
            let cache_path = self.cache_dir.join(&cache_key);

            debug!("Resolving git module from repository: {}", repository);

            // Clone or update repository
            self.clone_or_update_repo(repository, reference, &cache_path)
                .await?;

            // Read module files from the root of the repository
            let module_path = cache_path;

            self.load_from_directory(&module_path).await
        } else {
            Err(ResolveError::IncompatibleSource)
        }
    }

    fn cache_key(&self, source: &ModuleSource) -> String {
        match source {
            ModuleSource::Git {
                repository,
                reference,
                ..
            } => {
                // Create a safe directory name from the repository URL
                let safe_repo = repository
                    .replace("https://", "")
                    .replace("http://", "")
                    .replace("/", "_")
                    .replace(".", "_");
                format!("{}_{}", safe_repo, reference)
            }
            _ => unreachable!(),
        }
    }
}

/// HTTP module resolver
pub struct HttpResolver {
    client: reqwest::Client,
    cache_dir: PathBuf,
}

impl HttpResolver {
    pub fn new() -> Self {
        let cache_dir = if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home)
                .join(".rustle")
                .join("cache")
                .join("http")
        } else {
            PathBuf::from("/tmp/rustle/cache/http")
        };

        Self {
            client: reqwest::Client::new(),
            cache_dir,
        }
    }
}

#[async_trait]
impl ModuleSourceResolver for HttpResolver {
    fn can_resolve(&self, source: &ModuleSource) -> bool {
        matches!(source, ModuleSource::Http { .. })
    }

    async fn resolve(&self, source: &ModuleSource) -> Result<ModuleSourceCode, ResolveError> {
        if let ModuleSource::Http { url } = source {
            debug!("Resolving HTTP module from URL: {}", url);

            let request = self.client.get(url);

            let response = request.send().await.map_err(|e| ResolveError::HttpError {
                url: url.clone(),
                error: e.to_string(),
            })?;

            if !response.status().is_success() {
                return Err(ResolveError::HttpError {
                    url: url.clone(),
                    error: format!(
                        "HTTP {} {}",
                        response.status().as_u16(),
                        response.status().as_str()
                    ),
                });
            }

            let content = response.text().await.map_err(|e| ResolveError::HttpError {
                url: url.clone(),
                error: e.to_string(),
            })?;

            // Cache the downloaded content
            let cache_key = self.cache_key(source);
            let cache_path = self.cache_dir.join(&cache_key);
            tokio::fs::create_dir_all(cache_path.parent().unwrap())
                .await
                .map_err(|e| ResolveError::IoError {
                    operation: "create cache directory".to_string(),
                    error: e.to_string(),
                })?;

            tokio::fs::write(&cache_path, &content)
                .await
                .map_err(|e| ResolveError::IoError {
                    operation: "write cache file".to_string(),
                    error: e.to_string(),
                })?;

            Ok(ModuleSourceCode {
                main_file: content,
                additional_files: HashMap::new(),
                cargo_toml: None,
            })
        } else {
            Err(ResolveError::IncompatibleSource)
        }
    }

    fn cache_key(&self, source: &ModuleSource) -> String {
        match source {
            ModuleSource::Http { url, .. } => {
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(url.as_bytes());
                format!("http_{:x}", hasher.finalize())
            }
            _ => unreachable!(),
        }
    }
}

/// Registry module resolver (for module registries)
pub struct RegistryResolver {
    client: reqwest::Client,
    registry_configs: HashMap<String, RegistryConfig>,
}

#[derive(Debug, Clone)]
pub struct RegistryConfig {
    pub base_url: String,
    pub auth_token: Option<String>,
    pub verify_signatures: bool,
}

impl RegistryResolver {
    pub fn new() -> Self {
        let mut registry_configs = HashMap::new();

        // Add default registry
        registry_configs.insert(
            "default".to_string(),
            RegistryConfig {
                base_url: "https://modules.rustle.dev".to_string(),
                auth_token: None,
                verify_signatures: true,
            },
        );

        Self {
            client: reqwest::Client::new(),
            registry_configs,
        }
    }
}

#[async_trait]
impl ModuleSourceResolver for RegistryResolver {
    fn can_resolve(&self, source: &ModuleSource) -> bool {
        matches!(source, ModuleSource::Registry { .. })
    }

    async fn resolve(&self, source: &ModuleSource) -> Result<ModuleSourceCode, ResolveError> {
        if let ModuleSource::Registry { name, version } = source {
            let registry_name = "default"; // Use default registry for now
            let config = self.registry_configs.get(registry_name).ok_or_else(|| {
                ResolveError::UnknownRegistry {
                    name: registry_name.to_string(),
                }
            })?;

            debug!(
                "Resolving module {} version {} from registry {}",
                name, version, registry_name
            );

            // Construct API URL
            let api_url = format!(
                "{}/api/v1/modules/{}/{}/download",
                config.base_url, name, version
            );

            let mut request = self.client.get(&api_url);
            if let Some(token) = &config.auth_token {
                request = request.bearer_auth(token);
            }

            let response = request
                .send()
                .await
                .map_err(|e| ResolveError::RegistryError {
                    registry: registry_name.to_string(),
                    error: e.to_string(),
                })?;

            if !response.status().is_success() {
                return Err(ResolveError::RegistryError {
                    registry: registry_name.to_string(),
                    error: format!(
                        "HTTP {} {}",
                        response.status().as_u16(),
                        response.status().as_str()
                    ),
                });
            }

            let content = response
                .text()
                .await
                .map_err(|e| ResolveError::RegistryError {
                    registry: registry_name.to_string(),
                    error: e.to_string(),
                })?;

            // TODO: Verify signatures if configured

            Ok(ModuleSourceCode {
                main_file: content,
                additional_files: HashMap::new(),
                cargo_toml: None,
            })
        } else {
            Err(ResolveError::IncompatibleSource)
        }
    }

    fn cache_key(&self, source: &ModuleSource) -> String {
        match source {
            ModuleSource::Registry { name, version } => {
                let registry = "default";
                format!("registry_{}_{}_{}_{}", registry, name, version, "")
            }
            _ => unreachable!(),
        }
    }
}
