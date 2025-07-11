use crate::cli::commands::{CapabilityReport, ComponentStatus, ReadinessLevel, DeploymentResult};
use crate::compilation::optimizer::OptimizationAnalysis;
use std::collections::HashMap;

/// Print capability report in human-readable format
pub fn print_capability_report(report: &CapabilityReport) {
    println!("🔧 Zero-Infrastructure Cross-Compilation Capabilities");
    println!("================================================");
    println!();

    // Overall readiness
    match report.readiness_level {
        ReadinessLevel::FullyReady => {
            println!("✅ Status: Fully Ready - All cross-compilation features available");
        }
        ReadinessLevel::MostlyReady => {
            println!("⚡ Status: Mostly Ready - Cross-compilation available with some limitations");
        }
        ReadinessLevel::BasicReady => {
            println!("⚠️  Status: Basic Ready - Native compilation only");
        }
        ReadinessLevel::NotReady => {
            println!("❌ Status: Not Ready - Missing essential components");
        }
    }
    println!();

    // Component status
    println!("📦 Component Status:");
    print_component_status("Rust Toolchain", &report.rust_status);
    print_component_status("Zig", &report.zig_status);
    print_component_status("cargo-zigbuild", &report.zigbuild_status);
    println!();

    // Available targets
    if !report.available_targets.is_empty() {
        println!("🎯 Available Targets ({}):", report.available_targets.len());
        for target in &report.available_targets {
            println!("  • {}", target);
        }
        println!();
    }

    // Recommendations
    if !report.recommendations.is_empty() {
        println!("💡 Recommendations:");
        for recommendation in &report.recommendations {
            println!("  • {}", recommendation);
        }
        println!();
    }

    // Quick setup commands
    match report.readiness_level {
        ReadinessLevel::NotReady | ReadinessLevel::BasicReady => {
            println!("🚀 Quick Setup:");
            if matches!(report.zig_status, ComponentStatus::Missing) {
                println!("  # Install Zig");
                println!("  curl -o zig.tar.xz https://ziglang.org/download/0.11.0/zig-linux-x86_64-0.11.0.tar.xz");
                println!("  tar -xf zig.tar.xz && sudo mv zig-* /opt/zig && export PATH=$PATH:/opt/zig");
            }
            if matches!(report.zigbuild_status, ComponentStatus::Missing) {
                println!("  # Install cargo-zigbuild");
                println!("  cargo install cargo-zigbuild");
            }
        }
        _ => {}
    }
}

/// Print component status with appropriate icons
fn print_component_status(name: &str, status: &ComponentStatus) {
    match status {
        ComponentStatus::Available { version } => {
            println!("  ✅ {}: {} ({})", name, "Available", version);
        }
        ComponentStatus::Missing => {
            println!("  ❌ {}: Missing", name);
        }
        ComponentStatus::Outdated { current, recommended } => {
            println!("  ⚠️  {}: Outdated ({} → {})", name, current, recommended);
        }
        ComponentStatus::Error { message } => {
            println!("  ❌ {}: Error - {}", name, message);
        }
    }
}

/// Print optimization analysis results
pub fn print_optimization_analysis(analysis: &OptimizationAnalysis) {
    println!("📊 Deployment Optimization Analysis");
    println!("=================================");
    println!();

    // Overall score
    let score_percent = (analysis.optimization_score * 100.0) as u32;
    let score_icon = match analysis.optimization_score {
        s if s >= 0.8 => "🚀",
        s if s >= 0.5 => "⚡",
        s if s >= 0.3 => "⚠️",
        _ => "❌",
    };
    
    println!("{} Optimization Score: {}% ({:.2}/1.0)", score_icon, score_percent, analysis.optimization_score);
    println!();

    // Task compatibility
    let compatibility_percent = if analysis.total_tasks > 0 {
        (analysis.binary_compatible_tasks as f32 / analysis.total_tasks as f32 * 100.0) as u32
    } else {
        0
    };
    
    println!("🔄 Task Compatibility:");
    println!("  • Binary-compatible tasks: {}/{} ({}%)", 
             analysis.binary_compatible_tasks, analysis.total_tasks, compatibility_percent);
    println!("  • Estimated speedup: {:.1}x", analysis.estimated_speedup);
    println!("  • Compilation overhead: {:?}", analysis.compilation_overhead);
    println!();

    // Strategy recommendation
    println!("💭 Recommended Strategy:");
    match analysis.recommended_strategy {
        crate::compilation::optimizer::RecommendedStrategy::BinaryOnly => {
            println!("  🎯 Binary Deployment Only - Maximum performance optimization");
        }
        crate::compilation::optimizer::RecommendedStrategy::Hybrid => {
            println!("  ⚖️  Hybrid Deployment - Mix of binary and SSH for optimal balance");
        }
        crate::compilation::optimizer::RecommendedStrategy::SshOnly => {
            println!("  📡 SSH Deployment Only - Low optimization potential or high overhead");
        }
    }
    println!();

    // Target breakdown
    if !analysis.target_breakdown.is_empty() {
        println!("🎯 Target Breakdown:");
        for (target, target_analysis) in &analysis.target_breakdown {
            let benefit_icon = if target_analysis.estimated_benefit >= 5.0 {
                "🚀"
            } else if target_analysis.estimated_benefit >= 2.0 {
                "⚡"
            } else {
                "📡"
            };
            
            println!("  {} {} ({} hosts):", benefit_icon, target, target_analysis.host_count);
            println!("    • Compatible tasks: {}", target_analysis.compatible_tasks);
            println!("    • Compilation feasible: {}", if target_analysis.compilation_feasible { "Yes" } else { "No" });
            println!("    • Estimated benefit: {:.1}x", target_analysis.estimated_benefit);
        }
    }
}

