use crate::deploy::{DeployError, Result};
use crate::types::*;
use sha2::{Digest, Sha256};
use std::path::Path;
use tokio::process::Command;
use tracing::{debug, info, warn};

pub struct BinaryDeployer {
    connection_manager: ConnectionManager,
}

impl Default for BinaryDeployer {
    fn default() -> Self {
        Self::new()
    }
}

impl BinaryDeployer {
    pub fn new() -> Self {
        Self {
            connection_manager: ConnectionManager::new(),
        }
    }

    pub async fn deploy_to_host(
        &self,
        compilation: &BinaryCompilation,
        target: &DeploymentTarget,
    ) -> Result<()> {
        info!("Deploying binary to host: {}", target.host);

        // Read the compiled binary
        let binary_data =
            std::fs::read(&compilation.output_path).map_err(|e| DeployError::DeploymentFailed {
                host: target.host.clone(),
                reason: format!("Failed to read binary: {e}"),
            })?;

        match target.deployment_method {
            DeploymentMethod::Ssh => self.deploy_via_ssh(&binary_data, target).await,
            DeploymentMethod::Scp => self.deploy_via_scp(&binary_data, target).await,
            DeploymentMethod::Rsync => {
                self.deploy_via_rsync(&compilation.output_path, target)
                    .await
            }
            DeploymentMethod::Custom { ref command } => {
                self.deploy_via_custom(command, &compilation.output_path, target)
                    .await
            }
        }
    }

    pub async fn verify_deployment(&self, target: &DeploymentTarget) -> Result<bool> {
        info!("Verifying deployment on host: {}", target.host);

        let connection = self.connection_manager.get_connection(&target.host).await?;

        // Check if binary exists and is executable
        let check_cmd = format!("test -x {}", target.target_path);
        let result = connection.execute_command(&check_cmd).await?;

        if !result.success {
            debug!("Binary not found or not executable on {}", target.host);
            return Ok(false);
        }

        // Verify checksum if available
        if !target.version.is_empty() {
            let checksum_cmd = format!("sha256sum {} | cut -d' ' -f1", target.target_path);
            let checksum_result = connection.execute_command(&checksum_cmd).await?;

            if checksum_result.success {
                let deployed_checksum = checksum_result.stdout.trim();
                if deployed_checksum != target.version {
                    warn!(
                        "Checksum mismatch on {}: expected {}, got {}",
                        target.host, target.version, deployed_checksum
                    );
                    return Ok(false);
                }
            }
        }

        // Try to run binary with --version flag to ensure it's working
        let version_cmd = format!("{} --version", target.target_path);
        let version_result = connection.execute_command(&version_cmd).await?;

        if !version_result.success {
            debug!("Binary failed version check on {}", target.host);
            return Ok(false);
        }

        info!("Deployment verification successful for {}", target.host);
        Ok(true)
    }

    pub async fn execute_binary(
        &self,
        target: &DeploymentTarget,
        args: &[String],
    ) -> Result<ExecutionResult> {
        info!("Executing binary on host: {}", target.host);

        let connection = self.connection_manager.get_connection(&target.host).await?;

        let mut cmd = target.target_path.to_string();
        if !args.is_empty() {
            cmd.push(' ');
            cmd.push_str(&args.join(" "));
        }

        let start_time = std::time::Instant::now();
        let result = connection.execute_command(&cmd).await?;
        let execution_time = start_time.elapsed();

        Ok(ExecutionResult {
            exit_code: result.exit_code,
            stdout: result.stdout,
            stderr: result.stderr,
            execution_time,
        })
    }

    pub async fn cleanup_deployment(&self, target: &DeploymentTarget) -> Result<()> {
        info!("Cleaning up deployment on host: {}", target.host);

        let connection = self.connection_manager.get_connection(&target.host).await?;

        let cleanup_cmd = format!("rm -f {}", target.target_path);
        let result = connection.execute_command(&cleanup_cmd).await?;

        if !result.success {
            return Err(DeployError::DeploymentFailed {
                host: target.host.clone(),
                reason: format!("Failed to cleanup binary: {}", result.stderr),
            });
        }

        info!("Successfully cleaned up deployment on {}", target.host);
        Ok(())
    }

