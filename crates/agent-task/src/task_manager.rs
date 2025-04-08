use crate::task::{Task, TaskProgress, TaskResult, TaskRunner, TaskSignalSender};
use parking_lot::Mutex;
use std::cmp::Ordering;
use tokio::task::JoinHandle;

struct ManagedTask {
    pub task: Task,
    pub cancel_signal: TaskSignalSender<()>,
    pub join_handle: JoinHandle<()>,
}

impl PartialEq for ManagedTask {
    fn eq(&self, other: &Self) -> bool {
        self.task.eq(&other.task)
    }
}

impl Eq for ManagedTask {}

impl PartialOrd for ManagedTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.task.cmp(&other.task))
    }
}

impl Ord for ManagedTask {
    fn cmp(&self, other: &Self) -> Ordering {
        self.task.cmp(&other.task)
    }
}

pub struct TaskManager {
    tasks: Mutex<Vec<ManagedTask>>,
    progress_signal: TaskSignalSender<TaskProgress>,
    result_signal: TaskSignalSender<Box<dyn TaskResult>>,
}

impl TaskManager {
    pub fn new() -> Self {
        let (progress_signal, progress_signal_receiver) = tokio::sync::mpsc::channel(8);
        let (result_signal, result_signal_receiver) = tokio::sync::mpsc::channel(1);

        Self {
            tasks: Mutex::new(Vec::new()),
            progress_signal,
            result_signal,
        }
    }

    pub fn add_task(&self, task: Task, task_runner: impl TaskRunner) {
        let (cancel_signal, cancel_signal_receiver) = tokio::sync::mpsc::channel(1);
        let progress_signal = self.progress_signal.clone();
        let result_signal = self.result_signal.clone();

        let join_handle = tokio::spawn(async move {
            task_runner
                .run(cancel_signal_receiver, progress_signal, result_signal)
                .await;
        });
        let managed_task = ManagedTask {
            task,
            cancel_signal,
            join_handle,
        };

        let mut tasks = self.tasks.lock();
        tasks.push(managed_task);
        tasks.sort_unstable();
    }
}
