use crate::task::{
    Task, TaskFinishReason, TaskProgress, TaskResult, TaskRunner, TaskSignalReceiver,
    TaskSignalSender, TaskStatus,
};
use parking_lot::{Mutex, MutexGuard};
use std::{cmp::Ordering, sync::Arc};
use thiserror::Error;
use tokio::{select, task::JoinHandle};

/// A trait for handling task signals.
///
/// This trait is used to handle task signals, such as start, progress, and result.
pub trait TaskSignalHandler: Send + Sync + 'static {
    /// Called when the task starts.
    fn on_start(&self, task: Arc<Task>);
    /// Called when the task makes progress.
    fn on_progress(&self, task: Arc<Task>, progress: &TaskProgress);
    /// Called when the task finishes.
    fn on_result(&self, task: Task);
}

struct ManagedTask {
    pub task: Arc<Task>,
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

#[derive(Default)]
struct ManagedTaskList {
    tasks: Mutex<Vec<ManagedTask>>,
}

impl ManagedTaskList {
    fn try_add_task(&self, task: &Task) -> Option<OneTimeTaskAdder> {
        let tasks = self.tasks.lock();

        match tasks.binary_search_by(|managed| managed.task.as_ref().cmp(task)) {
            Ok(_) => None,
            Err(index) => Some(OneTimeTaskAdder { tasks, index }),
        }
    }

    fn find_task(&self, task_id: &str) -> Option<Arc<Task>> {
        let tasks = self.tasks.lock();

        tasks
            .iter()
            .find(|managed| managed.task.id() == task_id)
            .map(|managed| managed.task.clone())
    }

    fn remove_task(&self, task_id: &str) -> Option<ManagedTask> {
        let mut tasks = self.tasks.lock();

        let index = tasks
            .iter()
            .position(|managed| managed.task.id() == task_id)?;

        Some(tasks.remove(index))
    }
}

struct OneTimeTaskAdder<'a> {
    tasks: MutexGuard<'a, Vec<ManagedTask>>,
    index: usize,
}

impl OneTimeTaskAdder<'_> {
    pub fn insert(mut self, task: ManagedTask) {
        self.tasks.insert(self.index, task);
    }
}

struct TaskStartSignalArg {
    task_id: String,
    begin_signal: tokio::sync::oneshot::Sender<()>,
}

struct TaskProgressSignalArg {
    task_id: String,
    progress: TaskProgress,
}

struct TaskResultSignalArg {
    task_id: String,
    result: Box<dyn TaskResult>,
}

#[derive(Error, Debug)]
pub enum TaskAddFailure {
    #[error("the task `{task_id}` already exists")]
    AlreadyExists { task_id: String },
    #[error("the task `{task_id}` is already finished with status {status:?}")]
    TaskAlreadyFinished { task_id: String, status: TaskStatus },
    #[error("failed to start the task `{task_id}`")]
    FailedToStart { task_id: String },
}

#[derive(Error, Debug)]
pub enum TaskCancelFailure {
    #[error("the task `{task_id}` does not exist")]
    NotFound { task_id: String },
}

pub struct TaskManager {
    tasks: Arc<ManagedTaskList>,
    signal_handler: Arc<dyn TaskSignalHandler>,
    start_signal: TaskSignalSender<TaskStartSignalArg>,
    progress_signal: TaskSignalSender<TaskProgressSignalArg>,
    result_signal: TaskSignalSender<TaskResultSignalArg>,
    abort_signal: TaskSignalSender<()>,
    task_loop_handle: JoinHandle<()>,
}

