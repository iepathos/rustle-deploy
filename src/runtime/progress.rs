use crate::execution::Task;
use crate::runtime::{ExecutionResult, ReportError, TaskResult};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Progress reporting for controller communication
pub struct ProgressReporter {
    controller_endpoint: Option<String>,
    client: Option<Client>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionProgress {
    pub execution_id: String,
    pub current_task: Option<String>,
    pub completed_tasks: usize,
    pub total_tasks: usize,
    pub failed_tasks: usize,
    pub changed_tasks: usize,
    pub elapsed_time: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ProgressEvent {
    ExecutionStarted {
        execution_id: String,
        total_tasks: usize,
    },
    TaskStarted {
        execution_id: String,
        task_id: String,
        task_name: String,
    },
    TaskCompleted {
        execution_id: String,
        task_result: TaskResult,
    },
    ExecutionCompleted {
        execution_id: String,
        result: ExecutionResult,
    },
    ExecutionFailed {
        execution_id: String,
        error: String,
    },
}

impl ProgressReporter {
    pub fn new(controller_endpoint: Option<String>) -> Self {
        let client = if controller_endpoint.is_some() {
            Some(
                Client::builder()
                    .timeout(Duration::from_secs(10))
                    .build()
                    .unwrap_or_else(|_| Client::new()),
            )
        } else {
            None
        };

        Self {
            controller_endpoint,
            client,
        }
    }

    pub async fn report_execution_start(
        &self,
        execution_id: &str,
        total_tasks: usize,
    ) -> Result<(), ReportError> {
        let event = ProgressEvent::ExecutionStarted {
            execution_id: execution_id.to_string(),
            total_tasks,
        };
        self.send_event(&event).await
    }

    pub async fn report_task_start(&self, task: &Task) -> Result<(), ReportError> {
        let event = ProgressEvent::TaskStarted {
            execution_id: "default".to_string(), // TODO: Get from context
            task_id: task.id.clone(),
            task_name: task.name.clone(),
        };
        self.send_event(&event).await
    }

    pub async fn report_task_complete(&self, result: &TaskResult) -> Result<(), ReportError> {
        let event = ProgressEvent::TaskCompleted {
            execution_id: "default".to_string(), // TODO: Get from context
            task_result: result.clone(),
        };
        self.send_event(&event).await
    }

    pub async fn report_execution_complete(
        &self,
        result: &ExecutionResult,
    ) -> Result<(), ReportError> {
        let event = ProgressEvent::ExecutionCompleted {
            execution_id: result.execution_id.clone(),
            result: result.clone(),
        };
        self.send_event(&event).await
    }

    pub async fn report_error(&self, execution_id: &str, error: &str) -> Result<(), ReportError> {
        let event = ProgressEvent::ExecutionFailed {
            execution_id: execution_id.to_string(),
            error: error.to_string(),
        };
        self.send_event(&event).await
    }

    pub async fn report_progress(&self, progress: &ExecutionProgress) -> Result<(), ReportError> {
        // For now, just log the progress
        tracing::info!(
            "Execution progress: {}/{} tasks completed ({} failed, {} changed)",
            progress.completed_tasks,
            progress.total_tasks,
            progress.failed_tasks,
            progress.changed_tasks
        );

        // TODO: Send to controller if endpoint is configured
        Ok(())
    }

    async fn send_event(&self, event: &ProgressEvent) -> Result<(), ReportError> {
        // Log the event locally
        match event {
            ProgressEvent::ExecutionStarted { total_tasks, .. } => {
                tracing::info!("Execution started with {} tasks", total_tasks);
            }
            ProgressEvent::TaskStarted { task_name, .. } => {
                tracing::info!("Starting task: {}", task_name);
            }
            ProgressEvent::TaskCompleted { task_result, .. } => {
                if task_result.failed {
                    tracing::error!(
                        "Task '{}' failed: {:?}",
                        task_result.name,
                        task_result.error
                    );
                } else if task_result.skipped {
                    tracing::info!("Task '{}' skipped", task_result.name);
                } else {
                    tracing::info!(
                        "Task '{}' completed{}",
                        task_result.name,
                        if task_result.changed {
                            " (changed)"
                        } else {
                            ""
                        }
                    );
                }
            }
            ProgressEvent::ExecutionCompleted { result, .. } => {
                tracing::info!(
                    "Execution completed: {}/{} tasks successful",
                    result.summary.completed_tasks - result.summary.failed_tasks,
                    result.summary.total_tasks
                );
            }
            ProgressEvent::ExecutionFailed { error, .. } => {
                tracing::error!("Execution failed: {}", error);
            }
        }

        // Send to controller if configured
        if let (Some(endpoint), Some(client)) = (&self.controller_endpoint, &self.client) {
            let url = format!("{endpoint}/api/v1/progress");

            match client.post(&url).json(event).send().await {
                Ok(response) => {
                    if !response.status().is_success() {
                        tracing::warn!(
                            "Failed to send progress to controller: HTTP {}",
                            response.status()
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to send progress to controller: {}", e);
                    // Don't fail execution if controller communication fails
                }
            }
        }

        Ok(())
    }
}
