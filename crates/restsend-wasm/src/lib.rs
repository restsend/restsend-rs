use log::warn;
use restsend_sdk::models::AuthInfo;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
mod js;
mod logger;
#[cfg(test)]
mod tests;

pub use logger::enable_logging;

#[wasm_bindgen]
pub async fn signin(
    endpoint: String,
    user_id: String,
    password: Option<String>,
    token: Option<String>,
) -> Result<JsValue, JsValue> {
    let info = match password {
        Some(password) => {
            restsend_sdk::services::auth::login_with_password(endpoint, user_id, password).await
        }
        None => {
            restsend_sdk::services::auth::login_with_token(
                endpoint,
                user_id,
                token.unwrap_or_default(),
            )
            .await
        }
    }
    .map_err(|e| JsValue::from_str(e.to_string().as_str()))?;
    serde_wasm_bindgen::to_value(&info).map_err(|e| JsValue::from_str(e.to_string().as_str()))
}

#[wasm_bindgen]
pub async fn signup(
    endpoint: String,
    user_id: String,
    password: String,
) -> Result<JsValue, JsValue> {
    let info = restsend_sdk::services::auth::signup(endpoint, user_id, password)
        .await
        .map_err(|e| JsValue::from_str(e.to_string().as_str()))?;
    serde_wasm_bindgen::to_value(&info).map_err(|e| JsValue::from_str(e.to_string().as_str()))
}

#[wasm_bindgen]
pub struct Client {
    inner: Arc<restsend_sdk::client::Client>,
}

#[wasm_bindgen]
impl Client {
    #[wasm_bindgen(constructor)]
    pub fn new(endpoint: String, user_id: String, token: String) -> Self {
        let info = AuthInfo::new(&endpoint, &user_id, &token);
        Client {
            inner: restsend_sdk::client::Client::new("".to_string(), "".to_string(), &info),
        }
    }

    #[wasm_bindgen(getter)]
    pub fn connection_status(&self) -> String {
        self.inner.connection_status()
    }

    pub async fn connect(&self) -> Result<(), JsValue> {
        struct TestCallbackImpl;
        impl restsend_sdk::callback::Callback for TestCallbackImpl {
            fn on_connected(&self) {
                warn!("on_connected");
            }
            fn on_connecting(&self) {
                warn!("on_connecting");
            }
            fn on_net_broken(&self, reason: String) {
                warn!("on_disconnected {}", reason);
            }
        }
        self.inner.connect(Box::new(TestCallbackImpl {})).await;
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