    // Private deployment methods

    async fn deploy_via_ssh(&self, binary_data: &[u8], target: &DeploymentTarget) -> Result<()> {
        let connection = self.connection_manager.get_connection(&target.host).await?;

        // Create temporary file for binary transfer
        let temp_path = format!("/tmp/rustle-runner-{}", uuid::Uuid::new_v4());

        // Upload binary data
        connection.upload_bytes(binary_data, &temp_path).await?;

        // Set executable permissions and move to target location
        let setup_cmd = format!(
            "chmod +x {} && mkdir -p {} && mv {} {}",
            temp_path,
            Path::new(&target.target_path).parent().unwrap().display(),
            temp_path,
            target.target_path
        );

        let result = connection.execute_command(&setup_cmd).await?;

        if !result.success {
            return Err(DeployError::DeploymentFailed {
                host: target.host.clone(),
                reason: format!("Failed to setup binary: {}", result.stderr),
            });
        }

        // Verify the deployment
        self.verify_binary_integrity(binary_data, target).await?;

        info!("Successfully deployed via SSH to {}", target.host);
        Ok(())
    }

    async fn deploy_via_scp(&self, binary_data: &[u8], target: &DeploymentTarget) -> Result<()> {
        // Create temporary local file
        let temp_file =
            tempfile::NamedTempFile::new().map_err(|e| DeployError::DeploymentFailed {
                host: target.host.clone(),
                reason: format!("Failed to create temp file: {e}"),
            })?;

        std::fs::write(temp_file.path(), binary_data).map_err(|e| {
            DeployError::DeploymentFailed {
                host: target.host.clone(),
                reason: format!("Failed to write temp file: {e}"),
            }
        })?;

        // Use scp to transfer the file
        let mut cmd = Command::new("scp");
        cmd.arg("-o")
            .arg("StrictHostKeyChecking=no")
            .arg(temp_file.path())
            .arg(format!("{}:{}", target.host, target.target_path));

        let output = cmd
            .output()
            .await
            .map_err(|e| DeployError::DeploymentFailed {
                host: target.host.clone(),
                reason: format!("Failed to execute scp: {e}"),
            })?;

        if !output.status.success() {
            return Err(DeployError::DeploymentFailed {
                host: target.host.clone(),
                reason: format!("SCP failed: {}", String::from_utf8_lossy(&output.stderr)),
            });
        }

        // Set executable permissions via SSH
        let connection = self.connection_manager.get_connection(&target.host).await?;
        let chmod_result = connection
            .execute_command(&format!("chmod +x {}", target.target_path))
            .await?;

        if !chmod_result.success {
            return Err(DeployError::DeploymentFailed {
                host: target.host.clone(),
                reason: format!(
                    "Failed to set executable permissions: {}",
                    chmod_result.stderr
                ),
            });
        }

        // Verify the deployment
        self.verify_binary_integrity(binary_data, target).await?;

        info!("Successfully deployed via SCP to {}", target.host);
        Ok(())
    }

    async fn deploy_via_rsync(&self, binary_path: &Path, target: &DeploymentTarget) -> Result<()> {
        let mut cmd = Command::new("rsync");
        cmd.arg("-avz")
            .arg("--progress")
            .arg(binary_path)
            .arg(format!("{}:{}", target.host, target.target_path));

        let output = cmd
            .output()
            .await
            .map_err(|e| DeployError::DeploymentFailed {
                host: target.host.clone(),
                reason: format!("Failed to execute rsync: {e}"),
            })?;

        if !output.status.success() {
            return Err(DeployError::DeploymentFailed {
                host: target.host.clone(),
                reason: format!("Rsync failed: {}", String::from_utf8_lossy(&output.stderr)),
            });
        }

        // Set executable permissions
        let connection = self.connection_manager.get_connection(&target.host).await?;
        let chmod_result = connection
            .execute_command(&format!("chmod +x {}", target.target_path))
            .await?;

        if !chmod_result.success {
            return Err(DeployError::DeploymentFailed {
                host: target.host.clone(),
                reason: format!(
                    "Failed to set executable permissions: {}",
                    chmod_result.stderr
                ),
            });
        }

        info!("Successfully deployed via rsync to {}", target.host);
        Ok(())
    }

