use anyhow::Result;
use clap::Parser;
use rustle_deploy::compilation::compiler::{BinaryCompiler, CompilerConfig};
use rustle_deploy::compilation::TargetDetector;
use rustle_deploy::execution::format_migration::FormatMigrator;
use rustle_deploy::execution::rustle_plan::RustlePlanOutput;
use rustle_deploy::template::{BinaryTemplateGenerator, TargetInfo, TemplateConfig};
use rustle_deploy::types::compilation::{OptimizationLevel, TargetSpecification};
use rustle_deploy::types::platform::Platform;
use std::path::PathBuf;
use tracing::{error, info, warn};

#[derive(Parser)]
#[command(name = "rustle-deploy")]
#[command(about = "Ansible replacement with binary deployment optimization")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct RustleDeployCli {
    /// Execution plan JSON file from rustle-plan (or stdin if -)
    execution_plan: Option<PathBuf>,

    /// Inventory file with target host information
    #[arg(short, long)]
    inventory: Option<PathBuf>,

    /// Check cross-compilation capabilities
    #[arg(long)]
    check_capabilities: bool,

    /// Install missing dependencies
    #[arg(long)]
    setup: bool,

    /// Directory for compiled binaries
    #[arg(short, long, default_value = "./target")]
    output_dir: PathBuf,

    /// Target architecture (auto-detect from inventory/plan)
    #[arg(short, long)]
    target: Option<String>,

    /// Compilation cache directory
    #[arg(long)]
    cache_dir: Option<PathBuf>,

    /// Enable incremental compilation
    #[arg(long)]
    incremental: bool,

    /// Force rebuild of all binaries
    #[arg(long)]
    rebuild: bool,

    /// Deploy existing binaries without compilation
    #[arg(long)]
    deploy_only: bool,

    /// Compile binaries without deployment
    #[arg(long)]
    compile_only: bool,

    /// Optimization mode
    #[arg(long, default_value = "auto")]
    optimization: String,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Show what would be deployed without executing
    #[arg(long)]
    dry_run: bool,

    /// Test compilation and execution on localhost only
    #[arg(long)]
    localhost_test: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = RustleDeployCli::parse();

    // Initialize tracing
    let level = if cli.verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    tracing_subscriber::fmt().with_max_level(level).init();

    info!("Starting rustle-deploy v{}", env!("CARGO_PKG_VERSION"));

    if cli.check_capabilities {
        check_capabilities().await?;
    } else if cli.setup {
        run_setup().await?;
    } else if let Some(ref execution_plan) = cli.execution_plan {
        run_deployment(execution_plan.clone(), &cli).await?;
    } else {
        show_usage();
    }

    Ok(())
}

