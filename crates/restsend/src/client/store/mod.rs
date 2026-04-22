use self::attachments::UploadTask;
use crate::callback::{CountableCallback, RsCallback, SyncChatLogsCallback};
use crate::models::Attachment;
use crate::models::{ChatLog, GetChatLogsResult};
use crate::storage::Storage;
use crate::utils::{elapsed, now_millis};
use crate::{
    callback::MessageCallback,
    request::{ChatRequest, ChatRequestType},
};

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64};
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

const QUICK_SYNC_WAITERS_TTL_MS: i64 = 30_000;
const QUICK_SYNC_WAITERS_MAX_IN_FLIGHT: usize = 512;

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
    pub topic_owner_cache_expire_secs: AtomicUsize,
    pub removed_conversation_cache_expire_secs: AtomicUsize,
    pub ping_timeout_secs: AtomicUsize,
    pub build_local_unreadable: AtomicBool,
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
            topic_owner_cache_expire_secs: AtomicUsize::new(5 * 60),
            removed_conversation_cache_expire_secs: AtomicUsize::new(5), // 5 seconds
            ping_timeout_secs: AtomicUsize::new(5),
            build_local_unreadable: AtomicBool::new(false),
        }
    }
}

pub type ClientOptionRef = Arc<ClientOption>;
pub type ClientStoreRef = Arc<ClientStore>;
pub(super) type CallbackRef = Arc<RwLock<Option<Box<dyn RsCallback>>>>;
pub(super) type CountableCallbackRef = Arc<RwLock<Option<Box<dyn CountableCallback>>>>;
pub(super) enum QuickSyncSingleflightState {
    Leader,
    Joined,
    Rejected,
}

struct QuickSyncWaiterEntry {
    callbacks: Vec<Box<dyn SyncChatLogsCallback>>,
    created_at: i64,
}

pub(super) struct RecentChatLogsCacheEntry {
    pub items: Vec<ChatLog>,
    pub has_more: bool,
    pub limit: u32,
    pub need_fetch: bool,
    pub cached_at: i64,
}

