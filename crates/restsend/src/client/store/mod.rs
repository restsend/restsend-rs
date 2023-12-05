use crate::callback::Callback;
use crate::models::{Attachment, ChatLogStatus, Conversation, User};
use crate::services::response::Upload;
use crate::storage::{prepare, Storage};
use crate::utils::now_timestamp;
use crate::{
    callback::MessageCallback,
    request::{ChatRequest, ChatRequestType},
    MAX_RETRIES, MAX_SEND_IDLE_SECS,
};
use anyhow::{Error, Result};
use log::{debug, info, warn};
use std::sync::atomic::AtomicI64;
use std::time::Duration;
use std::{
    collections::{HashMap, VecDeque},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Instant,
};
use tokio::select;
use tokio::sync::mpsc::{self, UnboundedSender};
use tokio::time::sleep;

use self::attachments::AttachmentInner;

pub(super) enum StoreEvent {
    UploadSuccess(PendingRequest, Upload),
    UploadProgress(String, String, String, u64, u64), // req_id, topic_id, chat_id, progress, total
    PendingErr(PendingRequest, Error),
    Ack(ChatLogStatus, ChatRequest),
    SendFail(String),    // req_id
    SendSuccess(String), // req_id
    ProcessRetry,
    UpdateConversation(Conversation),
    UpdateUser(User),
}

mod attachments;
mod conversations;
mod requests;
mod users;

pub struct PendingRequest {
    pub callback: Option<Box<dyn MessageCallback>>,
    pub req: ChatRequest,
    pub retry: AtomicUsize,
    pub updated_at: Instant,
    pub last_fail_at: AtomicI64,
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
            last_fail_at: AtomicI64::new(0),
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
        self.last_fail_at.store(now_timestamp(), Ordering::Relaxed);
    }

    pub fn need_retry(&self, now: i64) -> bool {
        if !self.can_retry {
            return false;
        }

        let last_fail_at = self.last_fail_at.load(Ordering::Relaxed);
        if last_fail_at > 0 && now - last_fail_at >= 1 {
            self.last_fail_at.store(0, Ordering::Relaxed);
            true
        } else {
            false
        }
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
    event_tx: Mutex<Option<UnboundedSender<StoreEvent>>>,
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
            event_tx: Mutex::new(None),
        }
    }

    pub(super) fn migrate(&self) -> Result<()> {
        prepare(&self.message_storage)
    }

    pub async fn process(&self, callback: Arc<Box<dyn Callback>>) {
        let (response_tx, mut response_rx) = mpsc::unbounded_channel();
        let response_tx_clone = response_tx.clone();
        self.event_tx.lock().unwrap().replace(response_tx);

        let upload_result_loop = async move {
            loop {
                while let Some(event) = response_rx.recv().await {
                    match event {
                        StoreEvent::UploadSuccess(mut pending, result) => {
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
                        StoreEvent::PendingErr(req, e) => {
                            info!("upload failed: {}", e.to_string());
                            req.callback.map(|cb| cb.on_fail(e.to_string()));
                        }
                        StoreEvent::UploadProgress(req_id, topic_id, chat_id, progress, total) => {
                            debug!(
                                "upload progress req_id: {} topic_id:{} chat_id:{} progress:{} total:{}",
                                req_id, topic_id, chat_id, progress, total
                            );
                        }
                        StoreEvent::Ack(status, req) => {
                            if let Some(pending) = self.peek_pending_request(&req.id).await {
                                match status {
                                    ChatLogStatus::Sent => {
                                        pending.callback.map(|cb| cb.on_ack(req));
                                    }
                                    ChatLogStatus::SendFailed(_) => {
                                        let reason = req
                                            .message
                                            .unwrap_or(format!("send failed: {:?}", status));
                                        pending.callback.map(|cb| cb.on_fail(reason));
                                    }
                                    _ => {}
                                }
                            }
                        }
                        StoreEvent::SendFail(req_id) => {
                            warn!("send fail: {}", req_id);
                            let peek = if let Some(pending) =
                                self.outgoings.lock().unwrap().get(&req_id)
                            {
                                pending.did_retry();
                                pending.is_expired()
                            } else {
                                false
                            };

                            if peek {
                                if let Some(pending) = self.peek_pending_request(&req_id).await {
                                    pending
                                        .callback
                                        .map(|cb| cb.on_fail("request timeout".to_string()));
                                }
                            }
                        }

                        StoreEvent::SendSuccess(req_id) => {
                            debug!("send success: {}", req_id);
                        }

                        StoreEvent::ProcessRetry => {
                            let mut outgoings = self.outgoings.lock().unwrap();
                            let mut expired = Vec::new();
                            let now = now_timestamp();

                            for (req_id, pending) in outgoings.iter() {
                                if pending.is_expired() {
                                    expired.push(req_id.clone());
                                } else {
                                    if pending.need_retry(now) {
                                        debug!("retry send: {}", req_id);
                                        self.try_send(req_id.clone());
                                    }
                                }
                            }

                            for req_id in expired {
                                if let Some(pending) = outgoings.remove(&req_id) {
                                    pending
                                        .callback
                                        .map(|cb| cb.on_fail("send expired".to_string()));
                                }
                            }
                        }

                        StoreEvent::UpdateConversation(conversation) => {
                            let conversation_id = conversation.topic_id.clone();
                            match self.update_conversation(conversation.clone()).await {
                                Ok(conversation) => {
                                    debug!("update_conversation success: {}", conversation_id);
                                    callback.on_conversation_updated(vec![conversation]);
                                }
                                Err(e) => {
                                    warn!(
                                        "update_conversation failed: conversation_id:{} {}",
                                        conversation_id, e
                                    );
                                }
                            }
                        }

                        StoreEvent::UpdateUser(user) => {
                            let user_id = user.user_id.clone();
                            match self.update_user(user).await {
                                Ok(_) => {
                                    debug!("update_user success: {}", user_id);
                                }
                                Err(e) => {
                                    warn!("update_user failed: user_id:{} {}", user_id, e);
                                }
                            }
                        }
                    }
                }
            }
        };

        select! {
            _ = upload_result_loop => {},
            _ = async {
                loop {
                    sleep(Duration::from_secs(1)).await;
                    response_tx_clone.send(StoreEvent::ProcessRetry).ok();
                }
            } => {}
        }
    }
    pub async fn shutdown(&self) {}
}
