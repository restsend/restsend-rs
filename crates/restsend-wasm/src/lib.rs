// use restsend_sdk::models::AuthInfo;
// use std::{sync::Arc, time::Duration};
// use wasm_bindgen::prelude::*;

mod js;
#[cfg(test)]
mod tests;

// #[wasm_bindgen]
// pub struct RsClient(Arc<restsend_sdk::client::Client>);

// #[wasm_bindgen]
// impl RsClient {
//     #[wasm_bindgen(constructor)]
//     pub fn new() -> Self {
//         let info = AuthInfo::new("endpoint", "userid", "token");
//         RsClient {
//             0: restsend_sdk::client::Client::new("".to_string(), "".to_string(), &info),
//         }
//     }
// }