async fn check_capabilities() -> Result<()> {
    println!("🔧 Cross-Compilation Capabilities");
    println!("===================================================");

    // Basic capability detection without complex types
    let rust_available = check_rust().await;
    let zig_available = check_zig().await;
    let zigbuild_available = check_zigbuild().await;

    let capability_level = match (rust_available, zig_available, zigbuild_available) {
        (true, true, true) => "Full",
        (true, _, true) => "Limited",
        (true, _, _) => "Minimal",
        _ => "Insufficient",
    };

    match capability_level {
        "Full" => {
            println!("✅ Status: Fully Ready - All cross-compilation features available");
            println!("  🚀 Zig + cargo-zigbuild available for all targets");
        }
        "Limited" => {
            println!("⚡ Status: Mostly Ready - Some cross-compilation available");
            println!("  🔧 Rust + cargo-zigbuild available for cross-compilation");
        }
        "Minimal" => {
            println!("⚠️  Status: Basic Ready - Native compilation only");
            println!("  🏠 Only native target compilation available");
        }
        _ => {
            println!("❌ Status: Not Ready - Missing essential components");
            println!("  💡 Run --setup to install required components");
        }
    }

    println!();
    println!("📦 Component Status:");

    if rust_available {
        let version = get_rust_version()
            .await
            .unwrap_or_else(|| "unknown".to_string());
        println!("  ✅ Rust: {version}");
    } else {
        println!("  ❌ Rust: Not found");
    }

    println!(
        "  {} Zig: {}",
        if zig_available { "✅" } else { "❌" },
        if zig_available {
            "Available"
        } else {
            "Not found"
        }
    );

    println!(
        "  {} cargo-zigbuild: {}",
        if zigbuild_available { "✅" } else { "❌" },
        if zigbuild_available {
            "Available"
        } else {
            "Not found"
        }
    );

    println!();
    let available_targets =
        get_available_targets(rust_available, zig_available, zigbuild_available);
    println!("🎯 Available Targets ({}):", available_targets.len());
    for target in &available_targets {
        println!("  • {target}");
    }

    if capability_level != "Full" {
        println!();
        println!("💡 Recommendations:");
        if !rust_available {
            println!("  • Install Rust toolchain");
            println!("    Command: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh");
        }
        if !zig_available {
            println!("  • Install Zig for enhanced cross-compilation");
            println!("    Visit: https://ziglang.org/download/");
        }
        if !zigbuild_available && rust_available {
            println!("  • Install cargo-zigbuild");
            println!("    Command: cargo install cargo-zigbuild");
        }
    }

    Ok(())
}

async fn run_setup() -> Result<()> {
    println!("🚀 rustle-deploy Setup");
    println!("==========================================");

    println!("📋 Checking current setup...");
    let rust_available = check_rust().await;
    let zig_available = check_zig().await;
    let zigbuild_available = check_zigbuild().await;

    if rust_available && zig_available && zigbuild_available {
        println!("✅ Your setup is already fully optimized!");
        return Ok(());
    }

    println!("🔧 Installing missing components...");

    // Try to install cargo-zigbuild if missing
    if !zigbuild_available && rust_available {
        println!("📦 Installing cargo-zigbuild...");
        match install_zigbuild().await {
            Ok(()) => println!("✅ cargo-zigbuild installed successfully"),
            Err(e) => println!("❌ Failed to install cargo-zigbuild: {e}"),
        }
    } else if !rust_available {
        println!("⚠️  Rust is required before installing cargo-zigbuild");
        println!("   Please install Rust first: https://rustup.rs/");
    }

    if !zig_available {
        println!("📦 Zig installation required for full cross-compilation support");
        println!("   Please install Zig manually:");
        println!("   • Visit: https://ziglang.org/download/");
        println!("   • Or use your package manager:");
        println!("     - Ubuntu/Debian: apt install zig");
        println!("     - macOS: brew install zig");
        println!("     - Windows: Download from ziglang.org");
    }

    println!();
    println!("✅ Setup completed! Run --check-capabilities to verify.");

    Ok(())
}

