//! Facts caching system for performance optimization

use super::SystemFacts;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

pub struct FactCache {
    cache: Arc<RwLock<HashMap<String, CachedFacts>>>,
    default_ttl: Duration,
}

#[derive(Debug, Clone)]
struct CachedFacts {
    facts: SystemFacts,
    timestamp: SystemTime,
    ttl: Duration,
}

impl FactCache {
    pub fn new(default_ttl: Duration) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            default_ttl,
        }
    }

    pub async fn get_facts(&self, host: &str) -> Option<SystemFacts> {
        let cache = self.cache.read().await;
        if let Some(cached) = cache.get(host) {
            if cached.timestamp.elapsed().unwrap_or(Duration::MAX) < cached.ttl {
                return Some(cached.facts.clone());
            }
        }
        None
    }

    pub async fn cache_facts(&self, host: &str, facts: SystemFacts, ttl: Option<Duration>) {
        let mut cache = self.cache.write().await;
        cache.insert(
            host.to_string(),
            CachedFacts {
                facts,
                timestamp: SystemTime::now(),
                ttl: ttl.unwrap_or(self.default_ttl),
            },
        );
    }

    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    pub async fn remove_host(&self, host: &str) {
        let mut cache = self.cache.write().await;
        cache.remove(host);
    }

    pub async fn is_cached(&self, host: &str) -> bool {
        let cache = self.cache.read().await;
        if let Some(cached) = cache.get(host) {
            cached.timestamp.elapsed().unwrap_or(Duration::MAX) < cached.ttl
        } else {
            false
        }
    }
}

impl Default for FactCache {
    fn default() -> Self {
        Self::new(Duration::from_secs(3600)) // 1 hour default TTL
    }
}
