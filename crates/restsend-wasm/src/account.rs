use restsend_sdk::models::conversation::Extra;
use serde::Serialize;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

pub fn get_endpoint(endpoint: String) -> String {
    if endpoint.is_empty() {
        match web_sys::window() {
            Some(w) => w.location().origin().unwrap_or_default(),
            None => "".to_string(),
        }
    } else {
        endpoint
    }
}

/// Signin with userId and password or token
#[allow(non_snake_case)]
#[wasm_bindgen]
pub async fn signin(
    endpoint: String,
    userId: String,
    password: Option<String>,
    token: Option<String>,
) -> Result<JsValue, JsValue> {
    let endpoint = get_endpoint(endpoint);
    let serializer = &serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);

    match password {
        Some(password) => {
            restsend_sdk::services::auth::login_with_password(endpoint, userId, password).await
        }
        None => {
            restsend_sdk::services::auth::login_with_token(
                endpoint,
                userId,
                token.unwrap_or_default(),
            )
            .await
        }
    }
    .map(|v| v.serialize(serializer).unwrap_or(JsValue::UNDEFINED))
    .map_err(|e| e.into())
}

/// Signup with userId and password
#[allow(non_snake_case)]
#[wasm_bindgen]
pub async fn signup(
    endpoint: String,
    userId: String,
    password: String,
) -> Result<JsValue, JsValue> {
    let endpoint = get_endpoint(endpoint);
    let serializer = &serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);

    restsend_sdk::services::auth::signup(endpoint, userId, password)
        .await
        .map(|v| v.serialize(serializer).unwrap_or(JsValue::UNDEFINED))
        .map_err(|e| e.into())
}

/// Signup with userId and password
#[allow(non_snake_case)]
#[wasm_bindgen]
pub async fn guestLogin(
    endpoint: String,
    userId: String,
    extra: JsValue,
) -> Result<JsValue, JsValue> {
    let endpoint = get_endpoint(endpoint);
    let serializer = &serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
    let extra = serde_wasm_bindgen::from_value::<Extra>(extra).ok();
    restsend_sdk::services::auth::guest_login(endpoint, userId, extra)
        .await
        .map(|v| v.serialize(serializer).unwrap_or(JsValue::UNDEFINED))
        .map_err(|e| e.into())
}

/// Logout with token
#[allow(non_snake_case)]
#[wasm_bindgen]
pub async fn logout(endpoint: String, token: String) -> Result<(), JsValue> {
    let endpoint = get_endpoint(endpoint);
    restsend_sdk::services::auth::logout(endpoint, token)
        .await
        .map_err(|e| e.into())
}