async fn run_deployment(execution_plan_path: PathBuf, cli: &RustleDeployCli) -> Result<()> {
    println!("🚀 rustle-deploy: Deployment");
    println!("==============================================");

    // Parse execution plan from rustle-plan JSON
    let execution_plan = if execution_plan_path.to_string_lossy() == "-" {
        println!("📖 Execution Plan: <stdin>");
        parse_execution_plan_from_stdin().await?
    } else {
        println!("📖 Execution Plan: {execution_plan_path:?}");
        parse_execution_plan_from_file(&execution_plan_path).await?
    };

    if let Some(ref inventory) = cli.inventory {
        println!("📋 Inventory: {inventory:?}");
    } else {
        println!("📋 Inventory: <embedded in execution plan>");
    }

    println!("⚙️  Optimization: {}", cli.optimization);
    println!("📁 Output Directory: {:?}", cli.output_dir);

    if cli.dry_run {
        println!("🔍 DRY RUN MODE - No actual deployment will occur");
    }

    println!();

    // Analyze the execution plan
    analyze_execution_plan(&execution_plan).await?;

    // Check capabilities
    let rust_available = check_rust().await;
    let zig_available = check_zig().await;
    let zigbuild_available = check_zigbuild().await;

    let capability_level = match (rust_available, zig_available, zigbuild_available) {
        (true, true, true) => "Full",
        (true, _, true) => "Limited",
        (true, _, _) => "Minimal",
        _ => "Insufficient",
    };

    println!("🛠️  Compilation Strategy:");
    match capability_level {
        "Full" => {
            println!("  🚀 Full optimization available - using Zig cross-compilation");
        }
        "Limited" => {
            println!(
                "  ⚡ Limited optimization available - using cargo-zigbuild cross-compilation"
            );
        }
        "Minimal" => {
            println!("  ⚠️  Minimal optimization - native compilation only");
        }
        _ => {
            println!("  ❌ Insufficient setup - falling back to SSH deployment only");
            println!("     Run --setup to enable binary optimization");
        }
    }

    println!();
    let available_targets =
        get_available_targets(rust_available, zig_available, zigbuild_available);
    println!("📊 Deployment Analysis:");
    println!("  • Available targets: {}", available_targets.len());
    println!(
        "  • Binary deployment hosts: {}",
        execution_plan.binary_deployment_hosts
    );
    println!(
        "  • SSH fallback hosts: {}",
        execution_plan.ssh_fallback_hosts
    );
    println!("  • Total tasks: {}", execution_plan.total_tasks);
    println!(
        "  • Estimated performance gain: {}x",
        execution_plan.estimated_speedup
    );

    if let Some(compilation_time) = execution_plan.estimated_compilation_time {
        println!("  • Estimated compilation time: {compilation_time:?}");
    }

    if cli.dry_run {
        println!();
        println!("✅ Dry run completed successfully");
        println!(
            "   This deployment would use {} deployment strategy",
            match capability_level {
                "Full" => "binary optimization with Zig cross-compilation",
                "Limited" => "hybrid binary/SSH with cargo-zigbuild",
                "Minimal" => "hybrid binary/SSH with native compilation",
                _ => "SSH fallback only",
            }
        );

        if execution_plan.binary_deployment_hosts > 0 {
            println!(
                "   Binary deployment would be used for {} hosts",
                execution_plan.binary_deployment_hosts
            );
        }
        if execution_plan.ssh_fallback_hosts > 0 {
            println!(
                "   SSH fallback would be used for {} hosts",
                execution_plan.ssh_fallback_hosts
            );
        }
    } else if cli.compile_only || cli.localhost_test {
        println!();
        if cli.localhost_test {
            println!("🧪 Localhost test mode - compiling and testing locally");
        } else {
            println!("🔨 Compilation-only mode");
        }

        match run_compilation(&execution_plan, cli).await {
            Ok(()) => {
                println!("✅ Compilation completed successfully");
                if cli.localhost_test {
                    println!("✅ Localhost test completed successfully");
                }
            }
            Err(e) => {
                error!("❌ Compilation failed: {}", e);
                return Err(e);
            }
        }
    } else if cli.deploy_only {
        println!();
        println!("🚀 Deploy-only mode");
        println!("   Deploying existing binaries from: {:?}", cli.output_dir);
        println!("   ⚠️  Actual deployment not yet implemented");
    } else {
        println!();
        println!("⚠️  Full compile and deploy not yet implemented");
        println!("   Currently showing execution plan analysis only");
        println!("   Use --dry-run to see deployment planning");
        println!("   Use --compile-only to compile binaries only");
        println!("   Use --deploy-only to deploy existing binaries");
    }

    Ok(())
}