impl TaskManager {
    pub fn new(signal_handler: Arc<dyn TaskSignalHandler>) -> Self {
        let tasks: Arc<ManagedTaskList> = Arc::new(ManagedTaskList::default());

        let (start_signal, start_signal_receiver) = tokio::sync::mpsc::channel(4);
        let (progress_signal, progress_signal_receiver) = tokio::sync::mpsc::channel(4);
        let (result_signal, result_signal_receiver) = tokio::sync::mpsc::channel(4);
        let (abort_signal, abort_signal_receiver) = tokio::sync::mpsc::channel(1);

        let task_loop_handle = tokio::spawn(task_loop(
            tasks.clone(),
            signal_handler.clone(),
            start_signal_receiver,
            progress_signal_receiver,
            result_signal_receiver,
            abort_signal_receiver,
        ));

        Self {
            tasks,
            signal_handler,
            start_signal,
            progress_signal,
            result_signal,
            abort_signal,
            task_loop_handle,
        }
    }

    pub async fn add_task(
        &self,
        task: Task,
        task_runner: impl TaskRunner,
    ) -> Result<(), TaskAddFailure> {
        if task.status() != TaskStatus::InProgress {
            return Err(TaskAddFailure::TaskAlreadyFinished {
                task_id: task.id().to_owned(),
                status: task.status(),
            });
        }

        let task_adder = match self.tasks.try_add_task(&task) {
            Some(adder) => adder,
            None => {
                return Err(TaskAddFailure::AlreadyExists {
                    task_id: task.id().to_owned(),
                });
            }
        };

        let (proxy_progress_signal, mut proxy_progress_signal_receiver) =
            tokio::sync::mpsc::channel::<TaskProgress>(4);
        let (proxy_result_signal, mut proxy_result_signal_receiver) =
            tokio::sync::mpsc::channel::<Box<dyn TaskResult>>(4);
        let (cancel_signal, cancel_signal_receiver) = tokio::sync::mpsc::channel(1);

        let progress_signal = self.progress_signal.clone();
        let result_signal = self.result_signal.clone();

        let task_id = task.id().to_owned();
        tokio::spawn(async move {
            while let Some(progress) = proxy_progress_signal_receiver.recv().await {
                let arg = TaskProgressSignalArg {
                    task_id: task_id.clone(),
                    progress,
                };

                if let Err(err) = progress_signal.send(arg).await {
                    log::warn!("failed to send progress signal: {err:#?}");
                }
            }
        });

        let task_id = task.id().to_owned();
        tokio::spawn(async move {
            while let Some(result) = proxy_result_signal_receiver.recv().await {
                let arg = TaskResultSignalArg {
                    task_id: task_id.clone(),
                    result,
                };

                if let Err(err) = result_signal.send(arg).await {
                    log::error!("failed to send result signal: {err:#?}");
                }
            }
        });

        let (begin_signal, begin_signal_receiver) = tokio::sync::oneshot::channel();
        let join_handle = tokio::spawn(async move {
            let _ = begin_signal_receiver.await;
            task_runner
                .run(
                    cancel_signal_receiver,
                    proxy_progress_signal,
                    proxy_result_signal,
                )
                .await;
        });

        let task_id = task.id().to_owned();
        task_adder.insert(ManagedTask {
            task: Arc::new(task),
            cancel_signal,
            join_handle,
        });

        let arg = TaskStartSignalArg {
            task_id: task_id.clone(),
            begin_signal,
        };

        if let Err(err) = self.start_signal.send(arg).await {
            log::warn!("failed to send start signal: {err:#?}");

            let managed = match self.tasks.remove_task(&task_id) {
                Some(managed) => managed,
                None => {
                    // NOTE: unreachable code
                    return Err(TaskAddFailure::FailedToStart {
                        task_id: task_id.to_owned(),
                    });
                }
            };

            // NOTE: the task is waiting for the start signal forever,
            // so forcefully abort it
            managed.join_handle.abort();

            // NOTE: signal forwarding tasks spawned above will terminated automatically
            // when the channel is disconnected (out of scope),
            // so we don't need to do anything else
            return Err(TaskAddFailure::FailedToStart {
                task_id: task_id.to_owned(),
            });
        }

        Ok(())
    }

