use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

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
