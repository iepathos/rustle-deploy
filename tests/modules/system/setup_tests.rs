//! Tests for the setup module

use rustle_deploy::modules::interface::{
    ExecutionModule, ModuleArgs, ExecutionContext, HostInfo, Platform
};
use rustle_deploy::modules::system::setup::SetupModule;
use std::collections::HashMap;
use std::path::PathBuf;

fn create_test_context() -> ExecutionContext {
    ExecutionContext {
        facts: HashMap::new(),
        variables: HashMap::new(),
        host_info: HostInfo::detect(),
        working_directory: PathBuf::from("/tmp"),
        environment: HashMap::new(),
        check_mode: false,
        diff_mode: false,
        verbosity: 0,
    }
}

fn create_empty_args() -> ModuleArgs {
    ModuleArgs {
        args: HashMap::new(),
        special: Default::default(),
    }
}

#[tokio::test]
async fn test_setup_module_creation() {
    let module = SetupModule::new();
    assert_eq!(module.name(), "setup");
    assert_eq!(module.version(), "1.0.0");
    assert!(!module.supported_platforms().is_empty());
}

#[tokio::test]
async fn test_setup_module_validation() {
    let module = SetupModule::new();
    let args = create_empty_args();
    
    let result = module.validate_args(&args);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_setup_module_execution() {
    let module = SetupModule::new();
    let args = create_empty_args();
    let context = create_test_context();
    
    let result = module.execute(&args, &context).await;
    assert!(result.is_ok());
    
    let module_result = result.unwrap();
    assert!(!module_result.changed);
    assert!(!module_result.failed);
    assert!(!module_result.ansible_facts.is_empty());
    
    // Check for essential facts
    assert!(module_result.ansible_facts.contains_key("ansible_system"));
    assert!(module_result.ansible_facts.contains_key("ansible_hostname"));
    assert!(module_result.ansible_facts.contains_key("ansible_architecture"));
}

#[tokio::test]
async fn test_setup_module_check_mode() {
    let module = SetupModule::new();
    let args = create_empty_args();
    let context = create_test_context();
    
    let result = module.check_mode(&args, &context).await;
    assert!(result.is_ok());
    
    let module_result = result.unwrap();
    assert!(!module_result.changed);
    assert!(!module_result.failed);
}

#[tokio::test]
async fn test_setup_module_with_hardware_subset() {
    let module = SetupModule::new();
    let mut args = create_empty_args();
    args.args.insert(
        "gather_subset".to_string(),
        serde_json::json!(["hardware"])
    );
    
    let context = create_test_context();
    
    let result = module.execute(&args, &context).await;
    assert!(result.is_ok());
    
    let module_result = result.unwrap();
    assert!(!module_result.ansible_facts.is_empty());
}

#[tokio::test]
async fn test_setup_module_with_timeout() {
    let module = SetupModule::new();
    let mut args = create_empty_args();
    args.args.insert(
        "gather_timeout".to_string(),
        serde_json::json!(10)
    );
    
    let context = create_test_context();
    
    let result = module.execute(&args, &context).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_setup_module_documentation() {
    let module = SetupModule::new();
    let docs = module.documentation();
    
    assert!(!docs.description.is_empty());
    assert!(!docs.arguments.is_empty());
    assert!(!docs.examples.is_empty());
    assert!(!docs.return_values.is_empty());
    
    // Check that gather_subset argument exists
    assert!(docs.arguments.iter().any(|arg| arg.name == "gather_subset"));
}

#[cfg(test)]
mod fact_collection_tests {
    use super::*;
    use rustle_deploy::modules::system::facts::{
        SystemFacts, FactCategory,
        collector::{SystemFactCollector, FactCollector}
    };

    #[tokio::test]
    async fn test_fact_collector_creation() {
        let collector = SystemFactCollector::new();
        let facts = collector.collect_facts(&[FactCategory::Default]).await;
        assert!(facts.is_ok());
    }

    #[tokio::test]
    async fn test_system_facts_default() {
        let facts = SystemFacts::default();
        assert!(!facts.ansible_hostname.is_empty());
        assert!(!facts.ansible_architecture.is_empty());
    }

    #[tokio::test]
    async fn test_hardware_fact_collection() {
        let collector = SystemFactCollector::new();
        let facts = collector.collect_facts(&[FactCategory::Hardware]).await;
        assert!(facts.is_ok());
        
        let facts = facts.unwrap();
        assert!(facts.ansible_processor_vcpus > 0);
    }

    #[tokio::test]
    async fn test_network_fact_collection() {
        let collector = SystemFactCollector::new();
        let facts = collector.collect_facts(&[FactCategory::Network]).await;
        assert!(facts.is_ok());
        
        let facts = facts.unwrap();
        assert!(!facts.ansible_hostname.is_empty());
    }
}

#[cfg(test)]
mod cache_tests {
    use super::*;
    use rustle_deploy::modules::system::facts::{
        SystemFacts,
        cache::FactCache
    };
    use std::time::Duration;

    #[tokio::test]
    async fn test_fact_cache_operations() {
        let cache = FactCache::new(Duration::from_secs(60));
        let facts = SystemFacts::default();
        
        // Initially no facts
        assert!(cache.get_facts("test_host").await.is_none());
        
        // Cache facts
        cache.cache_facts("test_host", facts.clone(), None).await;
        
        // Should now have cached facts
        assert!(cache.get_facts("test_host").await.is_some());
        
        // Clear cache
        cache.clear_cache().await;
        assert!(cache.get_facts("test_host").await.is_none());
    }

    #[tokio::test]
    async fn test_fact_cache_ttl() {
        let cache = FactCache::new(Duration::from_millis(10));
        let facts = SystemFacts::default();
        
        cache.cache_facts("test_host", facts, None).await;
        assert!(cache.is_cached("test_host").await);
        
        // Wait for TTL to expire
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(!cache.is_cached("test_host").await);
    }
}