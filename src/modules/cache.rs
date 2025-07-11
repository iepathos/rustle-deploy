use crate::execution::plan::ModuleSpec;
use crate::modules::error::CacheError;
use crate::modules::loader::LoadedModule;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::fs;
use tracing::{debug, info, warn};

/// Cache for loaded and compiled modules
pub struct ModuleCache {
    cache_dir: PathBuf,
    memory_cache: HashMap<String, CachedModule>,
    ttl: Duration,
    max_cache_size: usize,
    max_memory_entries: usize,
}

#[derive(Debug, Clone)]
struct CachedModule {
    module: LoadedModule,
    cached_at: Instant,
    access_count: usize,
    last_accessed: Instant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheMetadata {
    cache_key: String,
    module_name: String,
    module_version: String,
    cached_at: u64,
    access_count: usize,
    last_accessed: u64,
    checksum: String,
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub memory_entries: usize,
    pub disk_entries: usize,
    pub total_size_bytes: usize,
    pub hit_rate: f64,
    pub access_count: usize,
}

impl ModuleCache {
    pub fn new(cache_dir: PathBuf, ttl: Duration) -> Self {
        Self {
            cache_dir,
            memory_cache: HashMap::new(),
            ttl,
            max_cache_size: 1024 * 1024 * 1024, // 1GB
            max_memory_entries: 100,
        }
    }

    pub async fn get(&mut self, spec: &ModuleSpec) -> Option<LoadedModule> {
        let cache_key = self.cache_key(spec);

        // Check memory cache first
        let cache_hit = if let Some(cached) = self.memory_cache.get(&cache_key) {
            if !self.is_expired(&cached.cached_at) {
                Some(cached.module.clone())
            } else {
                None
            }
        } else {
            None
        };

        if let Some(module) = cache_hit {
            // Update access stats
            if let Some(cached) = self.memory_cache.get_mut(&cache_key) {
                cached.access_count += 1;
                cached.last_accessed = Instant::now();
            }
            debug!("Module cache hit (memory): {}", spec.name);
            return Some(module);
        } else {
            // Remove expired entry
            self.memory_cache.remove(&cache_key);
        }

        // Check disk cache
        if let Ok(module) = self.load_from_disk(&cache_key).await {
            debug!("Module cache hit (disk): {}", spec.name);

            // Add to memory cache if there's space
            if self.memory_cache.len() < self.max_memory_entries {
                self.memory_cache.insert(
                    cache_key,
                    CachedModule {
                        module: module.clone(),
                        cached_at: Instant::now(),
                        access_count: 1,
                        last_accessed: Instant::now(),
                    },
                );
            }

            return Some(module);
        }

        debug!("Module cache miss: {}", spec.name);
        None
    }

    pub async fn store(&mut self, module: LoadedModule) -> Result<(), CacheError> {
        let cache_key = self.cache_key(&module.spec);

        info!("Caching module: {}", module.spec.name);

        // Store in memory cache
        if self.memory_cache.len() >= self.max_memory_entries {
            self.evict_lru_memory().await?;
        }

        self.memory_cache.insert(
            cache_key.clone(),
            CachedModule {
                module: module.clone(),
                cached_at: Instant::now(),
                access_count: 1,
                last_accessed: Instant::now(),
            },
        );

        // Store on disk
        self.store_to_disk(&cache_key, &module).await?;

        // Check if we need to clean up old entries
        if self.should_cleanup().await? {
            self.cleanup_expired().await?;
        }

        Ok(())
    }

    pub async fn invalidate(&mut self, spec: &ModuleSpec) -> Result<(), CacheError> {
        let cache_key = self.cache_key(spec);

        info!("Invalidating module cache: {}", spec.name);

        // Remove from memory cache
        self.memory_cache.remove(&cache_key);

        // Remove from disk cache
        let module_dir = self.cache_dir.join(&cache_key);
        if module_dir.exists() {
            fs::remove_dir_all(&module_dir)
                .await
                .map_err(|e| CacheError::IoError {
                    operation: "remove cache directory".to_string(),
                    error: e.to_string(),
                })?;
        }

        Ok(())
    }