pub struct ClientStore {
    user_id: String,
    endpoint: String,
    token: String,
    tmps: RwLock<VecDeque<String>>,
    outgoings: PendingRequests,
    upload_tasks: RwLock<HashMap<String, Arc<UploadTask>>>,
    msg_tx: RwLock<Option<UnboundedSender<String>>>,
    msg_direct_tx: RwLock<Option<UnboundedSender<ChatRequest>>>,
    removed_conversations: RwLock<HashMap<String, (i64 /* removed_at */, i64 /* seq */)>>,
    pub(crate) message_storage: Arc<Storage>,
    pub(crate) callback: CallbackRef,
    pub(crate) countable_callback: CountableCallbackRef,
    incoming_logs: RwLock<HashMap<String, Vec<String>>>,
    recent_chat_logs: RwLock<HashMap<String, RecentChatLogsCacheEntry>>,
    quick_sync_waiters: Mutex<HashMap<String, QuickSyncWaiterEntry>>,
    quick_sync_last_fetch_at: RwLock<HashMap<String, i64>>,
    pending_conversations: Mutex<HashSet<String>>,
    topic_owner_cache: RwLock<HashMap<String, (String, i64)>>,
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
            msg_direct_tx: RwLock::new(None),
            removed_conversations: RwLock::new(HashMap::new()),
            message_storage: Arc::new(Storage::new(db_path)),
            callback: Arc::new(RwLock::new(None)),
            countable_callback: Arc::new(RwLock::new(None)),
            incoming_logs: RwLock::new(HashMap::new()),
            recent_chat_logs: RwLock::new(HashMap::new()),
            quick_sync_waiters: Mutex::new(HashMap::new()),
            quick_sync_last_fetch_at: RwLock::new(HashMap::new()),
            pending_conversations: Mutex::new(HashSet::new()),
            topic_owner_cache: RwLock::new(HashMap::new()),
            option: Arc::new(ClientOption::default()),
        }
    }

    pub(super) fn begin_quick_sync_singleflight(
        &self,
        key: String,
        callback: Box<dyn SyncChatLogsCallback>,
    ) -> QuickSyncSingleflightState {
        self.begin_quick_sync_singleflight_at(key, callback, now_millis())
    }

    fn begin_quick_sync_singleflight_at(
        &self,
        key: String,
        callback: Box<dyn SyncChatLogsCallback>,
        now: i64,
    ) -> QuickSyncSingleflightState {
        let mut expired_callbacks = Vec::new();
        let mut rejected_callback = None;

        let state = {
            let mut waiters = self.quick_sync_waiters.lock().unwrap();

            let expired_keys: Vec<String> = waiters
                .iter()
                .filter_map(|(k, entry)| {
                    if now - entry.created_at > QUICK_SYNC_WAITERS_TTL_MS {
                        Some(k.clone())
                    } else {
                        None
                    }
                })
                .collect();

            for expired_key in expired_keys {
                if let Some(entry) = waiters.remove(&expired_key) {
                    expired_callbacks.extend(entry.callbacks);
                }
            }

            match waiters.get_mut(&key) {
                Some(entry) => {
                    entry.callbacks.push(callback);
                    QuickSyncSingleflightState::Joined
                }
                None => {
                    if waiters.len() >= QUICK_SYNC_WAITERS_MAX_IN_FLIGHT {
                        rejected_callback = Some(callback);
                        QuickSyncSingleflightState::Rejected
                    } else {
                        waiters.insert(
                            key,
                            QuickSyncWaiterEntry {
                                callbacks: vec![callback],
                                created_at: now,
                            },
                        );
                        QuickSyncSingleflightState::Leader
                    }
                }
            }
        };

        for callback in expired_callbacks {
            callback.on_fail(crate::Error::Other(
                "quick sync singleflight waiter expired".to_string(),
            ));
        }

        if let Some(callback) = rejected_callback {
            callback.on_fail(crate::Error::Other(
                "quick sync singleflight inflight overflow".to_string(),
            ));
        }

        state
    }

    fn cleanup_expired_quick_sync_waiters(&self, now: i64) {
        let expired_callbacks = {
            let mut waiters = match self.quick_sync_waiters.try_lock() {
                Ok(waiters) => waiters,
                Err(_) => return,
            };

            let expired_keys: Vec<String> = waiters
                .iter()
                .filter_map(|(k, entry)| {
                    if now - entry.created_at > QUICK_SYNC_WAITERS_TTL_MS {
                        Some(k.clone())
                    } else {
                        None
                    }
                })
                .collect();

            let mut expired_callbacks = Vec::new();
            for expired_key in expired_keys {
                if let Some(entry) = waiters.remove(&expired_key) {
                    expired_callbacks.extend(entry.callbacks);
                }
            }
            expired_callbacks
        };

        for callback in expired_callbacks {
            callback.on_fail(crate::Error::Other(
                "quick sync singleflight waiter expired".to_string(),
            ));
        }
    }

    pub(super) fn finish_quick_sync_singleflight_success(
        &self,
        key: &str,
        result: GetChatLogsResult,
    ) {
        let callbacks = self
            .quick_sync_waiters
            .lock()
            .unwrap()
            .remove(key)
            .map(|entry| entry.callbacks)
            .unwrap_or_default();
        for callback in callbacks {
            callback.on_success(result.clone());
        }
    }

    pub(super) fn finish_quick_sync_singleflight_fail(&self, key: &str, e: crate::Error) {
        let callbacks = self
            .quick_sync_waiters
            .lock()
            .unwrap()
            .remove(key)
            .map(|entry| entry.callbacks)
            .unwrap_or_default();
        for callback in callbacks {
            callback.on_fail(e.clone());
        }
    }

    pub(super) fn should_throttle_quick_sync_fetch(
        &self,
        topic_id: &str,
        now: i64,
        throttle_ms: i64,
    ) -> bool {
        let mut fetch_at = match self.quick_sync_last_fetch_at.try_write() {
            Ok(fetch_at) => fetch_at,
            Err(_) => return false,
        };

        if fetch_at.len() >= 256 {
            if let Some(oldest_key) = fetch_at
                .iter()
                .min_by_key(|(_, ts)| *ts)
                .map(|(topic, _)| topic.clone())
            {
                fetch_at.remove(&oldest_key);
            }
        }

        if let Some(last_fetch_at) = fetch_at.get(topic_id).copied() {
            if now - last_fetch_at <= throttle_ms {
                return true;
            }
        }

        fetch_at.insert(topic_id.to_string(), now);
        false
    }

    pub(crate) fn process_timeout_requests(&self) {
        self.cleanup_expired_quick_sync_waiters(now_millis());

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
                removed_conversations.retain(|_, (removed_at, _)| {
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

#[cfg(test)]
mod tests {
    use super::{ClientStore, QuickSyncSingleflightState};
    use crate::{callback::SyncChatLogsCallback, models::GetChatLogsResult};
    use std::sync::{Arc, Mutex};

    #[derive(Default, Clone)]
    struct Counter {
        fail_count: Arc<Mutex<u32>>,
    }

    impl Counter {
        fn inc_fail(&self) {
            let mut fail_count = self.fail_count.lock().unwrap();
            *fail_count += 1;
        }

        fn fail_count(&self) -> u32 {
            *self.fail_count.lock().unwrap()
        }
    }

    struct TestSyncCallback {
        counter: Counter,
    }

    impl SyncChatLogsCallback for TestSyncCallback {
        fn on_success(&self, _r: GetChatLogsResult) {}

        fn on_fail(&self, _e: crate::Error) {
            self.counter.inc_fail();
        }
    }

    #[test]
    fn quick_sync_singleflight_rejects_when_overflow() {
        let store = ClientStore::new("", ":memory:", "http://test", "token", "u1");
        let base = 1_000_000;

        for i in 0..512 {
            let state = store.begin_quick_sync_singleflight_at(
                format!("key-{i}"),
                Box::new(TestSyncCallback {
                    counter: Counter::default(),
                }),
                base,
            );
            assert!(matches!(state, QuickSyncSingleflightState::Leader));
        }

        let rejected_counter = Counter::default();
        let state = store.begin_quick_sync_singleflight_at(
            "key-overflow".to_string(),
            Box::new(TestSyncCallback {
                counter: rejected_counter.clone(),
            }),
            base,
        );

        assert!(matches!(state, QuickSyncSingleflightState::Rejected));
        assert_eq!(rejected_counter.fail_count(), 1);
    }

    #[test]
    fn quick_sync_singleflight_cleans_expired_waiters() {
        let store = ClientStore::new("", ":memory:", "http://test", "token", "u1");
        let first_counter = Counter::default();
        let base = 2_000_000;

        let first = store.begin_quick_sync_singleflight_at(
            "topic|None|50|false".to_string(),
            Box::new(TestSyncCallback {
                counter: first_counter.clone(),
            }),
            base,
        );
        assert!(matches!(first, QuickSyncSingleflightState::Leader));

        let second_counter = Counter::default();
        let second = store.begin_quick_sync_singleflight_at(
            "topic|None|50|false".to_string(),
            Box::new(TestSyncCallback {
                counter: second_counter.clone(),
            }),
            base + 30_001,
        );

        assert!(matches!(second, QuickSyncSingleflightState::Leader));
        assert_eq!(first_counter.fail_count(), 1);
        assert_eq!(second_counter.fail_count(), 0);
    }
}