/// Print deployment summary
pub fn print_deployment_summary(result: &DeploymentResult) {
    println!("📋 Deployment Summary");
    println!("==================");
    println!();

    // Overall status
    let status_icon = if result.success { "✅" } else { "❌" };
    println!("{} Overall Status: {}", status_icon, if result.success { "Success" } else { "Failed" });
    println!("⏱️  Total Duration: {:?}", result.total_duration);
    
    if let Some(gain) = result.performance_gain {
        println!("🚀 Performance Gain: {:.1}x", gain);
    }
    println!();

    // Binary deployments
    if !result.binary_deployments.is_empty() {
        println!("⚡ Binary Deployments ({}):", result.binary_deployments.len());
        for deployment in &result.binary_deployments {
            let status_icon = if deployment.success { "✅" } else { "❌" };
            println!("  {} {} → {} hosts ({:?})", 
                     status_icon, deployment.target, deployment.hosts.len(), deployment.duration);
        }
        println!();
    }

    // SSH deployments
    if !result.ssh_deployments.is_empty() {
        println!("📡 SSH Deployments ({}):", result.ssh_deployments.len());
        for deployment in &result.ssh_deployments {
            let status_icon = if deployment.success { "✅" } else { "❌" };
            println!("  {} {} hosts ({:?}) - {}", 
                     status_icon, deployment.hosts.len(), deployment.duration, deployment.fallback_reason);
        }
        println!();
    }

    // Errors
    if !result.errors.is_empty() {
        println!("⚠️  Errors:");
        for error in &result.errors {
            println!("  • {}", error);
        }
        println!();
    }

    // Performance summary
    let total_deployments = result.binary_deployments.len() + result.ssh_deployments.len();
    if total_deployments > 0 {
        let binary_ratio = result.binary_deployments.len() as f32 / total_deployments as f32;
        println!("📈 Performance Summary:");
        println!("  • Binary deployment ratio: {:.1}%", binary_ratio * 100.0);
        
        if binary_ratio > 0.5 {
            println!("  🎉 Excellent optimization - majority of deployments used binary strategy");
        } else if binary_ratio > 0.2 {
            println!("  👍 Good optimization - partial binary deployment achieved");
        } else {
            println!("  📡 Limited optimization - mostly SSH deployment used");
        }
    }
}

/// Print capability report in JSON format
pub fn print_capability_report_json(report: &CapabilityReport) -> Result<(), serde_json::Error> {
    let json_output = serde_json::to_string_pretty(&CapabilityReportJson {
        readiness_level: format!("{:?}", report.readiness_level),
        components: ComponentsJson {
            rust: component_status_to_json(&report.rust_status),
            zig: component_status_to_json(&report.zig_status),
            zigbuild: component_status_to_json(&report.zigbuild_status),
        },
        available_targets: report.available_targets.clone(),
        recommendations: report.recommendations.clone(),
    })?;
    
    println!("{}", json_output);
    Ok(())
}

#[derive(serde::Serialize)]
struct CapabilityReportJson {
    readiness_level: String,
    components: ComponentsJson,
    available_targets: Vec<String>,
    recommendations: Vec<String>,
}

#[derive(serde::Serialize)]
struct ComponentsJson {
    rust: ComponentJson,
    zig: ComponentJson,
    zigbuild: ComponentJson,
}

#[derive(serde::Serialize)]
struct ComponentJson {
    status: String,
    version: Option<String>,
    message: Option<String>,
}

fn component_status_to_json(status: &ComponentStatus) -> ComponentJson {
    match status {
        ComponentStatus::Available { version } => ComponentJson {
            status: "available".to_string(),
            version: Some(version.clone()),
            message: None,
        },
        ComponentStatus::Missing => ComponentJson {
            status: "missing".to_string(),
            version: None,
            message: None,
        },
        ComponentStatus::Outdated { current, recommended } => ComponentJson {
            status: "outdated".to_string(),
            version: Some(current.clone()),
            message: Some(format!("Recommended: {}", recommended)),
        },
        ComponentStatus::Error { message } => ComponentJson {
            status: "error".to_string(),
            version: None,
            message: Some(message.clone()),
        },
    }
}