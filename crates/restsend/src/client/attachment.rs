use super::Client;
use crate::{
    callback::UploadCallback,
    services::media::upload_file,
    services::{media::build_upload_url, response::Upload},
};
use anyhow::{Error, Result};
use std::{collections::HashMap, sync::Mutex};
use tokio::sync::oneshot;

struct UploadCallbackImpl {}
impl UploadCallback for UploadCallbackImpl {
    fn on_progress(&self, _progress: u64, _total: u64) {}
    fn on_success(&self, _url: String) {}
    fn on_fail(&self, _e: Error) {}
}

pub(crate) fn default_upload_callback() -> Box<dyn UploadCallback> {
    Box::new(UploadCallbackImpl {})
}

pub(super) struct AttachmentInner {
    pub(super) pending: Mutex<HashMap<String, oneshot::Sender<()>>>,
}

impl AttachmentInner {
    pub(super) fn new() -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
        }
    }
    pub(super) fn push(&self, key: &str, cancel_tx: oneshot::Sender<()>) {
        self.pending
            .lock()
            .unwrap()
            .insert(key.to_string(), cancel_tx);
    }

    pub(super) fn cancel(&self, key: &str) {
        self.pending.lock().unwrap().remove(key);
    }
}

// impl AttachmentInner {
//     pub(crate) async fn upload_attachment(
//         &self,
//         endpoint: &str,
//         token: &str,
//         attachment: Attachment,
//         callback: Option<Box<dyn UploadCallback>>,
//     ) -> Result<Upload> {
//         let uploader = build_upload_url(&endpoint, "");
//         let (cancel_tx, cancel_rx) = oneshot::channel();

//         if !attachment.key.is_empty() {
//             self.push(&attachment.key, cancel_tx);
//         }

//         let r = upload_file(
//             uploader,
//             Some(&token),
//             attachment.file_path,
//             attachment.is_private,
//             callback.unwrap_or(default_upload_callback()),
//             cancel_rx,
//         )
//         .await;

//         self.cancel(&attachment.key);

//         r.map(|r| r.unwrap_or_default())
//     }
// }

// impl Client {
//     pub async fn cancel_upload(&self, key: &str) {
//         self.attachment_inner.cancel(key);
//     }
// }
