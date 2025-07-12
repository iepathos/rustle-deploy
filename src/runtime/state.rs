use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Task execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub name: String,
    pub status: TaskStatus,
    pub changed: bool,
    pub failed: bool,
    pub skipped: bool,
    pub output: serde_json::Value,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub duration: Duration,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Running,
    Success,
    Failed,
    Skipped,
    Timeout,
    Cancelled,
}

/// Overall execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub execution_id: String,
    pub success: bool,
    pub failed: bool,
    pub task_results: HashMap<String, TaskResult>,
    pub summary: ExecutionSummary,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub duration: Duration,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSummary {
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
    pub skipped_tasks: usize,
    pub changed_tasks: usize,
}

/// Play execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayResult {
    pub play_name: String,
    pub success: bool,
    pub task_results: HashMap<String, TaskResult>,
    pub duration: Duration,
}

/// Batch execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult {
    pub task_results: HashMap<String, TaskResult>,
    pub overall_success: bool,
    pub duration: Duration,
}

/// Execution state management
pub struct StateManager {
    task_results: HashMap<String, TaskResult>,
    execution_state: ExecutionState,
    facts: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionState {
    pub execution_id: String,
    pub current_play: Option<String>,
    pub current_task: Option<String>,
    pub failed_tasks: Vec<String>,
    pub changed_tasks: Vec<String>,
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub start_time: DateTime<Utc>,
}

impl StateManager {
    pub fn new(execution_id: String, total_tasks: usize) -> Self {
        Self {
            task_results: HashMap::new(),
            execution_state: ExecutionState {
                execution_id,
                current_play: None,
                current_task: None,
                failed_tasks: Vec::new(),
                changed_tasks: Vec::new(),
                total_tasks,
                completed_tasks: 0,
                start_time: Utc::now(),
            },
            facts: HashMap::new(),
        }
    }

    pub fn add_task_result(&mut self, result: TaskResult) {
        if result.failed {
            self.execution_state
                .failed_tasks
                .push(result.task_id.clone());
        }

        if result.changed {
            self.execution_state
                .changed_tasks
                .push(result.task_id.clone());
        }

        if !result.skipped {
            self.execution_state.completed_tasks += 1;
        }

        self.task_results.insert(result.task_id.clone(), result);
    }

    pub fn get_task_result(&self, task_id: &str) -> Option<&TaskResult> {
        self.task_results.get(task_id)
    }

    pub fn get_all_task_results(&self) -> &HashMap<String, TaskResult> {
        &self.task_results
    }

    pub fn set_current_play(&mut self, play_name: Option<String>) {
        self.execution_state.current_play = play_name;
    }

    pub fn set_current_task(&mut self, task_id: Option<String>) {
        self.execution_state.current_task = task_id;
    }

    pub fn get_execution_state(&self) -> &ExecutionState {
        &self.execution_state
    }

    pub fn set_facts(&mut self, facts: HashMap<String, serde_json::Value>) {
        self.facts = facts;
    }

    pub fn get_facts(&self) -> &HashMap<String, serde_json::Value> {
        &self.facts
    }