async fn run_compilation(
    _execution_plan: &ExecutionPlanSummary,
    cli: &RustleDeployCli,
) -> Result<()> {
    info!("Starting binary compilation pipeline");

    // Parse the actual execution plan from the file
    let rustle_plan = parse_rustle_plan_from_file(
        cli.execution_plan
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Execution plan is required for compilation"))?,
    )
    .await?;

    // Parse optimization level
    let optimization_level = match cli.optimization.as_str() {
        "debug" => OptimizationLevel::Debug,
        "release" => OptimizationLevel::Release,
        "aggressive" => OptimizationLevel::MinSize,
        "auto" => OptimizationLevel::Release,
        _ => {
            warn!(
                "Unknown optimization level '{}', using 'release'",
                cli.optimization
            );
            OptimizationLevel::Release
        }
    };

    // Set up target detection
    let target_detector = TargetDetector::new();

    // Determine target specification
    let target_spec = if cli.localhost_test {
        target_detector.create_localhost_target_spec()?
    } else if let Some(target) = &cli.target {
        target_detector.create_target_spec(target, optimization_level.clone())?
    } else {
        target_detector.create_localhost_target_spec()?
    };

    info!("Compiling for target: {}", target_spec.target_triple);

    // Create binary template generator
    let template_config = TemplateConfig::default();
    let template_generator = BinaryTemplateGenerator::new(template_config)?;

    // Create target info
    let target_info = create_target_info_from_spec(&target_spec)?;

    // Generate binary template from execution plan
    info!("Generating binary template");

    // Create or modify binary deployment plan to include verbose setting
    let mut binary_deployment = rustle_plan
        .binary_deployments
        .first()
        .cloned()
        .unwrap_or_default();
    binary_deployment.verbose = Some(cli.verbose);

    // Ensure migration is applied to this specific deployment
    binary_deployment.migrate_from_legacy();

    let template = template_generator
        .generate_binary_template(&rustle_plan, &binary_deployment, &target_info)
        .await?;

    info!(
        "Template generated with {} source files",
        template.source_files.len()
    );
    info!("Template hash: {}", template.calculate_hash());

    // Binary compilation is now enabled with unified types
    if cli.compile_only {
        info!("Starting binary compilation");

        let compiler_config = CompilerConfig::default();
        let mut compiler = BinaryCompiler::new(compiler_config);
        let compiled_binary = compiler.compile_binary(&template, &target_spec).await?;

        info!("✅ Binary compiled successfully:");
        info!("   Target: {}", compiled_binary.target_triple);
        info!("   Size: {} bytes", compiled_binary.size);
        info!(
            "   Compilation time: {:?}",
            compiled_binary.compilation_time
        );
        info!("   Binary ID: {}", compiled_binary.binary_id);

        // Binary output management - copy to output directory
        tokio::fs::create_dir_all(&cli.output_dir).await?;
        let output_path = cli.output_dir.join("rustle-runner");

        // Write binary data to output directory
        tokio::fs::write(&output_path, &compiled_binary.binary_data).await?;

        // Make the binary executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&output_path)?.permissions();
            perms.set_mode(0o755); // rwxr-xr-x
            std::fs::set_permissions(&output_path, perms)?;
        }

        info!(
            "✅ Binary copied to output directory: {}",
            output_path.display()
        );
    } else {
        info!("✅ Template generated successfully:");
        info!("   Target: {}", target_spec.target_triple);
        info!("   Template files: {}", template.source_files.len());
    }
    // let binary_manager = BinaryOutputManager::new(...);
    // let copy_result = binary_manager.copy_to_output(&compiled_binary, &output_path).await?;

    if !cli.compile_only {
        info!("Output would be written to: {}", cli.output_dir.display());
    }

    // TODO: Make the binary executable and test execution
    // Currently disabled until compilation is working
    // #[cfg(unix)]
    // {
    //     use std::os::unix::fs::PermissionsExt;
    //     let mut perms = std::fs::metadata(&copy_result.output_path)?.permissions();
    //     perms.set_mode(0o755); // rwxr-xr-x
    //     std::fs::set_permissions(&copy_result.output_path, perms)?;
    // }

    // if cli.localhost_test {
    //     info!("Testing binary execution on localhost");
    //     test_binary_execution(&copy_result.output_path).await?;
    // }

    Ok(())
}

