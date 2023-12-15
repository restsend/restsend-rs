use crate::{CallbackFunction, Client};
use restsend_sdk::request::ChatRequest;
use std::sync::{Arc, Mutex};
use wasm_bindgen::prelude::*;

pub(super) struct CallbackWasmWrap {
    pub(super) cb_on_connected: CallbackFunction,
    pub(super) cb_on_connecting: CallbackFunction,
    pub(super) cb_on_token_expired: CallbackFunction,
    pub(super) cb_on_net_broken: CallbackFunction,
    pub(super) cb_on_kickoff_by_other_client: CallbackFunction,
}
unsafe impl Send for CallbackWasmWrap {}
unsafe impl Sync for CallbackWasmWrap {}

impl restsend_sdk::callback::Callback for CallbackWasmWrap {
    fn on_connected(&self) {
        if let Some(cb) = self.cb_on_connected.lock().unwrap().as_ref() {
            cb.call0(&JsValue::NULL).ok();
        }
    }
    fn on_connecting(&self) {
        if let Some(cb) = self.cb_on_connecting.lock().unwrap().as_ref() {
            cb.call0(&JsValue::NULL).ok();
        }
    }

    fn on_net_broken(&self, reason: String) {
        if let Some(cb) = self.cb_on_net_broken.lock().unwrap().as_ref() {
            cb.call1(&JsValue::NULL, &JsValue::from_str(&reason)).ok();
        }
    }

    fn on_kickoff_by_other_client(&self, reason: String) {
        if let Some(cb) = self.cb_on_kickoff_by_other_client.lock().unwrap().as_ref() {
            cb.call1(&JsValue::NULL, &JsValue::from_str(&reason)).ok();
        }
    }

    fn on_token_expired(&self, reason: String) {
        if let Some(cb) = self.cb_on_token_expired.lock().unwrap().as_ref() {
            cb.call1(&JsValue::NULL, &JsValue::from_str(&reason)).ok();
        }
    }
}

pub(super) struct MessageCallbackWasmWrap {
    pub(super) cb_on_sent: CallbackFunction,
    pub(super) cb_on_progress: CallbackFunction,
    pub(super) cb_on_ack: CallbackFunction,
    pub(super) cb_on_fail: CallbackFunction,
}

unsafe impl Send for MessageCallbackWasmWrap {}
unsafe impl Sync for MessageCallbackWasmWrap {}

fn get_function(cb: &JsValue, key: &str) -> CallbackFunction {
    let property_key = JsValue::from_str(key);
    let value = js_sys::Reflect::get(&cb, &property_key);
    if let Ok(v) = value {
        if let Ok(v) = v.dyn_into::<js_sys::Function>() {
            return Arc::new(Mutex::new(Some(v)));
        }
    }
    Arc::new(Mutex::new(None))
}

impl MessageCallbackWasmWrap {
    pub fn new(cb: JsValue) -> Self {
        Self {
            cb_on_sent: get_function(&cb, "onsent"),
            cb_on_progress: get_function(&cb, "onprogress"),
            cb_on_ack: get_function(&cb, "onack"),
            cb_on_fail: get_function(&cb, "onfail"),
        }
    }
}

impl restsend_sdk::callback::MessageCallback for MessageCallbackWasmWrap {
    fn on_sent(&self) {
        if let Some(cb) = self.cb_on_sent.lock().unwrap().as_ref() {
            cb.call0(&JsValue::NULL).ok();
        }
    }

    fn on_progress(&self, progress: u64, total: u64) {
        if let Some(cb) = self.cb_on_progress.lock().unwrap().as_ref() {
            cb.call2(
                &JsValue::NULL,
                &JsValue::from_f64(progress as f64),
                &JsValue::from_f64(total as f64),
            )
            .ok();
        }
    }

    fn on_ack(&self, req: ChatRequest) {
        if let Some(cb) = self.cb_on_ack.lock().unwrap().as_ref() {
            let req = serde_wasm_bindgen::to_value(&req).unwrap_or(JsValue::NULL);
            cb.call1(&JsValue::NULL, &req).ok();
        }
    }

    fn on_fail(&self, reason: String) {
        if let Some(cb) = self.cb_on_fail.lock().unwrap().as_ref() {
            cb.call1(&JsValue::NULL, &JsValue::from_str(&reason)).ok();
        }
    }
}

#[allow(non_snake_case)]
#[wasm_bindgen]
impl Client {
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
}
