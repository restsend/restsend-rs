use restsend_sdk::models::AuthInfo;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

mod js;
#[cfg(test)]
mod tests;

#[wasm_bindgen]
pub struct Client(Arc<restsend_sdk::client::Client>);

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
impl Client {
    #[wasm_bindgen(constructor)]
    pub fn new(endpoint: String, user_id: String, token: String) -> Self {
        let info = AuthInfo::new(&endpoint, &user_id, &token);
        Client {
            0: restsend_sdk::client::Client::new("".to_string(), "".to_string(), &info),
        }
    }

    pub async fn connect(&self) -> Result<(), JsValue> {
        struct TestCallbackImpl;
        impl restsend_sdk::callback::Callback for TestCallbackImpl {
            fn on_connected(&self) {
                js::log("on_connected");
            }
            fn on_connecting(&self) {
                js::log("on_connecting");
            }
        }
        self.0.connect(Box::new(TestCallbackImpl {})).await;
        Ok(())
    }

    pub async fn do_send_text(&self, topic_id: String, text: String) -> Result<(), JsValue> {
        struct TestCallbackImpl;
        impl restsend_sdk::callback::Callback for TestCallbackImpl {}
        self.0
            .do_send_text(topic_id, text, None, None, None)
            .await
            .ok();
        Ok(())
    }
}
