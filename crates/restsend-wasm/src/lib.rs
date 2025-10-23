use crate::{callback::CallbackWasmWrap, js_util::get_bool};
use js_util::get_string;
use restsend_sdk::models::{conversation::Extra, AuthInfo};
use std::{cell::RefCell, rc::Rc, sync::atomic::Ordering};
use wasm_bindgen::prelude::*;

#[cfg(feature = "auth")]
mod account;
mod callback;
#[cfg(feature = "conversation")]
mod conversations;
mod js_util;
mod logger;
#[cfg(feature = "message")]
mod message;
#[cfg(feature = "topic")]
mod topics;
mod users;

pub use logger::setLogging;

type CallbackFunction = Rc<RefCell<Option<js_sys::Function>>>;
#[wasm_bindgen]
pub struct Client {
    cb_on_connected: CallbackFunction,
    cb_on_connecting: CallbackFunction,
    cb_on_token_expired: CallbackFunction,
    cb_on_net_broken: CallbackFunction,
    cb_on_kickoff_by_other_client: CallbackFunction,
    cb_on_system_request: CallbackFunction,
    cb_on_unknown_request: CallbackFunction,
    cb_on_topic_typing: CallbackFunction,
    cb_on_topic_message: CallbackFunction,
    cb_on_topic_read: CallbackFunction,
    cb_on_conversations_updated: CallbackFunction,
    cb_on_conversation_removed: CallbackFunction,
    inner: restsend_sdk::client::Client,
}

#[allow(non_snake_case)]
#[wasm_bindgen]
impl Client {
    /// Create a new client
    /// # Arguments
    /// * `info` - AuthInfo
    /// * `db_name` - database name (optional), create an indexeddb when set it    
    #[wasm_bindgen(constructor)]
    pub fn new(info: JsValue, db_name: Option<String>) -> Self {
        let info = match serde_wasm_bindgen::from_value::<AuthInfo>(info.clone()) {
            Ok(v) => v,
            Err(_) => {
                let privateExtra = js_sys::Reflect::get(&info, &JsValue::from_str("privateExtra"))
                    .unwrap_or(JsValue::UNDEFINED);
                AuthInfo {
                    endpoint: get_string(&info, "endpoint").unwrap_or_default(),
                    user_id: get_string(&info, "userId").unwrap_or_default(),
                    token: get_string(&info, "token").unwrap_or_default(),
                    is_cross_domain: get_bool(&info, "isCrossDomain"),
                    is_staff: get_bool(&info, "isStaff"),
                    avatar: get_string(&info, "avatar").unwrap_or_default(),
                    name: get_string(&info, "name").unwrap_or_default(),
                    private_extra: serde_wasm_bindgen::from_value::<Extra>(privateExtra).ok(),
                }
            }
        };

        let inner = restsend_sdk::client::Client::new_sync(
            "".to_string(),
            db_name.unwrap_or_default(),
            &info,
        );
        Self::create(inner)
    }
    /// get the current connection status
    /// return: connecting, connected, broken, shutdown
    #[wasm_bindgen(getter)]
    #[allow(non_snake_case)]
    pub fn connectionStatus(&self) -> String {
        self.inner.connection_status()
    }

    /// get the last alive at
    #[wasm_bindgen(getter)]
    #[allow(non_snake_case)]
    pub fn lastAliveAt(&self) -> i64 {
        self.inner.get_last_alive_at()
    }

    #[wasm_bindgen(getter)]
    #[allow(non_snake_case)]
    pub async fn unreadCount(&self) -> u32 {
        self.inner.get_unread_count().await
    }
    /// connect immediately if the connection is broken    
    pub fn app_active(&self) {
        self.inner.app_active();
    }
    /// set the keepalive interval with seconds
    #[wasm_bindgen(setter)]
    pub fn set_keepalive(&self, secs: u32) {
        self.inner.set_keepalive_interval_secs(secs);
    }

    /// set the ping interval with seconds (for health check with error logs)
    /// default is 30 seconds
    #[wasm_bindgen(setter)]
    pub fn set_ping_interval(&self, secs: u32) {
        self.inner.set_keepalive_interval_secs(secs);
    }
    /// set the max retry count
    /// default is 2
    #[wasm_bindgen(setter)]
    pub fn set_maxRetry(&self, count: u32) {
        self.inner
            .store
            .option
            .max_retry
            .store(count as usize, Ordering::Relaxed);
    }
    /// set the max send idle seconds
    /// default is 20 seconds
    #[wasm_bindgen(setter)]
    pub fn set_maxSendIdleSecs(&self, secs: u32) {
        self.inner
            .store
            .option
            .max_send_idle_secs
            .store(secs as u64, Ordering::Relaxed);
    }
    /// set the max recall seconds
    /// default is 120 seconds
    /// note: server may have a limit as well
    /// for example, restsend server limit is 2 minutes
    #[wasm_bindgen(setter)]
    pub fn set_maxRecallSecs(&self, secs: u32) {
        self.inner
            .store
            .option
            .max_recall_secs
            .store(secs as usize, Ordering::Relaxed);
    }
    /// set the max conversation limit
    /// default is 1000
    /// note: this limit is for local storage only
    #[wasm_bindgen(setter)]
    pub fn set_maxConversationLimit(&self, limit: u32) {
        self.inner
            .store
            .option
            .max_conversation_limit
            .store(limit as usize, Ordering::Relaxed);
    }
    /// set the max logs limit per request
    /// default is 100
    /// note: this limit is for each request to fetch logs from server
    #[wasm_bindgen(setter)]
    pub fn set_maxLogsLimit(&self, limit: u32) {
        self.inner
            .store
            .option
            .max_logs_limit
            .store(limit as usize, Ordering::Relaxed);
    }
    /// set the max sync logs max count
    /// default is 200
    /// note: this limit is for each sync logs operation
    #[wasm_bindgen(setter)]
    pub fn set_maxSyncLogsMaxCount(&self, count: u32) {
        self.inner
            .store
            .option
            .max_sync_logs_max_count
            .store(count as usize, Ordering::Relaxed);
    }

