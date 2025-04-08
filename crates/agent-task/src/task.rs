use chrono::{DateTime, Utc};
use futures::future::BoxFuture;
use std::{cmp::Ordering, future::Future, pin::Pin};

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
            status: TaskStatus::Pending,
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

    pub fn status(&self) -> &TaskStatus {
        &self.status
    }

    pub fn result(&self) -> Option<&dyn TaskResult> {
        self.result.as_deref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
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
    Success,
    Failed,
    Cancelled,
}

pub type TaskSignalSender<T> = tokio::sync::mpsc::Sender<T>;
pub type TaskSignalReceiver<T> = tokio::sync::mpsc::Receiver<T>;

pub trait TaskRunner: Send + Sync + 'static {
    fn run(
        self,
        cancel_signal: TaskSignalReceiver<()>,
        progress_signal: TaskSignalSender<TaskProgress>,
        result_signal: TaskSignalSender<Box<dyn TaskResult>>,
    ) -> BoxFuture<'static, ()>;
}
