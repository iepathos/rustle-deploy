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
        let success = !failed && self.execution_state.completed_tasks > 0;

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