    async fn deploy_via_custom(
        &self,
        command: &str,
        binary_path: &Path,
        target: &DeploymentTarget,
    ) -> Result<()> {
        // Replace placeholders in custom command
        let expanded_command = command
            .replace("{binary_path}", &binary_path.display().to_string())
            .replace("{target_host}", &target.host)
            .replace("{target_path}", &target.target_path);

        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(&expanded_command);

        let output = cmd
            .output()
            .await
            .map_err(|e| DeployError::DeploymentFailed {
                host: target.host.clone(),
                reason: format!("Failed to execute custom command: {e}"),
            })?;

        if !output.status.success() {
            return Err(DeployError::DeploymentFailed {
                host: target.host.clone(),
                reason: format!(
                    "Custom deployment failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ),
            });
        }

        info!(
            "Successfully deployed via custom command to {}",
            target.host
        );
        Ok(())
    }

    async fn verify_binary_integrity(
        &self,
        binary_data: &[u8],
        target: &DeploymentTarget,
    ) -> Result<()> {
        let connection = self.connection_manager.get_connection(&target.host).await?;

        // Calculate expected checksum
        let mut hasher = Sha256::new();
        hasher.update(binary_data);
        let expected_checksum = format!("{:x}", hasher.finalize());

        // Get deployed binary checksum
        let checksum_cmd = format!("sha256sum {} | cut -d' ' -f1", target.target_path);
        let result = connection.execute_command(&checksum_cmd).await?;

        if !result.success {
            return Err(DeployError::VerificationFailed {
                host: target.host.clone(),
                expected: expected_checksum,
                actual: "checksum command failed".to_string(),
            });
        }

        let actual_checksum = result.stdout.trim();
        if actual_checksum != expected_checksum {
            return Err(DeployError::VerificationFailed {
                host: target.host.clone(),
                expected: expected_checksum,
                actual: actual_checksum.to_string(),
            });
        }

        debug!("Binary integrity verified for {}", target.host);
        Ok(())
    }
}

// Connection management - simplified for now
pub struct ConnectionManager;

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self
    }

    pub async fn get_connection(&self, host: &str) -> Result<Connection> {
        // TODO: Implement actual SSH connection management
        // This would handle connection pooling, authentication, etc.
        Ok(Connection::new(host))
    }
}

pub struct Connection {
    host: String,
}

impl Connection {
    fn new(host: &str) -> Self {
        Self {
            host: host.to_string(),
        }
    }

    pub async fn execute_command(&self, command: &str) -> Result<CommandResult> {
        debug!("Executing command on {}: {}", self.host, command);

        // Use SSH to execute the command
        let mut cmd = Command::new("ssh");
        cmd.arg("-o")
            .arg("StrictHostKeyChecking=no")
            .arg(&self.host)
            .arg(command);

        let output = cmd
            .output()
            .await
            .map_err(|e| DeployError::Network(format!("SSH command failed: {e}")))?;

        Ok(CommandResult {
            success: output.status.success(),
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }

    pub async fn upload_bytes(&self, data: &[u8], remote_path: &str) -> Result<()> {
        // Create temporary local file
        let temp_file = tempfile::NamedTempFile::new()
            .map_err(|e| DeployError::Network(format!("Failed to create temp file: {e}")))?;

        std::fs::write(temp_file.path(), data)
            .map_err(|e| DeployError::Network(format!("Failed to write temp file: {e}")))?;

        // Use scp to upload
        let mut cmd = Command::new("scp");
        cmd.arg("-o")
            .arg("StrictHostKeyChecking=no")
            .arg(temp_file.path())
            .arg(format!("{}:{}", self.host, remote_path));

        let output = cmd
            .output()
            .await
            .map_err(|e| DeployError::Network(format!("SCP upload failed: {e}")))?;

        if !output.status.success() {
            return Err(DeployError::Network(format!(
                "SCP upload failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct CommandResult {
    pub success: bool,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}