    pub async fn cleanup_expired(&mut self) -> Result<(), CacheError> {
        info!("Cleaning up expired cache entries");

        // Clean memory cache
        let expired_keys: Vec<String> = self
            .memory_cache
            .iter()
            .filter(|(_, cached)| self.is_expired(&cached.cached_at))
            .map(|(key, _)| key.clone())
            .collect();

        for key in expired_keys {
            self.memory_cache.remove(&key);
        }

        // Clean disk cache
        self.cleanup_disk_cache().await?;

        Ok(())
    }

    pub fn get_cache_stats(&self) -> CacheStats {
        let memory_entries = self.memory_cache.len();
        let total_access_count = self
            .memory_cache
            .values()
            .map(|cached| cached.access_count)
            .sum();

        CacheStats {
            total_entries: memory_entries, // TODO: Add disk entries count
            memory_entries,
            disk_entries: 0,     // TODO: Count disk entries
            total_size_bytes: 0, // TODO: Calculate total size
            hit_rate: 0.0,       // TODO: Track hit rate
            access_count: total_access_count,
        }
    }

    pub fn cache_key(&self, spec: &ModuleSpec) -> String {
        let mut hasher = Sha256::new();
        hasher.update(spec.name.as_bytes());
        hasher.update(spec.version.as_deref().unwrap_or("latest").as_bytes());
        hasher.update(format!("{:?}", spec.source).as_bytes());

        if let Some(checksum) = &spec.checksum {
            hasher.update(checksum.as_bytes());
        }

        format!("{:x}", hasher.finalize())
    }

    fn is_expired(&self, cached_at: &Instant) -> bool {
        cached_at.elapsed() > self.ttl
    }

    async fn load_from_disk(&self, cache_key: &str) -> Result<LoadedModule, CacheError> {
        let cache_path = self.cache_dir.join(cache_key);
        let metadata_path = cache_path.join("metadata.json");
        let module_path = cache_path.join("module.json");

        if !metadata_path.exists() || !module_path.exists() {
            return Err(CacheError::NotFound {
                key: cache_key.to_string(),
            });
        }

        // Read and validate metadata
        let metadata_content =
            fs::read_to_string(&metadata_path)
                .await
                .map_err(|e| CacheError::IoError {
                    operation: "read metadata".to_string(),
                    error: e.to_string(),
                })?;

        let metadata: CacheMetadata = serde_json::from_str(&metadata_content).map_err(|e| {
            CacheError::SerializationError {
                operation: "deserialize metadata".to_string(),
                error: e.to_string(),
            }
        })?;

        // Check if expired
        let cached_at = UNIX_EPOCH + Duration::from_secs(metadata.cached_at);
        if SystemTime::now()
            .duration_since(cached_at)
            .unwrap_or(Duration::MAX)
            > self.ttl
        {
            return Err(CacheError::Expired {
                key: cache_key.to_string(),
            });
        }

        // Read module data
        let _module_content =
            fs::read_to_string(&module_path)
                .await
                .map_err(|e| CacheError::IoError {
                    operation: "read module".to_string(),
                    error: e.to_string(),
                })?;

        // For now, return an error since we can't deserialize yet
        // In production, implement proper deserialization
        Err(CacheError::SerializationError {
            operation: "deserialize module".to_string(),
            error: "Module deserialization not implemented yet".to_string(),
        })
    }

    async fn store_to_disk(
        &self,
        cache_key: &str,
        module: &LoadedModule,
    ) -> Result<(), CacheError> {
        let cache_path = self.cache_dir.join(cache_key);

        // Create cache directory
        fs::create_dir_all(&cache_path)
            .await
            .map_err(|e| CacheError::IoError {
                operation: "create cache directory".to_string(),
                error: e.to_string(),
            })?;

        // Create metadata
        let metadata = CacheMetadata {
            cache_key: cache_key.to_string(),
            module_name: module.spec.name.clone(),
            module_version: module
                .spec
                .version
                .clone()
                .unwrap_or_else(|| "latest".to_string()),
            cached_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            access_count: 1,
            last_accessed: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            checksum: module
                .spec
                .checksum
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
        };

        // Write metadata
        let metadata_content = serde_json::to_string_pretty(&metadata).map_err(|e| {
            CacheError::SerializationError {
                operation: "serialize metadata".to_string(),
                error: e.to_string(),
            }
        })?;

        fs::write(cache_path.join("metadata.json"), metadata_content)
            .await
            .map_err(|e| CacheError::IoError {
                operation: "write metadata".to_string(),
                error: e.to_string(),
            })?;

        // Write module data (using a custom serializer for LoadedModule)
        let module_content = self.serialize_module(module)?;
        fs::write(cache_path.join("module.json"), module_content)
            .await
            .map_err(|e| CacheError::IoError {
                operation: "write module".to_string(),
                error: e.to_string(),
            })?;

        Ok(())
    }

