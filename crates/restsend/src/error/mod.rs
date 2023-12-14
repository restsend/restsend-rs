#[cfg(not(target_family = "wasm"))]
pub mod uniffi_wrap;
#[cfg(target_family = "wasm")]
pub mod wasm_wrap;

#[cfg(not(target_family = "wasm"))]
pub use uniffi_wrap::*;

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

#[cfg(target_family = "wasm")]
impl From<ClientError> for wasm_bindgen::JsValue {
    fn from(e: ClientError) -> wasm_bindgen::JsValue {
        wasm_bindgen::JsValue::from_str(&e.to_string())
    }
}
