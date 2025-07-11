use crate::deploy::Result;
use crate::types::CompiledBinary;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, info, warn};

#[derive(Clone)]
pub struct CompilationCache {
    cache_dir: PathBuf,
    memory_cache: std::sync::Arc<tokio::sync::RwLock<HashMap<String, CacheEntry>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    checksum: String,
    target_triple: String,
    binary_path: PathBuf,
    metadata_path: PathBuf,
    created_at: chrono::DateTime<chrono::Utc>,
    size: u64,
    compilation_time: std::time::Duration,
}

#[derive(Debug, Serialize, Deserialize)]
struct CacheMetadata {
    compilation_id: String,
    target_triple: String,
    checksum: String,
    size: u64,
    created_at: chrono::DateTime<chrono::Utc>,
    compilation_time: std::time::Duration,
    version: String,
}

impl CompilationCache {
    pub fn new(cache_dir: PathBuf) -> Self {
        Self {
            cache_dir,
            memory_cache: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing compilation cache at {:?}", self.cache_dir);

        // Create cache directory structure
        fs::create_dir_all(&self.cache_dir).await?;
        fs::create_dir_all(self.cache_dir.join("binaries")).await?;
        fs::create_dir_all(self.cache_dir.join("metadata")).await?;

        // Load existing cache entries
        self.load_cache_index().await?;

        // Clean up stale entries
        self.cleanup_stale_entries().await?;

        debug!("Cache initialization completed");
        Ok(())
    }

    pub fn get_cached_binary(&self, checksum: &str) -> Option<CompiledBinary> {
        // For now, use a simple synchronous check
        // In a real implementation, this would be async and check the filesystem
        let cache = self.memory_cache.try_read().ok()?;
        let entry = cache.get(checksum)?;

        // Check if the cached file still exists
        if !entry.binary_path.exists() {
            return None;
        }

        // Read the cached binary
        let binary_data = std::fs::read(&entry.binary_path).ok()?;

        Some(CompiledBinary {
            compilation_id: entry.checksum.clone(),
            target_triple: entry.target_triple.clone(),
            binary_data,
            checksum: entry.checksum.clone(),
            size: entry.size,
            compilation_time: entry.compilation_time,
        })
    }

    pub fn store_binary(&self, checksum: &str, binary: &CompiledBinary) -> Result<()> {
        let binary_filename = format!("{checksum}.bin");
        let metadata_filename = format!("{checksum}.json");

        let binary_path = self.cache_dir.join("binaries").join(&binary_filename);
        let metadata_path = self.cache_dir.join("metadata").join(&metadata_filename);

        // Write binary data
        std::fs::write(&binary_path, &binary.binary_data)?;

        // Write metadata
        let metadata = CacheMetadata {
            compilation_id: binary.compilation_id.clone(),
            target_triple: binary.target_triple.clone(),
            checksum: binary.checksum.clone(),
            size: binary.size,
            created_at: chrono::Utc::now(),
            compilation_time: binary.compilation_time,
            version: env!("CARGO_PKG_VERSION").to_string(),
        };

        let metadata_json = serde_json::to_string_pretty(&metadata)?;
        std::fs::write(&metadata_path, metadata_json)?;

        // Update memory cache
        let entry = CacheEntry {
            checksum: checksum.to_string(),
            target_triple: binary.target_triple.clone(),
            binary_path,
            metadata_path,
            created_at: chrono::Utc::now(),
            size: binary.size,
            compilation_time: binary.compilation_time,
        };

        if let Ok(mut cache) = self.memory_cache.try_write() {
            cache.insert(checksum.to_string(), entry);
        }

        info!(
            "Stored binary in cache: {} ({} bytes)",
            checksum, binary.size
        );
        Ok(())
    }

    pub async fn invalidate_entry(&self, checksum: &str) -> Result<()> {
        info!("Invalidating cache entry: {}", checksum);

        // Remove from memory cache
        {
            let mut cache = self.memory_cache.write().await;
            if let Some(entry) = cache.remove(checksum) {
                // Remove files
                let _ = fs::remove_file(&entry.binary_path).await;
                let _ = fs::remove_file(&entry.metadata_path).await;
            }
        }

        Ok(())
    }

    pub async fn clear_cache(&self) -> Result<()> {
        info!("Clearing entire compilation cache");

        // Clear memory cache
        {
            let mut cache = self.memory_cache.write().await;
            cache.clear();
        }

        // Remove cache directories
        if self.cache_dir.exists() {
            fs::remove_dir_all(&self.cache_dir).await?;
            fs::create_dir_all(&self.cache_dir).await?;
            fs::create_dir_all(self.cache_dir.join("binaries")).await?;
            fs::create_dir_all(self.cache_dir.join("metadata")).await?;
        }

        info!("Cache cleared successfully");
        Ok(())
    }

    pub async fn get_cache_stats(&self) -> CacheStats {
        let cache = self.memory_cache.read().await;
        let entry_count = cache.len();
        let total_size: u64 = cache.values().map(|entry| entry.size).sum();

        CacheStats {
            entry_count,
            total_size_bytes: total_size,
            cache_dir: self.cache_dir.clone(),
        }
    }

    pub async fn list_cache_entries(&self) -> Vec<CacheEntryInfo> {
        let cache = self.memory_cache.read().await;
        cache
            .values()
            .map(|entry| CacheEntryInfo {
                checksum: entry.checksum.clone(),
                target_triple: entry.target_triple.clone(),
                size: entry.size,
                created_at: entry.created_at,
                compilation_time: entry.compilation_time,
            })
            .collect()
    }

    // Private helper methods

    async fn load_cache_index(&self) -> Result<()> {
        let metadata_dir = self.cache_dir.join("metadata");

        if !metadata_dir.exists() {
            return Ok(());
        }

        let mut entries = fs::read_dir(&metadata_dir).await?;
        let mut loaded_count = 0;

        while let Some(entry) = entries.next_entry().await? {
            if let Some(extension) = entry.path().extension() {
                if extension == "json" {
                    if let Ok(metadata) = self.load_metadata_file(&entry.path()).await {
                        if let Some(cache_entry) =
                            self.metadata_to_cache_entry(metadata, &entry.path())
                        {
                            {
                                let mut cache = self.memory_cache.write().await;
                                cache.insert(cache_entry.checksum.clone(), cache_entry);
                                loaded_count += 1;
                            }
                        }
                    }
                }
            }
        }

        debug!("Loaded {} cache entries from disk", loaded_count);
        Ok(())
    }

    async fn load_metadata_file(&self, path: &Path) -> Result<CacheMetadata> {
        let content = fs::read_to_string(path).await?;
        let metadata: CacheMetadata = serde_json::from_str(&content)?;
        Ok(metadata)
    }

    fn metadata_to_cache_entry(
        &self,
        metadata: CacheMetadata,
        metadata_path: &Path,
    ) -> Option<CacheEntry> {
        let checksum = metadata.checksum.clone();
        let binary_filename = format!("{checksum}.bin");
        let binary_path = self.cache_dir.join("binaries").join(&binary_filename);

        // Check if binary file exists
        if !binary_path.exists() {
            warn!("Binary file missing for cache entry: {}", checksum);
            return None;
        }

        Some(CacheEntry {
            checksum,
            target_triple: metadata.target_triple,
            binary_path,
            metadata_path: metadata_path.to_path_buf(),
            created_at: metadata.created_at,
            size: metadata.size,
            compilation_time: metadata.compilation_time,
        })
    }

    async fn cleanup_stale_entries(&self) -> Result<()> {
        let max_age = chrono::Duration::days(7); // Remove entries older than 7 days
        let cutoff_time = chrono::Utc::now() - max_age;

        let mut stale_checksums = Vec::new();

        {
            let cache = self.memory_cache.read().await;
            for (checksum, entry) in cache.iter() {
                if entry.created_at < cutoff_time {
                    stale_checksums.push(checksum.clone());
                }
            }
        }

        for checksum in stale_checksums {
            self.invalidate_entry(&checksum).await?;
        }

        debug!("Cleaned up stale cache entries");
        Ok(())
    }
}

#[derive(Debug)]
pub struct CacheStats {
    pub entry_count: usize,
    pub total_size_bytes: u64,
    pub cache_dir: PathBuf,
}

#[derive(Debug)]
pub struct CacheEntryInfo {
    pub checksum: String,
    pub target_triple: String,
    pub size: u64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub compilation_time: std::time::Duration,
}

impl CacheStats {
    pub fn total_size_mb(&self) -> f64 {
        self.total_size_bytes as f64 / (1024.0 * 1024.0)
    }

    pub fn average_compilation_time(&self) -> Option<std::time::Duration> {
        if self.entry_count == 0 {
            None
        } else {
            // This would require tracking compilation times, simplified for now
            Some(std::time::Duration::from_secs(60))
        }
    }
}
