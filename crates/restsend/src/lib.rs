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
const USER_AGENT: &str = "restsend-sdk/0.0.2"; // ios/android
#[cfg(target_family = "wasm")]
const DB_SUFFIX: &str = "";
#[cfg(not(target_family = "wasm"))]
const DB_SUFFIX: &str = ".sqlite3";

const CHAT_ID_LEN: usize = 10;
const TEMP_FILENAME_LEN: usize = 12;

const MAX_RECALL_SECS: i64 = 2 * 60; // 2 minutes
const MAX_CONVERSATION_LIMIT: u32 = 1000;
const MAX_LOGS_LIMIT: u32 = 100;
const MAX_SYNC_LOGS_MAX_COUNT: u32 = 200;
const MAX_RETRIES: usize = 3;
const MAX_SEND_IDLE_SECS: u64 = 120; // 2 minutes
const MAX_CONNECT_INTERVAL_SECS: u64 = 5; // 5 seconds
#[allow(unused)]
const MAX_ATTACHMENT_CONCURRENT: usize = 12;

const KEEPALIVE_INTERVAL_SECS: u32 = 50; // 50 seconds
const MEDIA_PROGRESS_INTERVAL: u128 = 300; // 300ms to update progress
const CONVERSATION_CACHE_EXPIRE_SECS: i64 = 60; // 60 seconds
const USER_CACHE_EXPIRE_SECS: i64 = 60; // 60 seconds
const REMOVED_CONVERSATION_CACHE_EXPIRE_SECS: i64 = 10; // 10 seconds

#[cfg(not(target_family = "wasm"))]
const WORKER_THREADS: usize = 4;

#[cfg(target_arch = "aarch64")]
#[cfg(target_vendor = "apple")]
pub const DEVICE: &str = "ios";

#[cfg(target_arch = "aarch64")]
#[cfg(target_vendor = "unknown")]
pub const DEVICE: &str = "android";

#[cfg(any(target_arch = "x86_64", target_family = "wasm"))]
pub const DEVICE: &str = "web";

pub type Error = error::ClientError;
pub type Result<T> = std::result::Result<T, crate::Error>;