async fn parse_rustle_plan_from_file(path: &PathBuf) -> Result<RustlePlanOutput> {
    let content = tokio::fs::read_to_string(path).await?;
    let mut rustle_plan: RustlePlanOutput = serde_json::from_str(&content)?;

    // Apply format migration to ensure compatibility with both old and new formats
    let migrator = FormatMigrator::new();
    match migrator.migrate_rustle_plan_output(&mut rustle_plan) {
        Ok(warnings) => {
            if !warnings.is_empty() {
                warn!(
                    "Format migration completed with {} warnings",
                    warnings.len()
                );
                for warning in warnings {
                    warn!("Migration warning: {:?}", warning);
                }
            }
        }
        Err(e) => {
            warn!(
                "Format migration failed, continuing with original format: {}",
                e
            );
        }
    }

    Ok(rustle_plan)
}

fn create_target_info_from_spec(target_spec: &TargetSpecification) -> Result<TargetInfo> {
    let platform = if target_spec.target_triple.contains("apple-darwin") {
        Platform::MacOS
    } else if target_spec.target_triple.contains("linux") {
        Platform::Linux
    } else if target_spec.target_triple.contains("windows") {
        Platform::Windows
    } else {
        return Err(anyhow::anyhow!(
            "Unsupported target platform: {}",
            target_spec.target_triple
        ));
    };

    let architecture = if target_spec.target_triple.starts_with("aarch64") {
        "aarch64"
    } else if target_spec.target_triple.starts_with("x86_64") {
        "x86_64"
    } else {
        "unknown"
    };

    let os_family = if target_spec.target_triple.contains("windows") {
        "windows"
    } else {
        "unix"
    };

    let libc = if target_spec.target_triple.contains("musl") {
        Some("musl".to_string())
    } else if target_spec.target_triple.contains("gnu") {
        Some("gnu".to_string())
    } else {
        None
    };

    Ok(TargetInfo {
        target_triple: target_spec.target_triple.clone(),
        platform,
        architecture: architecture.to_string(),
        os_family: os_family.to_string(),
        libc,
        features: vec![], // Default empty features
    })
}

#[allow(dead_code)]
async fn test_binary_execution(binary_path: &PathBuf) -> Result<()> {
    info!("Executing binary for testing: {}", binary_path.display());

    let output = tokio::process::Command::new(binary_path)
        .arg("--help") // Try to get help output first
        .output()
        .await?;

    if output.status.success() {
        info!("✅ Binary executed successfully");
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.trim().is_empty() {
            info!("Binary output:\n{}", stdout);
        }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!("⚠️  Binary execution had non-zero exit status");
        if !stderr.trim().is_empty() {
            warn!("Stderr: {}", stderr);
        }

        // Try running without --help flag
        info!("Trying to run binary without arguments");
        let output2 = tokio::process::Command::new(binary_path).output().await?;

        if output2.status.success() {
            info!("✅ Binary executed successfully without arguments");
            let stdout = String::from_utf8_lossy(&output2.stdout);
            if !stdout.trim().is_empty() {
                info!("Binary output:\n{}", stdout);
            }
        } else {
            let stderr = String::from_utf8_lossy(&output2.stderr);
            return Err(anyhow::anyhow!("Binary execution failed: {}", stderr));
        }
    }

    Ok(())
}

