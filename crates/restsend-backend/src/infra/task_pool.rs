use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

use tokio::sync::{mpsc, Mutex};

type Job = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

#[derive(Clone)]
pub struct TaskPool {
    sender: mpsc::Sender<Job>,
    stats: Arc<TaskPoolStats>,
}

#[derive(Default)]
struct TaskPoolStats {
    queued: AtomicUsize,
    active: AtomicUsize,
    submitted: AtomicU64,
    completed: AtomicU64,
    rejected: AtomicU64,
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskPoolSnapshot {
    pub queued: usize,
    pub active: usize,
    pub submitted: u64,
    pub completed: u64,
    pub rejected: u64,
}

impl TaskPool {
    pub fn new(worker_count: usize, queue_size: usize) -> Self {
        let worker_count = worker_count.max(1);
        let queue_size = queue_size.max(worker_count);
        let (sender, receiver) = mpsc::channel::<Job>(queue_size);
        let receiver = Arc::new(Mutex::new(receiver));
        let stats = Arc::new(TaskPoolStats::default());

        for _ in 0..worker_count {
            let receiver = receiver.clone();
            let stats = stats.clone();
            tokio::spawn(async move {
                loop {
                    let job = {
                        let mut rx = receiver.lock().await;
                        rx.recv().await
                    };
                    match job {
                        Some(job) => {
                            stats.queued.fetch_sub(1, Ordering::Relaxed);
                            stats.active.fetch_add(1, Ordering::Relaxed);
                            job.await;
                            stats.active.fetch_sub(1, Ordering::Relaxed);
                            stats.completed.fetch_add(1, Ordering::Relaxed);
                        }
                        None => break,
                    }
                }
            });
        }

        Self { sender, stats }
    }

    pub async fn submit<F>(&self, job: F) -> Result<(), mpsc::error::SendError<Job>>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.stats.submitted.fetch_add(1, Ordering::Relaxed);
        self.stats.queued.fetch_add(1, Ordering::Relaxed);
        match self.sender.send(Box::pin(job)).await {
            Ok(()) => Ok(()),
            Err(err) => {
                self.stats.queued.fetch_sub(1, Ordering::Relaxed);
                self.stats.rejected.fetch_add(1, Ordering::Relaxed);
                Err(err)
            }
        }
    }

    pub fn snapshot(&self) -> TaskPoolSnapshot {
        TaskPoolSnapshot {
            queued: self.stats.queued.load(Ordering::Relaxed),
            active: self.stats.active.load(Ordering::Relaxed),
            submitted: self.stats.submitted.load(Ordering::Relaxed),
            completed: self.stats.completed.load(Ordering::Relaxed),
            rejected: self.stats.rejected.load(Ordering::Relaxed),
        }
    }
}
