#[cfg(not(target_family = "wasm"))]
uniffi::setup_scaffolding!();
pub mod account;
pub mod callback;
pub mod client;
pub mod error;

pub mod media;
pub mod models;
pub mod request;
pub mod services;
pub mod storage;
pub mod utils;
mod websocket;
#[allow(unused)]
const USER_AGENT: &str = concat!("restsend/", env!("CARGO_PKG_VERSION"));
#[cfg(target_family = "wasm")]
const DB_SUFFIX: &str = "";
#[cfg(not(target_family = "wasm"))]
const DB_SUFFIX: &str = ".sqlite3";

const CHAT_ID_LEN: usize = 10;
const TEMP_FILENAME_LEN: usize = 12;

#[cfg(not(target_family = "wasm"))]
const WORKER_THREADS: usize = 4;

#[cfg(all(target_arch = "aarch64", target_vendor = "apple"))]
pub const DEVICE: &str = "ios";

#[cfg(all(
    any(target_arch = "aarch64", target_arch = "arm", target_arch = "x86"),
    target_vendor = "unknown",
    not(target_family = "wasm")
))]
pub const DEVICE: &str = "android";

#[cfg(any(target_arch = "x86_64", target_family = "wasm"))]
pub const DEVICE: &str = "web";

#[cfg(not(any(
    all(target_arch = "aarch64", target_vendor = "apple"),
    all(
        any(target_arch = "aarch64", target_arch = "arm", target_arch = "x86"),
        target_vendor = "unknown",
        not(target_family = "wasm")
    ),
    target_arch = "x86_64",
    target_family = "wasm"
)))]
pub const DEVICE: &str = "unknown";

pub type Error = error::ClientError;
pub type Result<T> = std::result::Result<T, crate::Error>;
