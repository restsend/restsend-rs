use crate::CallbackFunction;
use wasm_bindgen::prelude::*;

pub(super) struct CallbackWasmWrap {
    pub(super) cb_on_connected: CallbackFunction,
    pub(super) cb_on_connecting: CallbackFunction,
    pub(super) cb_on_token_expired: CallbackFunction,
    pub(super) cb_on_net_broken: CallbackFunction,
    pub(super) cb_on_kickoff_by_other_client: CallbackFunction,
}

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
unsafe impl Send for CallbackWasmWrap {}
unsafe impl Sync for CallbackWasmWrap {}
