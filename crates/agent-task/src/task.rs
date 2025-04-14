use chrono::{DateTime, Utc};
use futures::future::BoxFuture;
use std::cmp::Ordering;

pub struct Task {
    id: String,
    name: String,
    description: String,
    created_at: DateTime<Utc>,
    status: TaskStatus,
    result: Option<Box<dyn TaskResult>>,
}

impl PartialEq for Task {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Task {}

impl PartialOrd for Task {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Task {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.created_at.cmp(&other.created_at) {
            Ordering::Equal => self.id.cmp(&other.id),
            ordering => ordering,
        }
    }
}

impl Task {
    pub fn new(id: String, name: String, description: String) -> Self {
        Self {
            id,
            name,
            description,
            created_at: Utc::now(),
            status: TaskStatus::InProgress,
            result: None,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn created_at(&self) -> &DateTime<Utc> {
        &self.created_at
    }

    pub fn status(&self) -> TaskStatus {
        self.status
    }

    pub fn result(&self) -> Option<&dyn TaskResult> {
        self.result.as_deref()
    }

    pub fn with_result(&self, result: Box<dyn TaskResult>) -> Self {
        let status = match result.finish_reason() {
            TaskFinishReason::Succeeded => TaskStatus::Succeeded,
            TaskFinishReason::Failed => TaskStatus::Failed,
            TaskFinishReason::Cancelled => TaskStatus::Cancelled,
        };

        Self {
            id: self.id.clone(),
            name: self.name.clone(),
            description: self.description.clone(),
            created_at: self.created_at,
            status,
            result: Some(result),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskStatus {
    InProgress,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct TaskProgress {
    title: String,
    description: String,
    progress: Option<f64>,
}

impl TaskProgress {
    pub fn new(title: String, description: String, progress: Option<f64>) -> Self {
        Self {
            title,
            description,
            progress,
        }
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn progress(&self) -> Option<f64> {
        self.progress
    }
}

pub trait TaskResult: Send + Sync + 'static {
    fn finish_reason(&self) -> TaskFinishReason;
    fn to_string(&self) -> String;
    fn to_json(&self) -> serde_json::Value;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskFinishReason {
    Succeeded,
    Failed,
    Cancelled,
}

pub type TaskSignalSender<T> = tokio::sync::mpsc::Sender<T>;
pub type TaskSignalReceiver<T> = tokio::sync::mpsc::Receiver<T>;

/// A trait for running a task. This trait is used to run a task by the `TaskManager`.
///
/// The task runner is responsible for:
/// - running the task
/// - sending progress updates to the task manager (optional, but highly recommended)
/// - sending the result to the task manager
///
/// Also, the task runner __MUST__ handle the `cancel_signal` to stop the task correctly.
///
/// You should signal the `result_signal` after all the task logic is finished, including critical resources cleanup.
/// All running instances will be terminated immediately after the result signal is sent.
///
/// The task will run forever unless:
/// - the task is cancelled (you will be notified by the `cancel_signal`)
/// - the task is finished (you should notify the task manager through the `result_signal`)
///
/// So you __MUST__ ensure that the task will be stopped eventually, notifying the task manager through the `result_signal`.
///
pub trait TaskRunner: Send + Sync + 'static {
    fn run(
        self,
        cancel_signal: TaskSignalReceiver<()>,
        progress_signal: TaskSignalSender<TaskProgress>,
        result_signal: TaskSignalSender<Box<dyn TaskResult>>,
    ) -> BoxFuture<'static, ()>;
}
