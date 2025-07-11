use crate::compilation::zigbuild::CompiledBinary;
use crate::deploy::{DeployError, Result};
use crate::template::GeneratedTemplate;
use crate::compilation::toolchain::TargetSpecification;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use sha2::{Sha256, Digest};
use serde::{Deserialize, Serialize};
use tokio::fs;
use tracing::{debug, info, warn};

/// Compilation cache for storing and reusing compiled binaries
pub struct CompilationCache {
    cache_dir: PathBuf,
    cache_index: CacheIndex,
    max_cache_size_mb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheIndex {
    entries: HashMap<String, CacheEntry>,
    total_size_bytes: u64,
    last_cleanup: std::time::SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    hash: String,
    target: String,
    binary_path: PathBuf,
    size_bytes: u64,
    created_at: std::time::SystemTime,
    last_accessed: std::time::SystemTime,
    compilation_time: std::time::Duration,
    metadata: BinaryMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BinaryMetadata {
    template_hash: String,
    target_triple: String,
    optimization_level: String,
    features: Vec<String>,
    rust_version: String,
    zig_version: Option<String>,
}

impl Default for CacheIndex {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            total_size_bytes: 0,
            last_cleanup: std::time::SystemTime::now(),
        }
    }
}

impl CompilationCache {
    /// Create new compilation cache
    pub fn new(cache_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&cache_dir)
            .map_err(|e| DeployError::Configuration(format!("Failed to create cache directory: {}", e)))?;

        let cache_index_path = cache_dir.join("index.json");
        let cache_index = if cache_index_path.exists() {
            let index_data = std::fs::read_to_string(&cache_index_path)
                .map_err(|e| DeployError::Configuration(format!("Failed to read cache index: {}", e)))?;
            serde_json::from_str(&index_data)
                .unwrap_or_else(|e| {
                    warn!("Failed to parse cache index: {}, creating new index", e);
                    CacheIndex::default()
                })
        } else {
            CacheIndex::default()
        };

