use super::{ClientStore, PendingRequest};
use crate::models::Content;
use crate::utils::elapsed;
use crate::Error;
use crate::{
    callback::UploadCallback,
    media::{build_upload_url, upload_file},
    services::response::Upload,
    utils::now_millis,
    MEDIA_PROGRESS_INTERVAL,
};
use log::{info, warn};
use std::sync::{
    atomic::{AtomicI64, Ordering},
    Arc, Mutex,
};
use tokio::sync::oneshot;

pub(super) struct UploadTask {
    req: Mutex<Option<PendingRequest>>,
    #[allow(unused)]
    cancel_tx: Mutex<Option<oneshot::Sender<()>>>,
    updated_at: AtomicI64,
    last_progress: AtomicI64,
}

impl UploadTask {
    pub fn new(req: PendingRequest, cancel_tx: oneshot::Sender<()>) -> Self {
        Self {
            req: Mutex::new(Some(req)),
            cancel_tx: Mutex::new(Some(cancel_tx)),
            updated_at: AtomicI64::new(now_millis()),
            last_progress: AtomicI64::new(0),
        }
    }
    pub fn abort(&self) {
        self.cancel_tx.lock().unwrap().take();
    }

    pub fn on_progress(&self, progress: u64, total: u64) {
        self.updated_at.store(now_millis(), Ordering::Relaxed);
        let req = &self.req.lock().unwrap();
        let last_progress = self.last_progress.load(Ordering::Relaxed);

        if elapsed(last_progress).as_millis() < MEDIA_PROGRESS_INTERVAL {
            // 300ms
            return;
        }
        self.last_progress.store(now_millis(), Ordering::Relaxed);

        if let Some(req) = req.as_ref() {
            if let Some(cb) = req.callback.as_ref() {
                cb.on_progress(progress, total);
            }
        }
    }

    pub fn on_success(&self, result: Upload) {
        let pending = self.req.lock().unwrap().take();
        let mut pending = match pending {
            Some(p) => p,
            None => {
                warn!("upload success but pending request is none");
                return;
            }
        };

        info!(
            "upload success: file: {} size: {} url: {}",
            result.file_name, result.size, result.path
        );

        let original = pending
            .req
            .content
            .clone()
            .unwrap_or(Content::new(crate::models::ContentType::File));

        let content = if let Some(cb) = pending.callback.as_ref() {
            match cb.on_attachment_upload(result.clone()) {
                Some(content) => Some(content),
                None => None,
            }
        } else {
            None
        };

        pending.req.content = match content {
            Some(content) => Some(content),
            None => {
                let mut content = original;
                content.text = result.path;
                content.size = result.size;
                content.thumbnail = result.thumbnail;
                content.placeholder = result.file_name;
                Some(content)
            }
        };
        // 囧
        self.req.lock().unwrap().replace(pending);
    }

    pub fn on_fail(&self, e: Error) {
        let req = &self.req.lock().unwrap();
        if let Some(req) = req.as_ref() {
            if let Some(cb) = req.callback.as_ref() {
                cb.on_fail(e.to_string());
            }
        }
    }
}
struct UploadTaskCallback {
    pub(super) task: Arc<UploadTask>,
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
    }

    fn on_fail(&self, e: Error) {
        self.task.on_fail(e)
    }
}

impl ClientStore {
    // upload or download media
    pub(super) async fn submit_upload(&self, req: PendingRequest) -> crate::Result<PendingRequest> {
        let (cancel_tx, cancel_rx) = oneshot::channel::<()>();
        let attachment = match req.get_attachment() {
            Some(a) => a,
            None => {
                return Err(Error::StdError("no attachment".to_string()));
            }
        };

        let task = Arc::new(UploadTask::new(req, cancel_tx));
        let task_callback = Box::new(UploadTaskCallback { task: task.clone() });

        let endpoint = self.endpoint.to_string();
        let token = self.token.to_string();

        let uploader = build_upload_url(&endpoint, "");
        //TODO: retry
        match upload_file(uploader, Some(&token), attachment, task_callback, cancel_rx).await {
            Err(e) => Err(e),
            _ => Ok(task.req.lock().unwrap().take().unwrap()),
        }
    }
}
