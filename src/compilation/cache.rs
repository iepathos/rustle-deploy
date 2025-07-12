use crate::compilation::compiler::CompiledBinary;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{debug, info, warn};

/// Simplified compilation cache for storing and reusing compiled binaries
#[derive(Debug, Clone)]
pub struct CompilationCache {
    cache_dir: PathBuf,
    enable_cache: bool,
    cache_index: CacheIndex,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct CacheIndex {
    entries: HashMap<String, CacheEntry>,
    total_size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    cache_key: String,
    target_triple: String,
    binary_path: PathBuf,
    size_bytes: u64,
    checksum: String,
    created_at: std::time::SystemTime,
    template_hash: String,
}

impl CompilationCache {
    pub fn new(cache_dir: PathBuf, enable_cache: bool) -> Self {
        if enable_cache {
            let _ = std::fs::create_dir_all(&cache_dir);
        }

        let cache_index = if enable_cache {
            let index_path = cache_dir.join("index.json");
            if index_path.exists() {
                std::fs::read_to_string(&index_path)
                    .ok()
                    .and_then(|content| serde_json::from_str(&content).ok())
                    .unwrap_or_default()
            } else {
                CacheIndex::default()
            }
        } else {
            CacheIndex::default()
        };

        Self {
            cache_dir,
            enable_cache,
            cache_index,
        }
    }

    pub fn get_binary(&self, template_hash: &str, target: &str) -> Option<CompiledBinary> {
        if !self.enable_cache {
            return None;
        }

        let cache_key = format!("{template_hash}:{target}");

        if let Some(entry) = self.cache_index.entries.get(&cache_key) {
            if entry.binary_path.exists() {
                debug!("Cache hit for {} {}", template_hash, target);

                // Read binary data
                if let Ok(binary_data) = std::fs::read(&entry.binary_path) {
                    return Some(CompiledBinary {
                        binary_id: uuid::Uuid::new_v4().to_string(),
                        target_triple: entry.target_triple.clone(),
                        binary_path: entry.binary_path.clone(),
                        binary_data,
                        effective_source: crate::compilation::compiler::BinarySource::Cache {
                            cache_path: entry.binary_path.clone(),
                        },
                        size: entry.size_bytes,
                        checksum: entry.checksum.clone(),
                        compilation_time: std::time::Duration::from_secs(0), // Cached, so no compilation time
                        optimization_level: crate::types::compilation::OptimizationLevel::Release, // Default for cached
                        template_hash: entry.template_hash.clone(),
                        created_at: chrono::DateTime::from(entry.created_at),
                    });
                }
            } else {
                warn!("Cached binary missing: {}", entry.binary_path.display());
            }
        }

        debug!("Cache miss for {} {}", template_hash, target);
        None
    }

    pub async fn store_binary(&mut self, binary: &CompiledBinary) -> Result<()> {
        if !self.enable_cache {
            return Ok(());
        }

        let cache_key = format!("{}:{}", binary.template_hash, binary.target_triple);

        // Create cache entry directory
        let entry_dir = self.cache_dir.join(&cache_key);
        tokio::fs::create_dir_all(&entry_dir).await?;

        // Copy binary to cache
        let cached_binary_path = entry_dir.join("binary");
        tokio::fs::copy(&binary.binary_path, &cached_binary_path).await?;

        // Create cache entry
        let entry = CacheEntry {
            cache_key: cache_key.clone(),
            target_triple: binary.target_triple.clone(),
            binary_path: cached_binary_path,
            size_bytes: binary.size,
            checksum: binary.checksum.clone(),
            created_at: std::time::SystemTime::now(),
            template_hash: binary.template_hash.clone(),
        };

        // Update cache index
        self.cache_index.entries.insert(cache_key, entry);
        self.cache_index.total_size_bytes += binary.size;

        // Save cache index
        self.save_cache_index().await?;

        info!(
            "Cached binary for {} ({} bytes)",
            binary.target_triple, binary.size
        );
        Ok(())
    }

    pub async fn clear_cache(&mut self) -> Result<()> {
        if !self.enable_cache {
            return Ok(());
        }

        info!("Clearing compilation cache");

        // Remove all cached files
        for entry in self.cache_index.entries.values() {
            if let Some(parent) = entry.binary_path.parent() {
                let _ = tokio::fs::remove_dir_all(parent).await;
            }
        }

        // Reset cache index
        self.cache_index = CacheIndex::default();
        self.save_cache_index().await?;

        Ok(())
    }

    pub fn get_cache_path(&self, template_hash: &str, target: &str) -> PathBuf {
        let cache_key = format!("{template_hash}:{target}");
        self.cache_dir.join(&cache_key).join("binary")
    }

    async fn save_cache_index(&self) -> Result<()> {
        if !self.enable_cache {
            return Ok(());
        }

        let index_path = self.cache_dir.join("index.json");
        let index_json = serde_json::to_string_pretty(&self.cache_index)?;
        tokio::fs::write(&index_path, index_json).await?;
        Ok(())
    }
}