        Ok(Self {
            cache_dir,
            cache_index,
            max_cache_size_mb: 1024, // 1GB default
        })
    }

    /// Get cached binary if available and valid
    pub fn get_cached_binary(&mut self, template: &GeneratedTemplate, target: &str) -> Option<CompiledBinary> {
        let template_hash = self.compute_template_hash(template);
        let cache_key = format!("{}:{}", template_hash, target);

        if let Some(entry) = self.cache_index.entries.get_mut(&cache_key) {
            // Check if binary file still exists
            if entry.binary_path.exists() {
                // Update last accessed time
                entry.last_accessed = std::time::SystemTime::now();
                
                debug!("Cache hit for template {} target {}", template_hash, target);
                
                return Some(CompiledBinary {
                    target_triple: entry.target.clone(),
                    binary_path: entry.binary_path.clone(),
                    size_bytes: entry.size_bytes,
                    compilation_time: entry.compilation_time,
                    optimization_level: crate::compilation::zigbuild::OptimizationLevel::Release, // Simplified
                    features: entry.metadata.features.clone(),
                });
            } else {
                // Binary file is missing, remove from cache
                warn!("Cached binary missing: {}", entry.binary_path.display());
                self.cache_index.entries.remove(&cache_key);
            }
        }

        debug!("Cache miss for template {} target {}", template_hash, target);
        None
    }

    /// Cache a compiled binary
    pub fn cache_binary(
        &mut self,
        template: &GeneratedTemplate,
        target: &str,
        binary: &CompiledBinary,
    ) -> Result<()> {
        let template_hash = self.compute_template_hash(template);
        let cache_key = format!("{}:{}", template_hash, target);

        // Create cache entry directory
        let entry_dir = self.cache_dir.join(&cache_key);
        std::fs::create_dir_all(&entry_dir)
            .map_err(|e| DeployError::Configuration(format!("Failed to create cache entry directory: {}", e)))?;

        // Copy binary to cache
        let cached_binary_path = entry_dir.join("binary");
        std::fs::copy(&binary.binary_path, &cached_binary_path)
            .map_err(|e| DeployError::Configuration(format!("Failed to cache binary: {}", e)))?;

        // Create cache entry
        let entry = CacheEntry {
            hash: cache_key.clone(),
            target: target.to_string(),
            binary_path: cached_binary_path,
            size_bytes: binary.size_bytes,
            created_at: std::time::SystemTime::now(),
            last_accessed: std::time::SystemTime::now(),
            compilation_time: binary.compilation_time,
            metadata: BinaryMetadata {
                template_hash: template_hash.clone(),
                target_triple: target.to_string(),
                optimization_level: format!("{:?}", binary.optimization_level),
                features: binary.features.clone(),
                rust_version: "unknown".to_string(), // TODO: Get actual version
                zig_version: None, // TODO: Get actual version if used
            },
        };

        // Add to cache index
        self.cache_index.entries.insert(cache_key, entry);
        self.cache_index.total_size_bytes += binary.size_bytes;

        // Save cache index
        self.save_cache_index()?;

        // Perform cleanup if needed
        if self.cache_index.total_size_bytes > self.max_cache_size_mb * 1024 * 1024 {
            self.cleanup_cache()?;
        }

        info!("Cached binary for template {} target {} ({} bytes)", 
              template_hash, target, binary.size_bytes);

        Ok(())
    }

    /// Prepare template directory for compilation
    pub async fn prepare_template_for_target(
        &self,
        template: &GeneratedTemplate,
        target: &TargetSpecification,
    ) -> Result<PathBuf> {
        let template_hash = self.compute_template_hash(template);
        let template_dir = self.cache_dir.join("templates").join(&template_hash);

        // Create template directory if it doesn't exist
        fs::create_dir_all(&template_dir).await
            .map_err(|e| DeployError::Configuration(format!("Failed to create template directory: {}", e)))?;

        // Write template files
        self.write_template_files(&template_dir, template, target).await?;

        Ok(template_dir)
    }

    /// Get cache statistics
    pub fn get_cache_stats(&self) -> CacheStats {
        let mut stats = CacheStats {
            total_entries: self.cache_index.entries.len(),
            total_size_bytes: self.cache_index.total_size_bytes,
            targets: HashMap::new(),
            oldest_entry: None,
            newest_entry: None,
        };

        for entry in self.cache_index.entries.values() {
            // Count by target
            *stats.targets.entry(entry.target.clone()).or_insert(0) += 1;

            // Track oldest/newest
            if stats.oldest_entry.is_none() || entry.created_at < stats.oldest_entry.unwrap() {
                stats.oldest_entry = Some(entry.created_at);
            }
            if stats.newest_entry.is_none() || entry.created_at > stats.newest_entry.unwrap() {
                stats.newest_entry = Some(entry.created_at);
            }
        }

        stats
    }

    /// Clear all cache entries
    pub fn clear_cache(&mut self) -> Result<()> {
        info!("Clearing compilation cache");

        // Remove all cached files
        for entry in self.cache_index.entries.values() {
            if let Some(parent) = entry.binary_path.parent() {
                let _ = std::fs::remove_dir_all(parent);
            }
        }

        // Reset cache index
        self.cache_index = CacheIndex::default();
        self.save_cache_index()?;

        Ok(())
    }

    // Private helper methods

    fn compute_template_hash(&self, template: &GeneratedTemplate) -> String {
        let mut hasher = Sha256::new();
        
        // Hash the execution plan content
        if let Ok(plan_json) = serde_json::to_string(&template.execution_plan) {
            hasher.update(plan_json.as_bytes());
        }
        
        // Hash template files content
        for (path, content) in &template.files {
            hasher.update(path.as_bytes());
            hasher.update(content.as_bytes());
        }

        format!("{:x}", hasher.finalize())[..16].to_string()
    }

    async fn write_template_files(
        &self,
        template_dir: &Path,
        template: &GeneratedTemplate,
        _target: &TargetSpecification,
    ) -> Result<()> {
        // Write Cargo.toml
        let cargo_toml_path = template_dir.join("Cargo.toml");
        if !cargo_toml_path.exists() {
            let cargo_toml_content = self.generate_cargo_toml(template);
            fs::write(&cargo_toml_path, cargo_toml_content).await
                .map_err(|e| DeployError::Configuration(format!("Failed to write Cargo.toml: {}", e)))?;
        }

        // Write source files
        let src_dir = template_dir.join("src");
        fs::create_dir_all(&src_dir).await
            .map_err(|e| DeployError::Configuration(format!("Failed to create src directory: {}", e)))?;

        for (relative_path, content) in &template.files {
            let file_path = src_dir.join(relative_path);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).await
                    .map_err(|e| DeployError::Configuration(format!("Failed to create directory: {}", e)))?;
            }
            fs::write(&file_path, content).await
                .map_err(|e| DeployError::Configuration(format!("Failed to write template file: {}", e)))?;
        }

        Ok(())
    }

    fn generate_cargo_toml(&self, _template: &GeneratedTemplate) -> String {
        // Generate a basic Cargo.toml for the template
        r#"[package]
name = "rustle-executor"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "rustle-executor"
path = "src/main.rs"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
anyhow = "1"
tracing = "0.1"
"#.to_string()
    }

    fn save_cache_index(&self) -> Result<()> {
        let index_path = self.cache_dir.join("index.json");
        let index_json = serde_json::to_string_pretty(&self.cache_index)
            .map_err(|e| DeployError::Configuration(format!("Failed to serialize cache index: {}", e)))?;
        
        std::fs::write(&index_path, index_json)
            .map_err(|e| DeployError::Configuration(format!("Failed to write cache index: {}", e)))?;

        Ok(())
    }

    fn cleanup_cache(&mut self) -> Result<()> {
        info!("Performing cache cleanup");

        // Sort entries by last accessed time (oldest first)
        let mut entries: Vec<_> = self.cache_index.entries.iter().collect();
        entries.sort_by_key(|(_, entry)| entry.last_accessed);

        // Remove oldest entries until we're under the size limit
        let target_size = (self.max_cache_size_mb * 1024 * 1024) / 2; // Clean to 50% of limit
        let mut removed_size = 0u64;

        let mut keys_to_remove = Vec::new();

        for (key, entry) in entries.iter() {
            if self.cache_index.total_size_bytes - removed_size <= target_size {
                break;
            }

            // Remove the cached binary file
            if let Some(parent) = entry.binary_path.parent() {
                if let Err(e) = std::fs::remove_dir_all(parent) {
                    warn!("Failed to remove cache entry directory: {}", e);
                }
            }

            removed_size += entry.size_bytes;
            keys_to_remove.push((*key).clone());
        }

        // Remove entries from index
        for key in keys_to_remove {
            self.cache_index.entries.remove(&key);
        }

        self.cache_index.total_size_bytes -= removed_size;
        self.cache_index.last_cleanup = std::time::SystemTime::now();

        self.save_cache_index()?;

        info!("Cache cleanup completed: removed {} bytes, {} entries remain", 
              removed_size, self.cache_index.entries.len());

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub total_size_bytes: u64,
    pub targets: HashMap<String, usize>,
    pub oldest_entry: Option<std::time::SystemTime>,
    pub newest_entry: Option<std::time::SystemTime>,
}