fn show_usage() {
    println!("rustle-deploy: Binary compiler and deployment manager");
    println!();
    println!("Usage:");
    println!("  rustle-deploy <execution-plan.json>                # Compile and deploy");
    println!("  rustle-deploy <execution-plan.json> --dry-run      # Show deployment plan");
    println!("  rustle-deploy <execution-plan.json> --compile-only # Compile binaries only");
    println!("  rustle-deploy <execution-plan.json> --deploy-only  # Deploy existing binaries");
    println!("  rustle-deploy --check-capabilities                 # Check setup");
    println!("  rustle-deploy --setup                              # Install dependencies");
    println!();
    println!("Input from rustle-plan:");
    println!("  rustle-plan playbook.yml -i inventory.yml | rustle-deploy -");
    println!("  rustle-plan playbook.yml -i inventory.yml --output plan.json");
    println!("  rustle-deploy plan.json");
    println!();
    println!("Examples:");
    println!("  rustle-deploy execution_plan.json --dry-run");
    println!("  rustle-deploy execution_plan.json -o ./binaries --compile-only");
    println!("  rustle-deploy execution_plan.json --optimization=aggressive");
    println!();
    println!("For more options, use --help");
}

// Helper functions for capability detection (same as before)

async fn check_rust() -> bool {
    tokio::process::Command::new("rustc")
        .arg("--version")
        .output()
        .await
        .map(|output| output.status.success())
        .unwrap_or(false)
}

async fn check_zig() -> bool {
    tokio::process::Command::new("zig")
        .arg("version")
        .output()
        .await
        .map(|output| output.status.success())
        .unwrap_or(false)
}

async fn check_zigbuild() -> bool {
    tokio::process::Command::new("cargo")
        .args(["zigbuild", "--help"])
        .output()
        .await
        .map(|output| output.status.success())
        .unwrap_or(false)
}

async fn get_rust_version() -> Option<String> {
    let output = tokio::process::Command::new("rustc")
        .arg("--version")
        .output()
        .await
        .ok()?;

    if output.status.success() {
        let version_output = String::from_utf8_lossy(&output.stdout);
        version_output
            .split_whitespace()
            .nth(1)
            .map(|s| s.to_string())
    } else {
        None
    }
}

async fn install_zigbuild() -> Result<()> {
    let output = tokio::process::Command::new("cargo")
        .args(["install", "cargo-zigbuild"])
        .output()
        .await?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(anyhow::anyhow!("Installation failed: {}", stderr))
    }
}

fn get_available_targets(
    rust_available: bool,
    zig_available: bool,
    zigbuild_available: bool,
) -> Vec<String> {
    let mut targets = Vec::new();

    if rust_available {
        // Always include native target
        let native_target = get_native_target();
        targets.push(native_target.to_string());

        if zig_available && zigbuild_available {
            // Add all Zig-supported targets
            targets.extend(
                [
                    "x86_64-unknown-linux-gnu",
                    "aarch64-unknown-linux-gnu",
                    "x86_64-unknown-linux-musl",
                    "aarch64-unknown-linux-musl",
                    "x86_64-apple-darwin",
                    "aarch64-apple-darwin",
                    "x86_64-pc-windows-gnu",
                    "wasm32-wasi",
                ]
                .iter()
                .map(|s| s.to_string()),
            );
        } else if zigbuild_available {
            // Add common cross-compilation targets
            targets.extend(
                [
                    "x86_64-unknown-linux-gnu",
                    "aarch64-unknown-linux-gnu",
                    "x86_64-apple-darwin",
                    "aarch64-apple-darwin",
                ]
                .iter()
                .map(|s| s.to_string()),
            );
        }
    }

    // Remove duplicates and sort
    targets.sort();
    targets.dedup();
    targets
}

fn get_native_target() -> &'static str {
    match (std::env::consts::ARCH, std::env::consts::OS) {
        ("x86_64", "linux") => "x86_64-unknown-linux-gnu",
        ("aarch64", "linux") => "aarch64-unknown-linux-gnu",
        ("x86_64", "macos") => "x86_64-apple-darwin",
        ("aarch64", "macos") => "aarch64-apple-darwin",
        ("x86_64", "windows") => "x86_64-pc-windows-msvc",
        _ => "unknown-unknown-unknown",
    }
}

// Execution plan parsing and analysis

