use tokio::process::Command;

/// Integration tests for CLI stdin functionality
/// These tests run the actual rustle-deploy binary to ensure end-to-end functionality

#[tokio::test]
async fn test_cli_help_works() {
    // Basic test to ensure the CLI is working
    let output = Command::new("cargo")
        .args(&["run", "--bin", "rustle-deploy", "--", "--help"])
        .output()
        .await
        .expect("Failed to run rustle-deploy --help");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("rustle-deploy") || stdout.contains("Usage:"));
}

#[tokio::test]
async fn test_cli_version_works() {
    // Test that version flag works
    let output = Command::new("cargo")
        .args(&["run", "--bin", "rustle-deploy", "--", "--version"])
        .output()
        .await
        .expect("Failed to run rustle-deploy --version");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("rustle-deploy"));
}

#[tokio::test]
async fn test_cli_capabilities_check() {
    // Test that capabilities check works
    let output = Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "rustle-deploy",
            "--",
            "--check-capabilities",
        ])
        .output()
        .await
        .expect("Failed to run capabilities check");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Cross-Compilation Capabilities"));
    assert!(stdout.contains("Component Status:"));
}

// Additional CLI tests can be added here as needed
