use super::{ClientStore, ClientStoreRef, PendingRequest};
use crate::utils::{elapsed, spawn};
use crate::Error;
use crate::{
    callback::UploadCallback,
    media::{build_upload_url, upload_file},
    services::response::Upload,
    utils::now_millis,
    MEDIA_PROGRESS_INTERVAL,
};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicI64, Ordering},
        Arc, Mutex,
    },
};
use tokio::sync::{oneshot, Barrier};

pub(super) struct UploadTask {
    store_ref: ClientStoreRef,
    req: Mutex<Option<PendingRequest>>,
    #[allow(unused)]
    cancel_tx: oneshot::Sender<()>,
    updated_at: AtomicI64,
    last_progress: Mutex<i64>,
}

impl UploadTask {
    pub fn new(
        store_ref: ClientStoreRef,
        cancel_tx: oneshot::Sender<()>,
        req: PendingRequest,
    ) -> Self {
        Self {
            cancel_tx,
            store_ref,
            req: Mutex::new(Some(req)),
            updated_at: AtomicI64::new(now_millis()),
            last_progress: Mutex::new(now_millis()),
        }
    }

    pub fn on_progress(&self, progress: u64, total: u64) {
        self.updated_at.store(now_millis(), Ordering::Relaxed);
        if let Some(req) = self.req.lock().unwrap().as_ref() {
            let mut last_progress = self.last_progress.lock().unwrap();
            if elapsed(*last_progress).as_millis() < MEDIA_PROGRESS_INTERVAL {
                // 300ms
                return;
            }
            *last_progress = now_millis();

            req.callback.as_ref().unwrap().on_progress(progress, total);
        }
    }

    pub fn on_success(&self, result: Upload) {
        self.updated_at.store(now_millis(), Ordering::Relaxed);
        let pending = self.req.lock().unwrap().take().unwrap();
        ClientStore::on_attachment_upload_success(self.store_ref.clone(), pending, result);
    }
    pub fn on_fail(&self, e: Error) {
        if let Some(req) = self.req.lock().unwrap().take() {
            req.callback.map(|cb| cb.on_fail(e.to_string()));
        }
        self.updated_at.store(now_millis(), Ordering::Relaxed)
    }
}

struct UploadTaskCallback {
    task: Arc<UploadTask>,
}

#[cfg(target_family = "wasm")]
unsafe impl Send for UploadTaskCallback {}
#[cfg(target_family = "wasm")]
unsafe impl Sync for UploadTaskCallback {}

impl UploadCallback for UploadTaskCallback {
    fn on_progress(&self, progress: u64, total: u64) {
        self.task.on_progress(progress, total)
    }

    fn on_success(&self, result: Upload) {
        self.task.on_success(result);
        // clall store
    }

    fn on_fail(&self, e: Error) {
        self.task.on_fail(e)
    }
}
struct UploadPendingTask {
    #[allow(unused)]
    task: Arc<UploadTask>,
    #[cfg(not(target_family = "wasm"))]
    job_handle: tokio::task::JoinHandle<()>,
}

pub(super) struct AttachmentInner {
    pendings: Mutex<HashMap<String, UploadPendingTask>>,
}

impl AttachmentInner {
    pub fn new() -> Self {
        Self {
            pendings: Mutex::new(HashMap::new()),
        }
    }

    // upload or download media
    pub(super) async fn submit_upload(
        &self,
        endpoint: &str,
        token: &str,
        task: Arc<UploadTask>,
        cancel_rx: oneshot::Receiver<()>,
    ) {
        //
        let endpoint = endpoint.to_string();
        let token = token.to_string();
        let req_id = task.req.lock().unwrap().as_ref().unwrap().get_req_id();
        let attachment = task
            .req
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .get_attachment()
            .unwrap();

        let uploader = build_upload_url(&endpoint, "");

        let task_ref = task.clone();
        let barrier = Arc::new(Barrier::new(2));
        let barrier_ref = barrier.clone();

        let task_handle = spawn(async move {
            barrier_ref.wait().await;

            let media_callback = Box::new(UploadTaskCallback {
                task: task_ref.clone(),
            });

            let r = upload_file(
                uploader,
                Some(&token),
                attachment,
                media_callback,
                cancel_rx,
            )
            .await;

            match r {
                Err(e) => {
                    task_ref.on_fail(e.into());
                }
                _ => {}
            }
        });

        barrier.wait().await;

        #[cfg(target_family = "wasm")]
        let _ = task_handle;

        let t = UploadPendingTask {
            task,
            #[cfg(not(target_family = "wasm"))]
            job_handle: task_handle,
        };
        self.pendings.lock().unwrap().insert(req_id.to_string(), t);
    }

    pub(super) fn cancel_send(&self, req_id: &str) {
        let mut pendings = self.pendings.lock().unwrap();
        if let Some(pending) = pendings.remove(req_id) {
            #[cfg(not(target_family = "wasm"))]
            pending.job_handle.abort();
        }
    }
}
