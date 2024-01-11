use crate::callback::CallbackWasmWrap;
use restsend_sdk::models::AuthInfo;
use std::sync::{Arc, Mutex};
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

type CallbackFunction = Arc<Mutex<Option<js_sys::Function>>>;
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
    inner: Arc<restsend_sdk::client::Client>,
}

#[allow(non_snake_case)]
#[wasm_bindgen]
impl Client {
    #[wasm_bindgen(constructor)]
    pub fn new(info: JsValue, db_name: Option<String>) -> Self {
        let info = match serde_wasm_bindgen::from_value::<AuthInfo>(info) {
            Ok(v) => v,
            Err(_) => AuthInfo::default(),
        };
        #[cfg(feature = "indexeddb")]
        let inner = restsend_sdk::client::Client::new(
            "".to_string(),
            db_name.unwrap_or(info.user_id.clone()),
            &info,
        );

        #[cfg(not(feature = "indexeddb"))]
        let inner =
            restsend_sdk::client::Client::new("".to_string(), db_name.unwrap_or_default(), &info);

        Self::create(inner)
    }
    /// get the current connection status
    /// return: connecting, connected, broken, shutdown
    #[wasm_bindgen(getter)]
    pub fn connectionStatus(&self) -> String {
        self.inner.connection_status()
    }

    pub async fn shutdown(&self) {
        self.inner.shutdown()
    }

    pub async fn connect(&self) -> Result<(), JsValue> {
        self.inner.connect().await;
        Ok(())
    }
}

impl Client {
    pub fn create(inner: Arc<restsend_sdk::client::Client>) -> Self {
        let c = Client {
            cb_on_connected: Arc::new(Mutex::new(None)),
            cb_on_connecting: Arc::new(Mutex::new(None)),
            cb_on_token_expired: Arc::new(Mutex::new(None)),
            cb_on_net_broken: Arc::new(Mutex::new(None)),
            cb_on_kickoff_by_other_client: Arc::new(Mutex::new(None)),
            cb_on_system_request: Arc::new(Mutex::new(None)),
            cb_on_unknown_request: Arc::new(Mutex::new(None)),
            cb_on_topic_typing: Arc::new(Mutex::new(None)),
            cb_on_topic_message: Arc::new(Mutex::new(None)),
            cb_on_topic_read: Arc::new(Mutex::new(None)),
            cb_on_conversations_updated: Arc::new(Mutex::new(None)),
            cb_on_conversation_removed: Arc::new(Mutex::new(None)),
            inner: inner.clone(),
        };

        let cb = Box::new(CallbackWasmWrap {
            cb_on_connected: c.cb_on_connected.clone(),
            cb_on_connecting: c.cb_on_connecting.clone(),
            cb_on_token_expired: c.cb_on_token_expired.clone(),
            cb_on_net_broken: c.cb_on_net_broken.clone(),
            cb_on_kickoff_by_other_client: c.cb_on_kickoff_by_other_client.clone(),
            cb_on_system_request: c.cb_on_system_request.clone(),
            cb_on_unknown_request: c.cb_on_unknown_request.clone(),
            cb_on_topic_typing: c.cb_on_topic_typing.clone(),
            cb_on_topic_message: c.cb_on_topic_message.clone(),
            cb_on_topic_read: c.cb_on_topic_read.clone(),
            cb_on_conversations_updated: c.cb_on_conversations_updated.clone(),
            cb_on_conversation_removed: c.cb_on_conversation_removed.clone(),
        });
        inner.set_callback(Some(cb));
        c
    }
}
