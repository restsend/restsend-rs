use crate::models::{Attachment, ChatLogStatus};
use crate::storage::Storage;
use crate::MEDIA_PROGRESS_INTERVAL;
use crate::{
    callback::MessageCallback,
    request::{ChatRequest, ChatRequestType},
    MAX_ATTACHMENT_CONCURRENT, MAX_RETRIES, MAX_SEND_IDLE_SECS,
};
use log::{debug, info, warn};
use std::{
    collections::{HashMap, VecDeque},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Instant,
};
use tokio::select;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use self::attachments::{AttachmentInner, UploadEvent};

mod attachments;
mod requests;

pub struct PendingRequest {
    pub callback: Option<Box<dyn MessageCallback>>,
    pub req: ChatRequest,
    pub retry: AtomicUsize,
    pub updated_at: Instant,
    pub can_retry: bool,
}

impl PendingRequest {
    pub fn new(req: ChatRequest, callback: Option<Box<dyn MessageCallback>>) -> Self {
        let can_retry = match ChatRequestType::from(&req.r#type) {
            ChatRequestType::Typing | ChatRequestType::Read => false,
            _ => true,
        };

        PendingRequest {
            callback,
            req,
            retry: AtomicUsize::new(0),
            can_retry,
            updated_at: Instant::now(),
        }
    }

    pub fn is_expired(&self) -> bool {
        if !self.can_retry {
            return true;
        }
        let retry_count = self.retry.load(Ordering::Relaxed);
        retry_count >= MAX_RETRIES || self.updated_at.elapsed().as_secs() > MAX_SEND_IDLE_SECS
    }

    pub fn did_retry(&self) {
        self.retry.fetch_add(1, Ordering::Relaxed);
    }

    pub fn has_attachment(&self) -> bool {
        self.req
            .content
            .as_ref()
            .map(|c| c.attachment.is_some())
            .unwrap_or(false)
    }

    pub fn get_attachment(&self) -> Option<Attachment> {
        self.req.content.as_ref().and_then(|c| c.attachment.clone())
    }

    pub fn get_req_id(&self) -> String {
        self.req.id.clone()
    }
    pub fn get_topic_id(&self) -> String {
        self.req.topic_id.clone()
    }
    pub fn get_chat_id(&self) -> String {
        self.req.chat_id.clone()
    }
}

type PendingRequests = Mutex<HashMap<String, PendingRequest>>;

pub(super) type ClientStoreRef = Arc<ClientStore>;
pub(super) struct ClientStore {
    endpoint: String,
    token: String,
    tmps: Mutex<VecDeque<String>>,
    outgoings: PendingRequests,
    msg_tx: Mutex<Option<UnboundedSender<String>>>,
    upload_result_tx: Mutex<Option<UnboundedSender<UploadEvent>>>,
    message_storage: Storage,
    attachment_inner: AttachmentInner,
}

impl ClientStore {
    pub fn new(root_path: &str, db_path: &str, endpoint: &str, token: &str) -> Self {
        Self {
            endpoint: endpoint.to_string(),
            token: token.to_string(),
            tmps: Mutex::new(VecDeque::new()),
            outgoings: Mutex::new(HashMap::new()),
            msg_tx: Mutex::new(None),
            message_storage: Storage::new(db_path),
            attachment_inner: AttachmentInner::new(),
            upload_result_tx: Mutex::new(None),
        }
    }

    pub async fn process(&self) {
        let (upload_result_tx, mut upload_result_rx) = mpsc::unbounded_channel();
        self.upload_result_tx
            .lock()
            .unwrap()
            .replace(upload_result_tx);

        let upload_result_loop = async move {
            loop {
                while let Some(event) = upload_result_rx.recv().await {
                    match event {
                        UploadEvent::Success(mut pending, result) => {
                            info!(
                                "upload success: file:{} url:{}",
                                result.file_name, result.path
                            );

                            let content = pending.req.content.as_mut().unwrap();
                            content.attachment.take();

                            content.text = result.path;
                            content.size = result.size;
                            content.thumbnail = result.thumbnail;
                            content.placeholder = result.file_name;

                            let topic_id = pending.req.topic_id.clone();
                            let chat_id = pending.req.chat_id.clone();

                            // update database status
                            if let Err(e) = self
                                .update_outoing_chat_log_state(
                                    &topic_id,
                                    &chat_id,
                                    ChatLogStatus::Sending,
                                )
                                .await
                            {
                                warn!("update_message_content failed: {}", e);
                            }
                            // requeue to send
                            self.add_pending_request(pending.req, pending.callback)
                                .await;
                        }
                        UploadEvent::Err(req, e) => {
                            info!("upload failed: {}", e.to_string());
                            req.callback.map(|cb| cb.on_fail(e.to_string()));
                        }
                        UploadEvent::Progress(topic_id, chat_id, progress, total) => {
                            debug!(
                                "upload progress: topic_id:{} chat_id:{} progress:{} total:{}",
                                topic_id, chat_id, progress, total
                            );
                        }
                    }
                }
            }
        };

        select! {
            _ = upload_result_loop => {
            },
        }
    }
    pub async fn shutdown(&self) {}
}
