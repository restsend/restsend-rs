use restsend_sdk::models::AuthInfo;
use std::sync::{Arc, Mutex};
use wasm_bindgen::prelude::*;
mod account;
mod callback;
mod logger;
#[cfg(test)]
mod tests;

pub use logger::enable_logging;

use crate::callback::CallbackWasmWrap;
type CallbackFunction = Arc<Mutex<Option<js_sys::Function>>>;
#[wasm_bindgen]
pub struct Client {
    cb_on_connected: CallbackFunction,
    cb_on_connecting: CallbackFunction,
    cb_on_token_expired: CallbackFunction,
    cb_on_net_broken: CallbackFunction,
    cb_on_kickoff_by_other_client: CallbackFunction,
    inner: Arc<restsend_sdk::client::Client>,
}

#[wasm_bindgen]
impl Client {
    #[wasm_bindgen(constructor)]
    pub fn new(endpoint: String, user_id: String, token: String) -> Self {
        let info = AuthInfo::new(&endpoint, &user_id, &token);
        Client {
            cb_on_connected: Arc::new(Mutex::new(None)),
            cb_on_connecting: Arc::new(Mutex::new(None)),
            cb_on_token_expired: Arc::new(Mutex::new(None)),
            cb_on_net_broken: Arc::new(Mutex::new(None)),
            cb_on_kickoff_by_other_client: Arc::new(Mutex::new(None)),
            inner: restsend_sdk::client::Client::new("".to_string(), "".to_string(), &info),
        }
    }

    #[wasm_bindgen(getter)]
    pub fn connection_status(&self) -> String {
        self.inner.connection_status()
    }

    /// Set the callback when connection connected
    #[wasm_bindgen(setter)]
    pub fn set_on_connected(&self, cb: JsValue) {
        if cb.is_function() {
            self.cb_on_connected
                .lock()
                .unwrap()
                .replace(js_sys::Function::from(cb));
        }
    }
    /// Set the callback when connection connecting
    #[wasm_bindgen(setter)]
    pub fn set_on_connecting(&self, cb: JsValue) {
        if cb.is_function() {
            self.cb_on_connecting
                .lock()
                .unwrap()
                .replace(js_sys::Function::from(cb));
        }
    }
    /// Set the callback when connection token expired
    #[wasm_bindgen(setter)]
    pub fn set_on_token_expired(&self, cb: JsValue) {
        if cb.is_function() {
            self.cb_on_token_expired
                .lock()
                .unwrap()
                .replace(js_sys::Function::from(cb));
        }
    }
    /// Set the callback when connection broken
    #[wasm_bindgen(setter)]
    pub fn set_on_net_broken(&self, cb: JsValue) {
        if cb.is_function() {
            self.cb_on_net_broken
                .lock()
                .unwrap()
                .replace(js_sys::Function::from(cb));
        }
    }
    /// Set the callback when kickoff by other client
    #[wasm_bindgen(setter)]
    pub fn set_on_kickoff_by_other_client(&self, cb: JsValue) {
        if cb.is_function() {
            self.cb_on_kickoff_by_other_client
                .lock()
                .unwrap()
                .replace(js_sys::Function::from(cb));
        }
    }

    pub async fn connect(&self) -> Result<(), JsValue> {
        self.inner
            .connect(Box::new(CallbackWasmWrap {
                cb_on_connected: self.cb_on_connected.clone(),
                cb_on_connecting: self.cb_on_connecting.clone(),
                cb_on_token_expired: self.cb_on_token_expired.clone(),
                cb_on_net_broken: self.cb_on_net_broken.clone(),
                cb_on_kickoff_by_other_client: self.cb_on_kickoff_by_other_client.clone(),
            }))
            .await;
        Ok(())
    }

    pub async fn do_send_text(&self, topic_id: String, text: String) -> Result<(), JsValue> {
        struct TestCallbackImpl;
        impl restsend_sdk::callback::Callback for TestCallbackImpl {}
        self.inner
            .do_send_text(topic_id, text, None, None, None)
            .await
            .ok();
        Ok(())
    }
}