    /// set the max connect interval seconds
    /// default is 5 seconds
    #[wasm_bindgen(setter)]
    pub fn set_maxConnectIntervalSecs(&self, secs: u32) {
        self.inner
            .store
            .option
            .max_connect_interval_secs
            .store(secs as usize, Ordering::Relaxed);
    }
    /// set the max sync logs limit
    /// default is 500
    #[wasm_bindgen(setter)]
    pub fn set_maxSyncLogsLimit(&self, limit: u32) {
        self.inner
            .store
            .option
            .max_sync_logs_limit
            .store(limit as usize, Ordering::Relaxed);
    }
    /// set the conversation cache expire seconds
    /// default is 60 seconds
    #[wasm_bindgen(setter)]
    pub fn set_conversationCacheExpireSecs(&self, secs: u32) {
        self.inner
            .store
            .option
            .conversation_cache_expire_secs
            .store(secs as usize, Ordering::Relaxed);
    }
    /// set the user cache expire seconds
    /// default is 60 seconds
    #[wasm_bindgen(setter)]
    pub fn set_userCacheExpireSecs(&self, secs: u32) {
        self.inner
            .store
            .option
            .user_cache_expire_secs
            .store(secs as usize, Ordering::Relaxed);
    }
    /// set the removed conversation cache expire seconds
    /// default is 10 seconds
    #[wasm_bindgen(setter)]
    pub fn set_removedConversationCacheExpireSecs(&self, secs: u32) {
        self.inner
            .store
            .option
            .removed_conversation_cache_expire_secs
            .store(secs as usize, Ordering::Relaxed);
    }
    /// set the ping timeout seconds
    /// default is 5 seconds
    #[wasm_bindgen(setter)]
    pub fn set_pingTimeoutSecs(&self, secs: u32) {
        self.inner
            .store
            .option
            .ping_timeout_secs
            .store(secs as usize, Ordering::Relaxed);
    }

    pub async fn shutdown(&self) {
        self.inner.shutdown().await
    }

    pub async fn connect(&self) -> Result<(), JsValue> {
        self.inner.connect().await;
        Ok(())
    }
}

impl Client {
    pub fn create(inner: restsend_sdk::client::Client) -> Self {
        let cb_on_connected = Rc::new(RefCell::new(None));
        let cb_on_connecting = Rc::new(RefCell::new(None));
        let cb_on_token_expired = Rc::new(RefCell::new(None));
        let cb_on_net_broken = Rc::new(RefCell::new(None));
        let cb_on_kickoff_by_other_client = Rc::new(RefCell::new(None));
        let cb_on_system_request = Rc::new(RefCell::new(None));
        let cb_on_unknown_request = Rc::new(RefCell::new(None));
        let cb_on_topic_typing = Rc::new(RefCell::new(None));
        let cb_on_topic_message = Rc::new(RefCell::new(None));
        let cb_on_topic_read = Rc::new(RefCell::new(None));
        let cb_on_conversations_updated = Rc::new(RefCell::new(None));
        let cb_on_conversation_removed = Rc::new(RefCell::new(None));

        let cb = Box::new(CallbackWasmWrap {
            cb_on_connected: cb_on_connected.clone(),
            cb_on_connecting: cb_on_connecting.clone(),
            cb_on_token_expired: cb_on_token_expired.clone(),
            cb_on_net_broken: cb_on_net_broken.clone(),
            cb_on_kickoff_by_other_client: cb_on_kickoff_by_other_client.clone(),
            cb_on_system_request: cb_on_system_request.clone(),
            cb_on_unknown_request: cb_on_unknown_request.clone(),
            cb_on_topic_typing: cb_on_topic_typing.clone(),
            cb_on_topic_message: cb_on_topic_message.clone(),
            cb_on_topic_read: cb_on_topic_read.clone(),
            cb_on_conversations_updated: cb_on_conversations_updated.clone(),
            cb_on_conversation_removed: cb_on_conversation_removed.clone(),
        });
        inner.set_callback(Some(cb));

        Client {
            cb_on_connected,
            cb_on_connecting,
            cb_on_token_expired,
            cb_on_net_broken,
            cb_on_kickoff_by_other_client,
            cb_on_system_request,
            cb_on_unknown_request,
            cb_on_topic_typing,
            cb_on_topic_message,
            cb_on_topic_read,
            cb_on_conversations_updated,
            cb_on_conversation_removed,
            inner,
        }
    }
}
