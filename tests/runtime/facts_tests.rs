use rustle_deploy::runtime::{FactsCollector, FactsCache};
use std::time::Duration;

#[test]
fn test_facts_collection() {
    let facts = FactsCollector::collect_all_facts().unwrap();
    
    // Check that basic facts are collected
    assert!(facts.contains_key("ansible_hostname"));
    assert!(facts.contains_key("ansible_system"));
    assert!(facts.contains_key("ansible_architecture"));
    
    // Verify values are not empty
    let hostname = facts.get("ansible_hostname").unwrap();
    assert!(hostname.is_string());
    assert!(!hostname.as_str().unwrap().is_empty());
    
    let system = facts.get("ansible_system").unwrap();
    assert!(system.is_string());
    assert!(!system.as_str().unwrap().is_empty());
    
    let arch = facts.get("ansible_architecture").unwrap();
    assert!(arch.is_string());
    assert!(!arch.as_str().unwrap().is_empty());
}

#[test]
fn test_os_specific_facts() {
    let facts = FactsCollector::collect_all_facts().unwrap();
    
    // Check OS family
    let os_family = facts.get("ansible_os_family").unwrap();
    assert!(os_family.is_string());
    
    let family = os_family.as_str().unwrap();
    assert!(family == "Darwin" || family == "Windows" || family == "RedHat");
}

#[test]
fn test_network_facts() {
    let facts = FactsCollector::collect_all_facts().unwrap();
    
    // Check that network interfaces are collected
    if let Some(interfaces) = facts.get("ansible_interfaces") {
        assert!(interfaces.is_array());
        let interfaces_array = interfaces.as_array().unwrap();
        
        // Should have at least one interface
        if !interfaces_array.is_empty() {
            for interface in interfaces_array {
                assert!(interface.is_string());
                assert!(!interface.as_str().unwrap().is_empty());
            }
        }
    }
}

#[test]
fn test_facts_cache() {
    let mut cache = FactsCache::new(Duration::from_secs(60));
    
    // Cache should start empty
    assert!(cache.get("test_fact").is_none());
    
    // Set a fact
    cache.set("test_fact".to_string(), serde_json::json!("test_value"));
    
    // Should be able to retrieve it
    let cached_value = cache.get("test_fact").unwrap();
    assert_eq!(cached_value.as_str().unwrap(), "test_value");
    
    // Invalidate the fact
    cache.invalidate("test_fact");
    assert!(cache.get("test_fact").is_none());
}

#[test]
fn test_facts_cache_expiration() {
    let mut cache = FactsCache::new(Duration::from_millis(10)); // Very short TTL
    
    // Set a fact
    cache.set("test_fact".to_string(), serde_json::json!("test_value"));
    
    // Should be retrievable immediately
    assert!(cache.get("test_fact").is_some());
    
    // Wait for expiration
    std::thread::sleep(Duration::from_millis(20));
    
    // Should be expired now
    assert!(cache.get("test_fact").is_none());
}

#[test]
fn test_facts_cache_refresh() {
    let mut cache = FactsCache::new(Duration::from_secs(60));
    
    // Refresh facts from system
    cache.refresh_facts().unwrap();
    
    // Should have system facts cached
    let all_facts = cache.get_all_facts();
    assert!(!all_facts.is_empty());
    assert!(all_facts.contains_key("ansible_hostname"));
}

#[test]
fn test_facts_cache_clear_expired() {
    let mut cache = FactsCache::new(Duration::from_millis(10));
    
    // Add some facts
    cache.set("fact1".to_string(), serde_json::json!("value1"));
    cache.set("fact2".to_string(), serde_json::json!("value2"));
    
    // Wait for expiration
    std::thread::sleep(Duration::from_millis(20));
    
    // Add a fresh fact
    cache.set("fact3".to_string(), serde_json::json!("value3"));
    
    // Clear expired facts
    cache.clear_expired();
    
    // Only the fresh fact should remain
    let all_facts = cache.get_all_facts();
    assert!(!all_facts.contains_key("fact1"));
    assert!(!all_facts.contains_key("fact2"));
    assert!(all_facts.contains_key("fact3"));
}