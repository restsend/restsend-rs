#[cfg(not(target_family = "wasm"))]
pub mod uniffi_wrap;
#[cfg(target_family = "wasm")]
pub mod wasm_wrap;

#[cfg(not(target_family = "wasm"))]
pub use uniffi_wrap::*;

use wasm_bindgen::JsCast;
#[cfg(target_family = "wasm")]
pub use wasm_wrap::*;

impl From<reqwest::Error> for ClientError {
    fn from(e: reqwest::Error) -> ClientError {
        ClientError::HTTP(e.to_string())
    }
}

impl From<std::num::ParseIntError> for ClientError {
    fn from(e: std::num::ParseIntError) -> ClientError {
        ClientError::StdError(e.to_string())
    }
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for ClientError {
    fn from(e: tokio::sync::mpsc::error::SendError<T>) -> ClientError {
        ClientError::StdError(e.to_string())
    }
}

impl From<std::io::Error> for ClientError {
    fn from(e: std::io::Error) -> ClientError {
        ClientError::StdError(format!("io error {}", e.to_string()))
    }
}

impl From<ClientError> for wasm_bindgen::JsValue {
    fn from(e: ClientError) -> wasm_bindgen::JsValue {
        wasm_bindgen::JsValue::from_str(&e.to_string())
    }
}

impl From<wasm_bindgen::JsValue> for ClientError {
    fn from(e: wasm_bindgen::JsValue) -> ClientError {
        match e.dyn_into::<js_sys::Error>() {
            Ok(v) => {
                let msg = v.message();
                ClientError::StdError(msg.as_string().unwrap_or_default())
            }
            Err(e) => ClientError::StdError(e.as_string().unwrap_or_default()),
        }
    }
}
