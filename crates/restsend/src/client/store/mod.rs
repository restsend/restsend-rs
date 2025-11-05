use self::attachments::UploadTask;
use crate::callback::{CountableCallback, RsCallback};
use crate::models::Attachment;
use crate::storage::Storage;
use crate::utils::{elapsed, now_millis};
use crate::{
    callback::MessageCallback,
    request::{ChatRequest, ChatRequestType},
};

use std::collections::HashSet;
use std::sync::atomic::{AtomicI64, AtomicU64};
use std::sync::Mutex;
use std::{
    collections::{HashMap, VecDeque},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, RwLock,
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
    pub option: ClientOptionRef,
    pub callback: Option<Box<dyn MessageCallback>>,
    pub req: ChatRequest,
    pub retry: AtomicUsize,
    pub updated_at: i64,
    pub last_fail_at: AtomicI64,
    pub can_retry: bool,
}

impl PendingRequest {
    pub fn new(
        req: ChatRequest,
        callback: Option<Box<dyn MessageCallback>>,
        option: ClientOptionRef,
    ) -> Self {
        let can_retry = match ChatRequestType::from(&req.req_type) {
            ChatRequestType::Typing | ChatRequestType::Read => false,
            _ => true,
        };

        PendingRequest {
            option,
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
        retry_count >= self.option.max_retry.load(Ordering::Relaxed)
            || elapsed(self.updated_at).as_secs()
                > self.option.max_send_idle_secs.load(Ordering::Relaxed)
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

type PendingRequests = Arc<RwLock<HashMap<String, PendingRequest>>>;
pub struct ClientOption {
    pub max_retry: AtomicUsize,
    pub max_send_idle_secs: AtomicU64,
    pub max_recall_secs: AtomicUsize,
    pub max_conversation_limit: AtomicUsize,
    pub max_logs_limit: AtomicUsize,
    pub max_sync_logs_max_count: AtomicUsize,
    pub max_connect_interval_secs: AtomicUsize,
    pub max_attachment_concurrent: AtomicUsize,
    pub max_incoming_log_cache_count: AtomicUsize,
    pub max_sync_logs_limit: AtomicUsize,
    pub keepalive_interval_secs: AtomicUsize,
    pub ping_interval_secs: AtomicUsize,
    pub media_progress_interval: AtomicUsize,
    pub conversation_cache_expire_secs: AtomicUsize,
    pub user_cache_expire_secs: AtomicUsize,
    pub removed_conversation_cache_expire_secs: AtomicUsize,
    pub ping_timeout_secs: AtomicUsize,
}

impl Default for ClientOption {
    fn default() -> Self {
        Self {
            max_retry: AtomicUsize::new(2),
            max_send_idle_secs: AtomicU64::new(20),
            max_recall_secs: AtomicUsize::new(2 * 60),
            max_conversation_limit: AtomicUsize::new(1000),
            max_logs_limit: AtomicUsize::new(100),
            max_sync_logs_max_count: AtomicUsize::new(200),
            max_connect_interval_secs: AtomicUsize::new(5),
            max_attachment_concurrent: AtomicUsize::new(12),
            max_incoming_log_cache_count: AtomicUsize::new(300),
            max_sync_logs_limit: AtomicUsize::new(500),
            keepalive_interval_secs: AtomicUsize::new(50),
            ping_interval_secs: AtomicUsize::new(30),
            media_progress_interval: AtomicUsize::new(300),
            conversation_cache_expire_secs: AtomicUsize::new(60),
            user_cache_expire_secs: AtomicUsize::new(60),
            removed_conversation_cache_expire_secs: AtomicUsize::new(1),
            ping_timeout_secs: AtomicUsize::new(5),
        }
    }
}

pub type ClientOptionRef = Arc<ClientOption>;
pub type ClientStoreRef = Arc<ClientStore>;
pub(super) type CallbackRef = Arc<RwLock<Option<Box<dyn RsCallback>>>>;
pub(super) type CountableCallbackRef = Arc<RwLock<Option<Box<dyn CountableCallback>>>>;
pub struct ClientStore {
    user_id: String,
    endpoint: String,
    token: String,
    tmps: RwLock<VecDeque<String>>,
    outgoings: PendingRequests,
    upload_tasks: RwLock<HashMap<String, Arc<UploadTask>>>,
    msg_tx: RwLock<Option<UnboundedSender<String>>>,
    removed_conversations: RwLock<HashMap<String, i64>>,
    pub(crate) message_storage: Arc<Storage>,
    pub(crate) callback: CallbackRef,
    pub(crate) countable_callback: CountableCallbackRef,
    incoming_logs: RwLock<HashMap<String, Vec<String>>>,
    pending_conversations: Mutex<HashSet<String>>,
    pub option: ClientOptionRef,
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
            tmps: RwLock::new(VecDeque::new()),
            outgoings: Arc::new(RwLock::new(HashMap::new())),
            upload_tasks: RwLock::new(HashMap::new()),
            msg_tx: RwLock::new(None),
            removed_conversations: RwLock::new(HashMap::new()),
            message_storage: Arc::new(Storage::new(db_path)),
            callback: Arc::new(RwLock::new(None)),
            countable_callback: Arc::new(RwLock::new(None)),
            incoming_logs: RwLock::new(HashMap::new()),
            pending_conversations: Mutex::new(HashSet::new()),
            option: Arc::new(ClientOption::default()),
        }
    }

    pub(crate) fn process_timeout_requests(&self) {
        if self.outgoings.read().unwrap().len() == 0 {
            return;
        }

        let outgoings_ref = self.outgoings.clone();
        let mut outgoings = match outgoings_ref.try_write() {
            Ok(outgoings) => outgoings,
            Err(_) => return,
        };
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

    pub(crate) fn process_removed_conversations(&self) {
        match self.removed_conversations.try_write() {
            Ok(mut removed_conversations) => {
                removed_conversations.retain(|_, removed_at| {
                    !is_cache_expired(
                        *removed_at,
                        self.option
                            .removed_conversation_cache_expire_secs
                            .load(Ordering::Relaxed) as i64,
                    )
                });
            }
            Err(_) => {}
        }
    }
    pub fn shutdown(&self) {}
}
