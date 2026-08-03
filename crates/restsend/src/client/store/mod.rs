use self::attachments::UploadTask;
use crate::callback::{CountableCallback, RsCallback, SyncChatLogsCallback};
use crate::models::Attachment;
use crate::models::{ChatLog, Conversation, GetChatLogsResult};
use crate::storage::{ConversationRouting, Storage, StoreModel};
use crate::utils::{elapsed, now_millis, spawn_task};
use crate::{
    callback::MessageCallback,
    request::{ChatRequest, ChatRequestType},
};

use lru::LruCache;
use std::collections::HashSet;
use std::num::NonZeroUsize;
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
const MESSAGE_CLEANUP_INTERVAL_MS: i64 = 5 * 60 * 1000; // 5 minutes
const MESSAGE_TOPICS_MAX_CAPACITY: usize = 2048;
const TOPIC_OWNER_CACHE_MAX_CAPACITY: usize = 512;
const CONVERSATION_PRUNE_INTERVAL_MS: i64 = 60_000; // 1 minute
const INCOMING_LOGS_MAX_TOPICS: usize = 512;

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
        let can_retry = !matches!(
            ChatRequestType::from(&req.req_type),
            ChatRequestType::Typing | ChatRequestType::Read
        );

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
    pub max_message_retention_days: AtomicUsize,
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
            max_message_retention_days: AtomicUsize::new(30),
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
    topic_owner_cache: Mutex<LruCache<String, (String, i64)>>,
    message_topics: Mutex<LruCache<String, ()>>,
    last_message_cleanup_at: AtomicI64,
    last_conversation_prune_at: AtomicI64,
    pub(crate) syncing_conversations: AtomicBool,
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
            topic_owner_cache: Mutex::new(LruCache::new(
                NonZeroUsize::new(TOPIC_OWNER_CACHE_MAX_CAPACITY).unwrap(),
            )),
            message_topics: Mutex::new(LruCache::new(
                NonZeroUsize::new(MESSAGE_TOPICS_MAX_CAPACITY).unwrap(),
            )),
            last_message_cleanup_at: AtomicI64::new(0),
            last_conversation_prune_at: AtomicI64::new(0),
            syncing_conversations: AtomicBool::new(false),
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

        if self.outgoings.read().unwrap().is_empty() {
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
                if let Some(cb) = pending.callback {
                    cb.on_fail("send expired".to_string());
                }
            }
        }
    }

    pub(crate) fn process_removed_conversations(&self) {
        if let Ok(mut removed_conversations) = self.removed_conversations.try_write() {
            removed_conversations.retain(|_, (removed_at, _)| {
                !is_cache_expired(
                    *removed_at,
                    self.option
                        .removed_conversation_cache_expire_secs
                        .load(Ordering::Relaxed) as i64,
                )
            });
        }
    }

    /// Control whether conversations are kept in memory instead of the
    /// persistent store (IndexedDB). Defaults to in-memory.
    pub fn set_conversations_in_memory(&self, value: bool) {
        self.message_storage.set_conversations_in_memory(value);
    }

    pub(super) fn register_message_topic(&self, topic_id: &str) {
        if let Ok(mut topics) = self.message_topics.try_lock() {
            topics.put(topic_id.to_string(), ());
        }
    }

    pub(super) fn unregister_message_topic(&self, topic_id: &str) {
        if let Ok(mut topics) = self.message_topics.try_lock() {
            topics.pop(topic_id);
        }
    }

    async fn collect_message_topics(&self) -> Vec<String> {
        let mut topics: HashSet<String> = match self.message_topics.try_lock() {
            Ok(mut topics) => {
                let mut keys = HashSet::new();
                for (k, _) in topics.iter() {
                    keys.insert(k.clone());
                }
                keys
            }
            Err(_) => HashSet::new(),
        };
        // Also union conversations (kept in memory by default), so messages of
        // conversations synced in this session are always covered even if no
        // new message was saved yet.
        if let Ok(t) = self.message_storage.readonly_table::<Conversation>().await {
            if let Some(items) = t.filter("", Box::new(|c| Some(c)), None, None).await {
                for conversation in items {
                    topics.insert(conversation.topic_id);
                }
            }
        }
        topics.into_iter().collect()
    }

    /// Remove locally stored messages older than `max_message_retention_days`.
    /// When the option is `0`, cleanup is disabled. Messages whose `created_at`
    /// cannot be parsed are kept.
    pub async fn cleanup_old_messages(&self) {
        let retention_days = self
            .option
            .max_message_retention_days
            .load(Ordering::Relaxed) as i64;
        if retention_days <= 0 {
            return;
        }
        let cutoff = now_millis() - retention_days * 86_400_000;

        let topics = self.collect_message_topics().await;
        if topics.is_empty() {
            return;
        }

        let log_t = match self.message_storage.readonly_table::<ChatLog>().await {
            Ok(t) => t,
            Err(_) => return,
        };
        let mut expired = Vec::new();
        for topic_id in &topics {
            let items = match log_t
                .filter(
                    topic_id,
                    Box::new(move |log| {
                        let created_ms =
                            chrono::DateTime::parse_from_rfc3339(&log.created_at)
                                .map(|v| v.timestamp_millis())
                                .unwrap_or(i64::MAX);
                        if created_ms < cutoff {
                            Some(log)
                        } else {
                            None
                        }
                    }),
                    None,
                    None,
                )
                .await
            {
                Some(items) => items,
                None => continue,
            };
            for log in items {
                expired.push((topic_id.clone(), log.id));
            }
        }

        if expired.is_empty() {
            return;
        }
        if let Ok(log_t) = self.message_storage.table::<ChatLog>().await {
            for (topic_id, chat_id) in &expired {
                log_t.remove(topic_id, chat_id).await.ok();
            }
        }
        log::info!(
            "cleanup_old_messages removed {} expired messages, retention_days: {}",
            expired.len(),
            retention_days
        );
    }

    /// Throttled entry point for the background cleanup loop. Checks the
    /// retention setting and a 5-minute throttle, then spawns the async
    /// cleanup. Call this from the connection keepalive loop.
    pub fn maybe_cleanup_messages(self: &Arc<Self>) {
        let retention_days = self
            .option
            .max_message_retention_days
            .load(Ordering::Relaxed) as i64;
        if retention_days <= 0 {
            return;
        }
        let now = now_millis();
        let last = self.last_message_cleanup_at.load(Ordering::Relaxed);
        if now - last < MESSAGE_CLEANUP_INTERVAL_MS {
            return;
        }
        if self
            .last_message_cleanup_at
            .compare_exchange(last, now, Ordering::Relaxed, Ordering::Relaxed)
            .is_err()
        {
            return;
        }
        let this = self.clone();
        spawn_task(async move {
            this.cleanup_old_messages().await;
        });
    }

    /// Keep the locally stored conversation list bounded to
    /// `max_conversation_limit` (most recently updated wins). When pruning a
    /// conversation, its local messages are cleared as well so old data does
    /// not linger in IndexedDB.
    pub async fn prune_conversations(&self) {
        let limit = self
            .option
            .max_conversation_limit
            .load(Ordering::Relaxed) as usize;
        if limit == 0 {
            return;
        }
        let t = match self.message_storage.readonly_table::<Conversation>().await {
            Ok(t) => t,
            Err(_) => return,
        };
        let all = match t
            .filter("", Box::new(|c| Some(c)), None, None)
            .await
        {
            Some(items) => items,
            None => return,
        };
        if all.len() <= limit {
            return;
        }
        let excess = all.len() - limit;
        let mut sorted = all;
        sorted.sort_by_key(|c| c.sort_key());
        for conversation in sorted.iter().take(excess) {
            self.clear_conversation(&conversation.topic_id).await.ok();
        }
        log::info!(
            "prune_conversations removed {} conversations over local limit {}",
            excess,
            limit
        );
    }

    /// Throttled entry point for the background conversation pruning loop.
    /// Call this from the connection keepalive loop.
    pub fn maybe_prune_conversations(self: &Arc<Self>) {
        let now = now_millis();
        let last = self.last_conversation_prune_at.load(Ordering::Relaxed);
        if now - last < CONVERSATION_PRUNE_INTERVAL_MS {
            return;
        }
        if self
            .last_conversation_prune_at
            .compare_exchange(last, now, Ordering::Relaxed, Ordering::Relaxed)
            .is_err()
        {
            return;
        }
        let this = self.clone();
        spawn_task(async move {
            this.prune_conversations().await;
        });
    }

    pub fn shutdown(&self) {
        if let Ok(mut topics) = self.message_topics.try_lock() {
            topics.clear();
        }
        if let Ok(mut owners) = self.topic_owner_cache.try_lock() {
            owners.clear();
        }
        if let Ok(mut cache) = self.recent_chat_logs.try_write() {
            cache.clear();
        }
        if let Ok(mut logs) = self.incoming_logs.try_write() {
            logs.clear();
        }
        if let Ok(mut removed) = self.removed_conversations.try_write() {
            removed.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ClientStore, QuickSyncSingleflightState, MESSAGE_TOPICS_MAX_CAPACITY,
        TOPIC_OWNER_CACHE_MAX_CAPACITY,
    };
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

    fn make_log(topic_id: &str, id: &str, seq: i64, created_at: &str) -> crate::models::ChatLog {
        crate::models::ChatLog {
            id: id.to_string(),
            topic_id: topic_id.to_string(),
            seq,
            created_at: created_at.to_string(),
            content: crate::models::Content {
                content_type: "text".to_string(),
                text: "hello".to_string(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn rfc3339_at_millis(ms: i64) -> String {
        chrono::DateTime::from_timestamp_millis(ms)
            .unwrap()
            .to_rfc3339()
    }

    #[tokio::test]
    async fn cleanup_old_messages_removes_expired_keeps_recent() {
        let store = Arc::new(ClientStore::new("", ":memory:", "http://test", "token", "u1"));
        store
            .option
            .max_message_retention_days
            .store(30, std::sync::atomic::Ordering::Relaxed);

        let now = crate::utils::now_millis();
        let day = 86_400_000;
        let old_at = rfc3339_at_millis(now - 40 * day);
        let recent_at = rfc3339_at_millis(now - day);

        let log_t = store.message_storage.table::<crate::models::ChatLog>().await.unwrap();
        log_t
            .set("t1", "old-1", Some(&make_log("t1", "old-1", 1, &old_at)))
            .await
            .unwrap();
        log_t
            .set("t1", "new-1", Some(&make_log("t1", "new-1", 2, &recent_at)))
            .await
            .unwrap();
        log_t
            .set("t2", "old-2", Some(&make_log("t2", "old-2", 1, &old_at)))
            .await
            .unwrap();
        // t3 has no conversation record; discovered only via register_message_topic
        log_t
            .set("t3", "old-3", Some(&make_log("t3", "old-3", 1, &old_at)))
            .await
            .unwrap();

        let conv_t = store.message_storage.table::<crate::models::Conversation>().await.unwrap();
        conv_t
            .set("", "t1", Some(&crate::models::Conversation::new("t1")))
            .await
            .unwrap();
        conv_t
            .set("", "t2", Some(&crate::models::Conversation::new("t2")))
            .await
            .unwrap();
        store.register_message_topic("t3");

        store.cleanup_old_messages().await;

        let log_t = store.message_storage.table::<crate::models::ChatLog>().await.unwrap();
        assert!(log_t.get("t1", "old-1").await.is_none(), "old message on t1 must be removed");
        assert!(log_t.get("t2", "old-2").await.is_none(), "old message on t2 must be removed");
        assert!(log_t.get("t3", "old-3").await.is_none(), "old message on t3 must be removed");
        assert!(log_t.get("t1", "new-1").await.is_some(), "recent message must be kept");
    }

    #[tokio::test]
    async fn cleanup_old_messages_disabled_when_retention_zero() {
        let store = Arc::new(ClientStore::new("", ":memory:", "http://test", "token", "u1"));
        store
            .option
            .max_message_retention_days
            .store(0, std::sync::atomic::Ordering::Relaxed);

        let now = crate::utils::now_millis();
        let old_at = rfc3339_at_millis(now - 100 * 86_400_000);
        let log_t = store.message_storage.table::<crate::models::ChatLog>().await.unwrap();
        log_t
            .set("t1", "old", Some(&make_log("t1", "old", 1, &old_at)))
            .await
            .unwrap();
        store.register_message_topic("t1");

        store.cleanup_old_messages().await;

        let log_t = store.message_storage.table::<crate::models::ChatLog>().await.unwrap();
        assert!(
            log_t.get("t1", "old").await.is_some(),
            "cleanup must be skipped when retention_days == 0"
        );
    }

    #[tokio::test]
    async fn maybe_cleanup_messages_skips_when_retention_zero() {
        let store = Arc::new(ClientStore::new("", ":memory:", "http://test", "token", "u1"));
        store
            .option
            .max_message_retention_days
            .store(0, std::sync::atomic::Ordering::Relaxed);
        store.maybe_cleanup_messages();
        assert_eq!(store.last_message_cleanup_at.load(std::sync::atomic::Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn maybe_cleanup_messages_updates_throttle_timestamp() {
        let store = Arc::new(ClientStore::new("", ":memory:", "http://test", "token", "u1"));
        store
            .option
            .max_message_retention_days
            .store(30, std::sync::atomic::Ordering::Relaxed);
        store.maybe_cleanup_messages();
        assert!(
            store
                .last_message_cleanup_at
                .load(std::sync::atomic::Ordering::Relaxed)
                > 0,
            "throttle timestamp must be set when cleanup is scheduled"
        );
    }

    #[test]
    fn message_topics_lru_evicts_oldest() {
        let store = ClientStore::new("", ":memory:", "http://test", "token", "u1");
        let total = MESSAGE_TOPICS_MAX_CAPACITY + 100;
        for i in 0..total {
            store.register_message_topic(&format!("t{}", i));
        }
        let mut topics = store.message_topics.try_lock().unwrap();
        assert_eq!(topics.len(), MESSAGE_TOPICS_MAX_CAPACITY, "LRU must stay bounded");
        assert!(topics.get("t0").is_none(), "oldest entry must be evicted");
        assert!(topics.get("t99").is_none(), "oldest entries must be evicted");
        assert!(
            topics.get(&format!("t{}", total - 1)).is_some(),
            "newest entry must remain"
        );
    }

    #[test]
    fn unregister_message_topic_removes() {
        let store = ClientStore::new("", ":memory:", "http://test", "token", "u1");
        store.register_message_topic("t1");
        store.unregister_message_topic("t1");
        let mut topics = store.message_topics.try_lock().unwrap();
        assert!(topics.get("t1").is_none());
    }

    #[test]
    fn topic_owner_cache_lru_evicts_oldest() {
        let store = ClientStore::new("", ":memory:", "http://test", "token", "u1");
        {
            let mut cache = store.topic_owner_cache.try_lock().unwrap();
            for i in 0..(TOPIC_OWNER_CACHE_MAX_CAPACITY + 50) {
                cache.put(format!("t{}", i), (format!("owner-{}", i), 0));
            }
        }
        let mut cache = store.topic_owner_cache.try_lock().unwrap();
        assert_eq!(cache.len(), TOPIC_OWNER_CACHE_MAX_CAPACITY, "LRU must stay bounded");
        assert!(cache.get("t0").is_none(), "oldest entry must be evicted");
        assert!(
            cache
                .get(&format!("t{}", TOPIC_OWNER_CACHE_MAX_CAPACITY + 49))
                .is_some(),
            "newest entry must remain"
        );
    }

    #[tokio::test]
    async fn prune_conversations_bounds_local_store() {
        let store = ClientStore::new("", ":memory:", "http://test", "token", "u1");
        store
            .option
            .max_conversation_limit
            .store(5, std::sync::atomic::Ordering::Relaxed);
        let t = store.message_storage.table::<crate::models::Conversation>().await.unwrap();
        let now = crate::utils::now_millis();
        for i in 0..10 {
            let mut conversation = crate::models::Conversation::new(&format!("topic-{}", i));
            conversation.updated_at = rfc3339_at_millis(now - (10 - i) * 60_000);
            t.set("", &conversation.topic_id, Some(&conversation))
                .await
                .unwrap();
        }

        store.prune_conversations().await;

        let t = store.message_storage.table::<crate::models::Conversation>().await.unwrap();
        let items = t
            .filter("", Box::new(|c| Some(c)), None, None)
            .await
            .unwrap();
        assert_eq!(items.len(), 5, "conversation store must be pruned to the limit");
        let mut topics: Vec<String> = items.iter().map(|c| c.topic_id.clone()).collect();
        topics.sort();
        assert_eq!(
            topics,
            vec![
                "topic-5".to_string(),
                "topic-6".to_string(),
                "topic-7".to_string(),
                "topic-8".to_string(),
                "topic-9".to_string()
            ],
            "the newest conversations must be kept"
        );
    }
}