    pub fn build_execution_result(&self, end_time: DateTime<Utc>) -> ExecutionResult {
        let duration = (end_time - self.execution_state.start_time)
            .to_std()
            .unwrap_or(Duration::from_secs(0));

        let summary = ExecutionSummary {
            total_tasks: self.execution_state.total_tasks,
            completed_tasks: self.execution_state.completed_tasks,
            failed_tasks: self.execution_state.failed_tasks.len(),
            skipped_tasks: self.task_results.values().filter(|r| r.skipped).count(),
            changed_tasks: self.execution_state.changed_tasks.len(),
        };

        let failed = !self.execution_state.failed_tasks.is_empty();
        let success = !failed;

        let errors = self
            .task_results
            .values()
            .filter_map(|r| r.error.as_ref())
            .cloned()
            .collect();

        ExecutionResult {
            execution_id: self.execution_state.execution_id.clone(),
            success,
            failed,
            task_results: self.task_results.clone(),
            summary,
            start_time: self.execution_state.start_time,
            end_time,
            duration,
            errors,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::time::Duration;

    #[test]
    fn test_execution_success_calculation() {
        let mut state_manager = StateManager::new("test-execution".to_string(), 3);

        // Add successful task results
        let task_results = vec![
            TaskResult {
                task_id: "task_0".to_string(),
                name: "Debug Task".to_string(),
                status: TaskStatus::Success,
                changed: false,
                failed: false,
                skipped: false,
                output: serde_json::json!({"msg": "hello world"}),
                stdout: None,
                stderr: None,
                start_time: Utc::now(),
                end_time: Utc::now(),
                duration: Duration::from_millis(10),
                error: None,
            },
            TaskResult {
                task_id: "task_1".to_string(),
                name: "Package Task".to_string(),
                status: TaskStatus::Success,
                changed: false,
                failed: false,
                skipped: false,
                output: serde_json::json!({"changed": false}),
                stdout: None,
                stderr: None,
                start_time: Utc::now(),
                end_time: Utc::now(),
                duration: Duration::from_millis(100),
                error: None,
            },
            TaskResult {
                task_id: "task_2".to_string(),
                name: "Command Task".to_string(),
                status: TaskStatus::Success,
                changed: true,
                failed: false,
                skipped: false,
                output: serde_json::json!({"rc": 0}),
                stdout: Some("success".to_string()),
                stderr: None,
                start_time: Utc::now(),
                end_time: Utc::now(),
                duration: Duration::from_millis(50),
                error: None,
            },
        ];

        for result in task_results {
            state_manager.add_task_result(result);
        }

        let execution_result = state_manager.build_execution_result(Utc::now());

        // The key assertions: if all tasks complete successfully (even if some don't change anything),
        // the overall execution should be marked as successful, not failed
        assert!(
            !execution_result.failed,
            "Execution should not be marked as failed when all tasks complete successfully"
        );
        assert!(
            execution_result.success,
            "Execution should be marked as successful when all tasks complete successfully"
        );
        assert_eq!(execution_result.summary.total_tasks, 3);
        assert_eq!(execution_result.summary.completed_tasks, 3);
        assert_eq!(execution_result.summary.failed_tasks, 0);
        assert_eq!(execution_result.summary.changed_tasks, 1);
    }

    #[test]
    fn test_execution_failure_calculation() {
        let mut state_manager = StateManager::new("test-execution".to_string(), 2);

        // Add one successful and one failed task
        let successful_task = TaskResult {
            task_id: "task_0".to_string(),
            name: "Successful Task".to_string(),
            status: TaskStatus::Success,
            changed: false,
            failed: false,
            skipped: false,
            output: serde_json::json!({"msg": "success"}),
            stdout: None,
            stderr: None,
            start_time: Utc::now(),
            end_time: Utc::now(),
            duration: Duration::from_millis(10),
            error: None,
        };

        let failed_task = TaskResult {
            task_id: "task_1".to_string(),
            name: "Failed Task".to_string(),
            status: TaskStatus::Failed,
            changed: false,
            failed: true,
            skipped: false,
            output: serde_json::json!({"msg": "failed"}),
            stdout: None,
            stderr: Some("error".to_string()),
            start_time: Utc::now(),
            end_time: Utc::now(),
            duration: Duration::from_millis(5),
            error: Some("Task failed".to_string()),
        };

        state_manager.add_task_result(successful_task);
        state_manager.add_task_result(failed_task);

        let execution_result = state_manager.build_execution_result(Utc::now());

        // When there are failed tasks, execution should be marked as failed
        assert!(
            execution_result.failed,
            "Execution should be marked as failed when tasks fail"
        );
        assert!(
            !execution_result.success,
            "Execution should not be marked as successful when tasks fail"
        );
        assert_eq!(execution_result.summary.total_tasks, 2);
        assert_eq!(execution_result.summary.completed_tasks, 2); // Both completed, even if one failed
        assert_eq!(execution_result.summary.failed_tasks, 1);
        assert_eq!(execution_result.summary.changed_tasks, 0);
    }
}
