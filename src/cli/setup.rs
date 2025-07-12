use crate::compilation::toolchain::ToolchainDetector;
use crate::deploy::Result;
use tracing::{info, warn};

/// Interactive setup wizard for first-time users
pub struct SetupWizard {
    detector: ToolchainDetector,
}

impl SetupWizard {
    pub fn new() -> Self {
        Self {
            detector: ToolchainDetector::new(),
        }
    }

    /// Run interactive setup process
    pub async fn run_interactive_setup(&mut self) -> Result<()> {
        println!("🚀 Welcome to rustle-deploy Setup");
        println!("===================================================");
        println!();

        // Detect current capabilities
        info!("Detecting current capabilities...");
        let capabilities = self.detector.detect_full_capabilities().await?;
        
        println!("Current setup status:");
        self.print_capability_summary(&capabilities);
        println!();

        // Get recommendations
        let recommendations = self.detector.recommend_setup_improvements(&capabilities);
        
        if recommendations.is_empty() {
            println!("✅ Your setup is fully optimized for deployment!");
            return Ok(());
        }

        println!("📝 Setup recommendations:");
        for (i, rec) in recommendations.iter().enumerate() {
            println!("{}. {} (Impact: {:?})", i + 1, rec.improvement, rec.impact);
            if let Some(cmd) = &rec.installation_command {
                println!("   Command: {}", cmd);
            }
            println!("   {}", rec.description);
            println!();
        }

        // Ask user if they want to proceed with automatic installation
        if self.prompt_yes_no("Would you like to install missing components automatically?") {
            self.install_missing_components().await?;
        } else {
            println!("Setup skipped. You can run 'rustle-deploy setup --install-all' later.");
        }

        Ok(())
    }

    /// Validate current setup and provide guidance
    pub async fn validate_setup(&mut self) -> Result<()> {
        info!("Validating setup");

        let validation_result = self.detector.validate_toolchain().await?;
        
        match validation_result.overall_health {
            crate::compilation::toolchain::HealthStatus::Excellent => {
                println!("✅ Excellent! Your setup is fully optimized for deployment.");
            }
            crate::compilation::toolchain::HealthStatus::Good => {
                println!("👍 Good setup! You have most cross-compilation features available.");
            }
            crate::compilation::toolchain::HealthStatus::Fair => {
                println!("⚠️  Fair setup. Basic compilation works, but cross-compilation is limited.");
            }
            crate::compilation::toolchain::HealthStatus::Poor => {
                println!("❌ Poor setup. Essential components are missing.");
            }
        }

        if !validation_result.issues.is_empty() {
            println!("\n⚠️  Issues found:");
            for issue in &validation_result.issues {
                println!("  • {}", issue);
            }
        }

        Ok(())
    }

    /// Create configuration file with optimized settings
    pub fn create_configuration_file(&self, config_path: &std::path::Path) -> Result<()> {
        let config_content = self.generate_default_config();
        
        std::fs::write(config_path, config_content)
            .map_err(|e| crate::deploy::DeployError::Configuration(format!("Failed to write config file: {}", e)))?;

        println!("📄 Configuration file created: {}", config_path.display());
        Ok(())
    }

    // Private helper methods

    async fn install_missing_components(&mut self) -> Result<()> {
        info!("Installing missing components");

        // Check what's missing and install
        if !self.detector.check_zigbuild_installation().await? {
            println!("📦 Installing cargo-zigbuild...");
            self.detector.install_zigbuild_if_missing().await?;
            println!("✅ cargo-zigbuild installed successfully");
        }

        // Note: Zig installation typically requires manual setup
        match self.detector.check_zig_installation().await? {
            None => {
                println!("⚠️  Zig is not installed. Please install it manually:");
                println!("   Visit: https://ziglang.org/download/");
                println!("   Or use your package manager:");
                println!("   • Ubuntu/Debian: apt install zig");
                println!("   • macOS: brew install zig");
                println!("   • Windows: Download from ziglang.org");
            }
            Some(_) => {
                println!("✅ Zig is already installed");
            }
        }

        Ok(())
    }

    fn print_capability_summary(&self, capabilities: &crate::compilation::capabilities::CompilationCapabilities) {
        match capabilities.capability_level {
            crate::compilation::capabilities::CapabilityLevel::Full => {
                println!("  🚀 Full capability: Zig + cargo-zigbuild available");
            }
            crate::compilation::capabilities::CapabilityLevel::Limited => {
                println!("  ⚡ Limited capability: Some cross-compilation available");
            }
            crate::compilation::capabilities::CapabilityLevel::Minimal => {
                println!("  ⚠️  Minimal capability: Native compilation only");
            }
            crate::compilation::capabilities::CapabilityLevel::Insufficient => {
                println!("  ❌ Insufficient capability: Missing requirements");
            }
        }

        println!("  • Available targets: {}", capabilities.available_targets.len());
        if let Some(version) = &capabilities.rust_version {
            println!("  • Rust version: {}", version);
        }
        println!("  • Zig available: {}", if capabilities.zig_available { "Yes" } else { "No" });
        println!("  • cargo-zigbuild: {}", if capabilities.zigbuild_available { "Yes" } else { "No" });
    }

    fn prompt_yes_no(&self, question: &str) -> bool {
        // In a real implementation, this would use proper terminal input
        // For now, default to yes for automated setup
        println!("{} [Y/n]", question);
        true
    }

    fn generate_default_config(&self) -> String {
        r#"# rustle-deploy Configuration
# This file contains optimized settings for cross-compilation

[compilation]
# Capability detection
auto_detect_capabilities = true
cache_capability_results = true
capability_cache_duration_hours = 24

# Cross-compilation preferences
prefer_zig_when_available = true
parallel_compilation = true
max_compilation_jobs = 4

# Optimization settings
optimization_threshold = 0.3    # Minimum benefit for binary deployment
compilation_timeout_secs = 300
binary_cache_size_mb = 1024

# Fallback behavior
auto_fallback_on_failure = true
fallback_timeout_secs = 30
preserve_failed_artifacts = false

[deployment]
# Strategy selection
default_optimization_mode = "auto"
binary_deployment_preference = 0.7  # Bias toward binary when close
ssh_deployment_preference = 0.3

# Performance tuning
parallel_deployments = true
max_deployment_threads = 8
deployment_timeout_secs = 1800

[toolchain]
# Tool discovery
rust_discovery_paths = ["/usr/local/bin", "~/.cargo/bin"]
zig_discovery_paths = ["/usr/local/bin", "~/zig"]
auto_install_missing = false

# Version requirements
minimum_rust_version = "1.70.0"
recommended_zig_version = "0.11.0"
"#.to_string()
    }
}

impl Default for SetupWizard {
    fn default() -> Self {
        Self::new()
    }
}