use self::attachments::UploadTask;
use crate::callback::Callback;
use crate::models::Attachment;
use crate::storage::Storage;
use crate::utils::{elapsed, now_millis};
use crate::{
    callback::MessageCallback,
    request::{ChatRequest, ChatRequestType},
    MAX_RETRIES, MAX_SEND_IDLE_SECS,
};
use std::sync::atomic::AtomicI64;
use std::{
    collections::{HashMap, VecDeque},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};
use tokio::sync::mpsc::UnboundedSender;

mod attachments;
mod conversations;
mod requests;
mod users;

pub fn is_cache_expired(cached_at: i64, expire_secs: i64) -> bool {
    (now_millis() - cached_at) / 1000 > expire_secs
}

pub struct PendingRequest {
    pub callback: Option<Box<dyn MessageCallback>>,
    pub req: ChatRequest,
    pub retry: AtomicUsize,
    pub updated_at: i64,
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
            updated_at: now_millis(),
            last_fail_at: AtomicI64::new(0),
        }
    }

    pub fn is_expired(&self) -> bool {
        if !self.can_retry {
            return true;
        }
        let retry_count = self.retry.load(Ordering::Relaxed);
        retry_count >= MAX_RETRIES || elapsed(self.updated_at).as_secs() > MAX_SEND_IDLE_SECS
    }

    pub fn did_retry(&self) {
        self.retry.fetch_add(1, Ordering::Relaxed);
        self.last_fail_at.store(now_millis(), Ordering::Relaxed);
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
}

type PendingRequests = Arc<Mutex<HashMap<String, PendingRequest>>>;

pub(super) type ClientStoreRef = Arc<ClientStore>;
pub(super) type CallbackRef = Arc<Mutex<Option<Box<dyn Callback>>>>;
pub(super) struct ClientStore {
    user_id: String,
    endpoint: String,
    token: String,
    tmps: Mutex<VecDeque<String>>,
    outgoings: PendingRequests,
    upload_tasks: Mutex<HashMap<String, Arc<UploadTask>>>,
    msg_tx: Mutex<Option<UnboundedSender<String>>>,
    pub(crate) message_storage: Arc<Storage>,
    pub(crate) callback: CallbackRef,
}

impl ClientStore {
    pub fn new(
        _root_path: &str,
        db_path: &str,
        endpoint: &str,
        token: &str,
        user_id: &str,
    ) -> Self {
        Self {
            user_id: user_id.to_string(),
            endpoint: endpoint.to_string(),
            token: token.to_string(),
            tmps: Mutex::new(VecDeque::new()),
            outgoings: Arc::new(Mutex::new(HashMap::new())),
            upload_tasks: Mutex::new(HashMap::new()),
            msg_tx: Mutex::new(None),
            message_storage: Arc::new(Storage::new(db_path)),
            callback: Arc::new(Mutex::new(None)),
        }
    }

    pub fn new_with_storage(
        _root_path: &str,
        endpoint: &str,
        token: &str,
        user_id: &str,
        message_storage: Arc<Storage>,
    ) -> Self {
        Self {
            user_id: user_id.to_string(),
            endpoint: endpoint.to_string(),
            token: token.to_string(),
            tmps: Mutex::new(VecDeque::new()),
            outgoings: Arc::new(Mutex::new(HashMap::new())),
            upload_tasks: Mutex::new(HashMap::new()),
            msg_tx: Mutex::new(None),
            message_storage,
            callback: Arc::new(Mutex::new(None)),
        }
    }

    pub(crate) fn process_timeout_requests(&self) {
        if self.outgoings.lock().unwrap().len() == 0 {
            return;
        }

        let outgoings_ref = self.outgoings.clone();
        let mut outgoings = outgoings_ref.lock().unwrap();
        let mut expired = Vec::new();
        let now = now_millis();

        for (chat_id, pending) in outgoings.iter() {
            if pending.is_expired() {
                expired.push(chat_id.clone());
            } else {
                if pending.need_retry(now) {
                    self.try_send(chat_id.clone());
                }
            }
        }

        for chat_id in expired {
            if let Some(pending) = outgoings.remove(&chat_id) {
                pending
                    .callback
                    .map(|cb| cb.on_fail("send expired".to_string()));
            }
        }
    }
    pub fn shutdown(&self) {}
}
