use futures_util::{pin_mut, select, FutureExt};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::js_sys;
mod js;

#[wasm_bindgen]
pub struct RsClient();

#[wasm_bindgen]
impl RsClient {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        RsClient {}
    }
    pub async fn connect(&self, callback: JsValue) {
        js::console_log("connect");
    }
}