    async fn evict_lru_memory(&mut self) -> Result<(), CacheError> {
        // Find the least recently used entry
        let lru_key = self
            .memory_cache
            .iter()
            .min_by_key(|(_, cached)| cached.last_accessed)
            .map(|(key, _)| key.clone());

        if let Some(key) = lru_key {
            debug!("Evicting LRU module from memory cache");
            self.memory_cache.remove(&key);
        }

        Ok(())
    }

    async fn should_cleanup(&self) -> Result<bool, CacheError> {
        // Check if cache directory size exceeds limit
        let total_size = self.calculate_cache_size().await?;
        Ok(total_size > self.max_cache_size)
    }

    async fn calculate_cache_size(&self) -> Result<usize, CacheError> {
        let mut total_size = 0;

        if !self.cache_dir.exists() {
            return Ok(0);
        }

        let mut entries = fs::read_dir(&self.cache_dir)
            .await
            .map_err(|e| CacheError::IoError {
                operation: "read cache directory".to_string(),
                error: e.to_string(),
            })?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| CacheError::IoError {
                operation: "read directory entry".to_string(),
                error: e.to_string(),
            })?
        {
            if entry.file_type().await.unwrap().is_dir() {
                total_size += self.calculate_directory_size(&entry.path()).await?;
            }
        }

        Ok(total_size)
    }

    async fn calculate_directory_size(&self, dir: &std::path::Path) -> Result<usize, CacheError> {
        let mut total_size = 0;
        let mut entries = fs::read_dir(dir).await.map_err(|e| CacheError::IoError {
            operation: "read directory".to_string(),
            error: e.to_string(),
        })?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| CacheError::IoError {
                operation: "read directory entry".to_string(),
                error: e.to_string(),
            })?
        {
            let metadata = entry.metadata().await.map_err(|e| CacheError::IoError {
                operation: "read file metadata".to_string(),
                error: e.to_string(),
            })?;

            total_size += metadata.len() as usize;
        }

        Ok(total_size)
    }

    async fn cleanup_disk_cache(&self) -> Result<(), CacheError> {
        if !self.cache_dir.exists() {
            return Ok(());
        }

        let mut entries = fs::read_dir(&self.cache_dir)
            .await
            .map_err(|e| CacheError::IoError {
                operation: "read cache directory".to_string(),
                error: e.to_string(),
            })?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| CacheError::IoError {
                operation: "read directory entry".to_string(),
                error: e.to_string(),
            })?
        {
            if entry.file_type().await.unwrap().is_dir() {
                let metadata_path = entry.path().join("metadata.json");

                if metadata_path.exists() {
                    if let Ok(metadata_content) = fs::read_to_string(&metadata_path).await {
                        if let Ok(metadata) =
                            serde_json::from_str::<CacheMetadata>(&metadata_content)
                        {
                            let cached_at = UNIX_EPOCH + Duration::from_secs(metadata.cached_at);
                            if SystemTime::now()
                                .duration_since(cached_at)
                                .unwrap_or(Duration::MAX)
                                > self.ttl
                            {
                                debug!("Removing expired cache entry: {}", metadata.module_name);
                                if let Err(e) = fs::remove_dir_all(&entry.path()).await {
                                    warn!("Failed to remove expired cache entry: {}", e);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn serialize_module(&self, module: &LoadedModule) -> Result<String, CacheError> {
        // For now, use a simple JSON representation
        // In production, implement proper serialization
        let simplified = format!(
            r#"{{
            "name": "{}",
            "version": "{}",
            "main_file_size": {},
            "additional_files_count": {}
        }}"#,
            module.spec.name,
            module.spec.version.as_deref().unwrap_or("unknown"),
            module.source_code.main_file.len(),
            module.source_code.additional_files.len()
        );

        Ok(simplified)
    }
}