    pub async fn cancel_task(&self, task_id: &str) -> Result<(), TaskCancelFailure> {
        let managed = match self.tasks.remove_task(task_id) {
            Some(managed) => managed,
            None => {
                return Err(TaskCancelFailure::NotFound {
                    task_id: task_id.to_owned(),
                });
            }
        };

        if let Err(err) = managed.cancel_signal.send(()).await {
            log::warn!("failed to send cancel signal: {err:#?}");
        }

        struct CanceledTaskResult;

        impl TaskResult for CanceledTaskResult {
            fn finish_reason(&self) -> TaskFinishReason {
                TaskFinishReason::Cancelled
            }

            fn to_string(&self) -> String {
                "cancelled".to_owned()
            }

            fn to_json(&self) -> serde_json::Value {
                serde_json::json!({
                    "finish_reason": "cancelled",
                    "message": "cancelled",
                })
            }
        }

        let task = managed.task.with_result(Box::new(CanceledTaskResult));
        self.signal_handler.on_result(task);

        Ok(())
    }

    pub async fn close(self) {
        if let Err(err) = self.abort_signal.send(()).await {
            log::warn!("failed to send abort signal: {err:#?}");
        }

        if let Err(err) = self.task_loop_handle.await {
            log::warn!("failed to await task loop handle: {err:#?}");
        }
    }
}

async fn task_loop(
    tasks: Arc<ManagedTaskList>,
    signal_handler: Arc<dyn TaskSignalHandler>,
    mut start_signal_receiver: TaskSignalReceiver<TaskStartSignalArg>,
    mut progress_signal_receiver: TaskSignalReceiver<TaskProgressSignalArg>,
    mut result_signal_receiver: TaskSignalReceiver<TaskResultSignalArg>,
    mut abort_signal_receiver: TaskSignalReceiver<()>,
) {
    loop {
        select! {
            arg = start_signal_receiver.recv() => {
                if let Some(arg) = arg {
                    handle_start_signal(arg, tasks.clone(), signal_handler.clone()).await;
                }
            }
            arg = progress_signal_receiver.recv() => {
                if let Some(arg) = arg {
                    handle_progress_signal(arg, tasks.clone(), signal_handler.clone()).await;
                }
            }
            arg = result_signal_receiver.recv() => {
                if let Some(arg) = arg {
                    handle_result_signal(arg, tasks.clone(), signal_handler.clone()).await;
                }
            }
            _ = abort_signal_receiver.recv() => {
                break;
            }
        }
    }
}

async fn handle_start_signal(
    arg: TaskStartSignalArg,
    tasks: Arc<ManagedTaskList>,
    signal_handler: Arc<dyn TaskSignalHandler>,
) {
    if arg.begin_signal.send(()).is_err() {
        log::warn!("failed to send begin signal for task `{}`", arg.task_id);
    }

    if let Some(task) = tasks.find_task(&arg.task_id) {
        signal_handler.on_start(task);
    }
}

async fn handle_progress_signal(
    arg: TaskProgressSignalArg,
    tasks: Arc<ManagedTaskList>,
    signal_handler: Arc<dyn TaskSignalHandler>,
) {
    if let Some(task) = tasks.find_task(&arg.task_id) {
        signal_handler.on_progress(task, &arg.progress);
    }
}

async fn handle_result_signal(
    arg: TaskResultSignalArg,
    tasks: Arc<ManagedTaskList>,
    signal_handler: Arc<dyn TaskSignalHandler>,
) {
    let managed = match tasks.remove_task(&arg.task_id) {
        Some(managed) => managed,
        None => {
            // NOTE: there is a chance that the task is not found,
            // because the task is removed by the `cancel_task` method.
            return;
        }
    };

    managed.join_handle.abort();

    let task = managed.task.with_result(arg.result);
    signal_handler.on_result(task);
}
