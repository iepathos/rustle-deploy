use rustle_deploy::compilation::capabilities::{
    CompilationCapabilities, CapabilityLevel, CompilationStrategy,
    detect_rust_installation, detect_zig_installation, is_zigbuild_available
};
use tokio_test;

#[tokio_test::test]
async fn test_rust_detection() {
    let rust_install = detect_rust_installation().await;
    
    // This test may fail in environments without Rust, which is expected
    if rust_install.is_ok() {
        let install = rust_install.unwrap();
        assert!(!install.version.is_empty());
        assert!(!install.targets.is_empty());
        assert!(install.cargo_path.exists());
        assert!(install.rustc_path.exists());
    }
}

#[tokio_test::test]
async fn test_zig_detection() {
    let zig_install = detect_zig_installation().await;
    
    // This test should always succeed (returns Ok(None) if Zig not found)
    assert!(zig_install.is_ok());
    
    if let Ok(Some(install)) = zig_install {
        assert!(!install.version.is_empty());
        assert!(install.zig_path.exists());
        assert!(install.supports_cross_compilation);
    }
}

#[tokio_test::test]
async fn test_zigbuild_detection() {
    let zigbuild_available = is_zigbuild_available().await;
    assert!(zigbuild_available.is_ok());
    
    // Result can be true or false depending on installation
    let _available = zigbuild_available.unwrap();
}

#[tokio_test::test]
async fn test_capability_levels() {
    let capabilities = CompilationCapabilities::detect_basic().await;
    assert!(capabilities.is_ok());
    
    let caps = capabilities.unwrap();
    
    // Test capability level logic
    match caps.capability_level {
        CapabilityLevel::Full => {
            assert!(caps.zig_available);
            assert!(caps.zigbuild_available);
        }
        CapabilityLevel::Limited => {
            // Either Rust-only cross-compilation or missing some components
            assert!(caps.rust_version.is_some());
        }
        CapabilityLevel::Minimal => {
            assert!(caps.rust_version.is_some());
            assert_eq!(caps.available_targets.len(), 1); // Only native target
        }
        CapabilityLevel::Insufficient => {
            assert!(caps.rust_version.is_none());
        }
    }
}

#[tokio_test::test]
async fn test_target_support() {
    let capabilities = CompilationCapabilities::detect_basic().await.unwrap();
    
    // Native target should always be supported if Rust is available
    if capabilities.rust_version.is_some() {
        assert!(capabilities.supports_target(&capabilities.native_target));
    }
    
    // Test unsupported target
    assert!(!capabilities.supports_target("invalid-target-triple"));
}

#[tokio_test::test]
async fn test_compilation_strategy() {
    let capabilities = CompilationCapabilities::detect_basic().await.unwrap();
    
    // Test strategy selection for native target
    let native_strategy = capabilities.get_strategy_for_target(&capabilities.native_target);
    
    if capabilities.zigbuild_available {
        // Should prefer ZigBuild if available and target is supported
        assert!(matches!(native_strategy, CompilationStrategy::ZigBuild | CompilationStrategy::NativeCargo));
    } else {
        assert!(matches!(native_strategy, CompilationStrategy::NativeCargo | CompilationStrategy::SshFallback));
    }
    
    // Test strategy for unsupported target
    let unsupported_strategy = capabilities.get_strategy_for_target("invalid-target");
    assert!(matches!(unsupported_strategy, CompilationStrategy::SshFallback));
}

#[tokio_test::test]
async fn test_recommendations() {
    let capabilities = CompilationCapabilities::detect_basic().await.unwrap();
    let recommendations = capabilities.get_recommendations();
    
    // Should have recommendations if not at full capability
    if capabilities.capability_level != CapabilityLevel::Full {
        assert!(!recommendations.is_empty());
    }
    
    // Check that recommendations are reasonable
    for rec in recommendations {
        assert!(!rec.improvement.is_empty());
        assert!(!rec.description.is_empty());
        // Installation command may or may not be present
    }
}

#[tokio_test::test]
async fn test_full_vs_basic_detection() {
    let basic = CompilationCapabilities::detect_basic().await.unwrap();
    let full = CompilationCapabilities::detect_full().await.unwrap();
    
    // Full detection should have at least as many targets as basic
    assert!(full.available_targets.len() >= basic.available_targets.len());
    
    // Core capability level should be the same
    assert_eq!(basic.capability_level, full.capability_level);
}