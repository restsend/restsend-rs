use restsend_sdk::models::AuthInfo;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

mod js;
#[cfg(test)]
mod tests;

#[wasm_bindgen]
pub struct RsClient(Arc<restsend_sdk::client::Client>);

#[wasm_bindgen]
impl RsClient {
    #[wasm_bindgen(constructor)]
    pub fn new(endpoint: String, user_id: String, token: String) -> Self {
        let info = AuthInfo::new(&endpoint, &user_id, &token);
        RsClient {
            0: restsend_sdk::client::Client::new("".to_string(), "".to_string(), &info),
        }
    }
}
