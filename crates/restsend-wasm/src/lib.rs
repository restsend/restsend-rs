use crate::{
    callback::{CallbackWasmWrap, MessageCallbackWasmWrap},
    js_util::{get_string, get_vec_strings, js_value_to_attachment},
};
use restsend_sdk::models::AuthInfo;
use std::sync::{Arc, Mutex};
use wasm_bindgen::prelude::*;

mod account;
mod callback;
mod js_util;
mod logger;
mod message;
#[cfg(test)]
mod tests;
pub use logger::enable_logging;

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
}