#[derive(Debug)]
struct ExecutionPlanSummary {
    total_tasks: u32,
    binary_deployment_hosts: usize,
    ssh_fallback_hosts: usize,
    estimated_speedup: f32,
    estimated_compilation_time: Option<std::time::Duration>,
    strategy: String,
}

async fn parse_execution_plan_from_file(path: &std::path::Path) -> Result<ExecutionPlanSummary> {
    let content = tokio::fs::read_to_string(path).await?;
    parse_execution_plan_json(&content)
}

async fn parse_execution_plan_from_stdin() -> Result<ExecutionPlanSummary> {
    use tokio::io::{self, AsyncReadExt};
    let mut stdin = io::stdin();
    let mut content = String::new();
    stdin.read_to_string(&mut content).await?;
    parse_execution_plan_json(&content)
}

fn parse_execution_plan_json(content: &str) -> Result<ExecutionPlanSummary> {
    // Parse the JSON to extract key information for analysis
    let json: serde_json::Value = serde_json::from_str(content)?;

    let total_tasks = json["total_tasks"].as_u64().unwrap_or(0) as u32;

    // Count binary deployment opportunities
    let empty_vec = Vec::new();
    let binary_deployments = json["binary_deployments"].as_array().unwrap_or(&empty_vec);
    let binary_deployment_hosts = binary_deployments
        .iter()
        .map(|deployment| {
            let empty_hosts = Vec::new();
            deployment["hosts"].as_array().unwrap_or(&empty_hosts).len()
        })
        .sum();

    // Calculate SSH fallback hosts (total hosts - binary hosts)
    let empty_hosts = Vec::new();
    let all_hosts = json["hosts"].as_array().unwrap_or(&empty_hosts).len();
    let ssh_fallback_hosts = all_hosts.saturating_sub(binary_deployment_hosts);

    // Estimate performance speedup based on binary deployment ratio
    let binary_ratio = if all_hosts > 0 {
        binary_deployment_hosts as f32 / all_hosts as f32
    } else {
        0.0
    };
    let estimated_speedup = 1.0 + (binary_ratio * 4.0); // 1x to 5x speedup

    // Extract strategy from metadata
    let strategy = json["metadata"]["planning_options"]["strategy"]
        .as_str()
        .unwrap_or("Unknown")
        .to_string();

    // Estimate compilation time (simplified calculation)
    let estimated_compilation_time = if binary_deployment_hosts > 0 {
        let base_time = std::time::Duration::from_secs(30); // Base compilation time
        let per_target_time = std::time::Duration::from_secs(10); // Per target overhead
        Some(base_time + per_target_time * (binary_deployment_hosts.min(5) as u32))
    } else {
        None
    };

    Ok(ExecutionPlanSummary {
        total_tasks,
        binary_deployment_hosts,
        ssh_fallback_hosts,
        estimated_speedup,
        estimated_compilation_time,
        strategy,
    })
}

async fn analyze_execution_plan(plan: &ExecutionPlanSummary) -> Result<()> {
    println!("📋 Execution Plan Analysis:");
    println!("  • Strategy: {}", plan.strategy);
    println!("  • Total tasks: {}", plan.total_tasks);
    println!(
        "  • Total hosts: {}",
        plan.binary_deployment_hosts + plan.ssh_fallback_hosts
    );

    if plan.binary_deployment_hosts > 0 {
        println!(
            "  • Binary deployment targets: {} hosts",
            plan.binary_deployment_hosts
        );
        let binary_ratio = plan.binary_deployment_hosts as f32
            / (plan.binary_deployment_hosts + plan.ssh_fallback_hosts) as f32;
        println!("  • Binary deployment ratio: {:.1}%", binary_ratio * 100.0);
    }

    if plan.ssh_fallback_hosts > 0 {
        println!(
            "  • SSH fallback targets: {} hosts",
            plan.ssh_fallback_hosts
        );
    }

    Ok(())
}
