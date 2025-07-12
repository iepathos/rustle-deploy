use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use super::GeneratedTemplate;

/// Template caching system for improved performance
pub struct TemplateCache {
    enabled: bool,
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    max_size: usize,
    ttl: Duration,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    template: GeneratedTemplate,
    created_at: Instant,
    access_count: u64,
    last_accessed: Instant,
}

impl TemplateCache {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            cache: Arc::new(RwLock::new(HashMap::new())),
            max_size: 100,                                  // Cache up to 100 templates
            ttl: <Duration as DurationExt>::from_hours(24), // 24 hour TTL
        }
    }

    /// Get cached template by key
    pub fn get(&self, key: &str) -> Option<GeneratedTemplate> {
        if !self.enabled {
            return None;
        }

        let mut cache = self.cache.write().ok()?;

        if let Some(entry) = cache.get_mut(key) {
            // Check if entry is still valid
            if entry.created_at.elapsed() < self.ttl {
                entry.access_count += 1;
                entry.last_accessed = Instant::now();
                Some(entry.template.clone())
            } else {
                // Entry expired, remove it
                cache.remove(key);
                None
            }
        } else {
            None
        }
    }

    /// Insert template into cache
    pub fn insert(&self, key: String, template: GeneratedTemplate) {
        if !self.enabled {
            return;
        }

        let mut cache = self.cache.write().ok().unwrap();

        // Check if we need to evict entries
        if cache.len() >= self.max_size {
            self.evict_lru(&mut cache);
        }

        let entry = CacheEntry {
            template,
            created_at: Instant::now(),
            access_count: 0,
            last_accessed: Instant::now(),
        };

        cache.insert(key, entry);
    }

    /// Remove expired entries and perform maintenance
    pub fn cleanup(&self) {
        if !self.enabled {
            return;
        }

        let mut cache = self.cache.write().ok().unwrap();
        let now = Instant::now();

        cache.retain(|_, entry| now.duration_since(entry.created_at) < self.ttl);
    }

    /// Clear all cached templates
    pub fn clear(&self) {
        if !self.enabled {
            return;
        }

        let mut cache = self.cache.write().ok().unwrap();
        cache.clear();
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        if !self.enabled {
            return CacheStats::default();
        }

        let cache = self.cache.read().ok().unwrap();
        let total_entries = cache.len();
        let total_access_count = cache.values().map(|e| e.access_count).sum();

        let now = Instant::now();
        let expired_entries = cache
            .values()
            .filter(|e| now.duration_since(e.created_at) >= self.ttl)
            .count();

        CacheStats {
            total_entries,
            expired_entries,
            total_access_count,
            hit_rate: if total_access_count > 0 {
                cache.values().filter(|e| e.access_count > 0).count() as f64 / total_entries as f64
            } else {
                0.0
            },
        }
    }

    /// Evict least recently used entry
    fn evict_lru(&self, cache: &mut HashMap<String, CacheEntry>) {
        if cache.is_empty() {
            return;
        }

        // Find the least recently used entry
        let lru_key = cache
            .iter()
            .min_by_key(|(_, entry)| entry.last_accessed)
            .map(|(key, _)| key.clone());

        if let Some(key) = lru_key {
            cache.remove(&key);
        }
    }
}

impl Default for TemplateCache {
    fn default() -> Self {
        Self::new(true)
    }
}

#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub total_entries: usize,
    pub expired_entries: usize,
    pub total_access_count: u64,
    pub hit_rate: f64,
}

/// Background cache maintenance task
pub struct CacheMaintainer {
    cache: Arc<TemplateCache>,
    interval: Duration,
}

impl CacheMaintainer {
    pub fn new(cache: Arc<TemplateCache>, interval: Duration) -> Self {
        Self { cache, interval }
    }

    /// Start background maintenance task
    pub async fn start(&self) -> Result<()> {
        let cache = self.cache.clone();
        let interval = self.interval;

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);

            loop {
                ticker.tick().await;
                cache.cleanup();
            }
        });

        Ok(())
    }
}

// Extend Duration with helper methods
trait DurationExt {
    fn from_hours(hours: u64) -> Duration;
}

impl DurationExt for Duration {
    fn from_hours(hours: u64) -> Duration {
        Duration::from_secs(hours * 3600)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::template::{EmbeddedData, EncryptedSecrets, TargetInfo};
    use crate::types::deployment::RuntimeConfig;
    use std::collections::HashMap;

    fn create_test_template(id: &str) -> GeneratedTemplate {
        GeneratedTemplate {
            template_id: id.to_string(),
            source_files: HashMap::new(),
            embedded_data: EmbeddedData {
                execution_plan: "{}".to_string(),
                static_files: HashMap::new(),
                module_binaries: HashMap::new(),
                runtime_config: RuntimeConfig {
                    controller_endpoint: None,
                    execution_timeout: Duration::from_secs(300),
                    report_interval: Duration::from_secs(30),
                    cleanup_on_completion: true,
                    log_level: "info".to_string(),
                    verbose: false,
                },
                secrets: EncryptedSecrets {
                    vault_data: HashMap::new(),
                    encryption_key_id: "test".to_string(),
                    decryption_method: "none".to_string(),
                },
                facts_cache: None,
            },
            cargo_toml: String::new(),
            build_script: None,
            target_info: TargetInfo {
                target_triple: "x86_64-unknown-linux-gnu".to_string(),
                platform: crate::types::platform::Platform::Linux,
                architecture: "x86_64".to_string(),
                os_family: "unix".to_string(),
                libc: Some("glibc".to_string()),
                features: vec![],
            },
            compilation_flags: vec![],
            estimated_binary_size: 5_000_000,
            cache_key: id.to_string(),
        }
    }

    #[test]
    fn test_cache_insert_and_get() {
        let cache = TemplateCache::new(true);
        let template = create_test_template("test1");

        cache.insert("key1".to_string(), template.clone());

        let retrieved = cache.get("key1");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().template_id, "test1");
    }

    #[test]
    fn test_cache_disabled() {
        let cache = TemplateCache::new(false);
        let template = create_test_template("test1");

        cache.insert("key1".to_string(), template);

        let retrieved = cache.get("key1");
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_cache_stats() {
        let cache = TemplateCache::new(true);
        let template = create_test_template("test1");

        cache.insert("key1".to_string(), template);
        cache.get("key1"); // Access the template

        let stats = cache.stats();
        assert_eq!(stats.total_entries, 1);
        assert_eq!(stats.total_access_count, 1);
    }

    #[test]
    fn test_cache_clear() {
        let cache = TemplateCache::new(true);
        let template = create_test_template("test1");

        cache.insert("key1".to_string(), template);
        assert!(cache.get("key1").is_some());

        cache.clear();
        assert!(cache.get("key1").is_none());
    }
}
