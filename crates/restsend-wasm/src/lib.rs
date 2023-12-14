use restsend_sdk::models::AuthInfo;
use std::sync::{Arc, Mutex};
use wasm_bindgen::prelude::*;
mod account;
mod callback;
mod logger;
#[cfg(test)]
mod tests;

pub use logger::enable_logging;

use crate::callback::{CallbackWasmWrap, MessageCallbackWasmWrap};
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

#[allow(non_snake_case)]
#[wasm_bindgen]
impl Client {
    #[wasm_bindgen(constructor)]
    pub fn new(endpoint: String, userId: String, token: String) -> Self {
        let info = AuthInfo::new(&endpoint, &userId, &token);
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
    pub fn set_onconnected(&self, cb: JsValue) {
        if cb.is_function() {
            self.cb_on_connected
                .lock()
                .unwrap()
                .replace(js_sys::Function::from(cb));
        }
    }
    /// Set the callback when connection connecting
    #[wasm_bindgen(setter)]
    pub fn set_onconnecting(&self, cb: JsValue) {
        if cb.is_function() {
            self.cb_on_connecting
                .lock()
                .unwrap()
                .replace(js_sys::Function::from(cb));
        }
    }
    /// Set the callback when connection token expired
    #[wasm_bindgen(setter)]
    pub fn set_ontokenexpired(&self, cb: JsValue) {
        if cb.is_function() {
            self.cb_on_token_expired
                .lock()
                .unwrap()
                .replace(js_sys::Function::from(cb));
        }
    }
    /// Set the callback when connection broken
    #[wasm_bindgen(setter)]
    pub fn set_onbroken(&self, cb: JsValue) {
        if cb.is_function() {
            self.cb_on_net_broken
                .lock()
                .unwrap()
                .replace(js_sys::Function::from(cb));
        }
    }
    /// Set the callback when kickoff by other client
    #[wasm_bindgen(setter)]
    pub fn set_onkickoff(&self, cb: JsValue) {
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
    /// Send text message
    /// # Arguments
    /// * `topicId` - The topic id
    /// * `text` - The text message
    /// * `mentions` - The mentions userid , optional
    /// * `replyTo` - The reply message id, optional
    /// * `cb` - The callback function, optional
    /// # Example
    /// ```javascript
    /// const client = new Client(endpoint, userId, token);
    /// await client.sendText(topicId, text, mentions, replyTo, {
    ///     onsent:  () => {},
    ///     onprogress:  (progress:Number, total:Number)  =>{},
    ///     onack:  (req:ChatRequest)  => {},
    ///     onfail:  (reason:String)  => {}
    /// });
    /// ```
    ///
    pub async fn doSendText(
        &self,
        topicId: String,
        text: String,
        mentions: Option<Vec<String>>,
        replyTo: Option<String>,
        cb: JsValue,
    ) -> Result<(), JsValue> {
        self.inner
            .do_send_text(
                topicId,
                text,
                mentions,
                replyTo,
                Some(Box::new(MessageCallbackWasmWrap::new(cb))),
            )
            .await
            .ok();
        Ok(())
    }
}
