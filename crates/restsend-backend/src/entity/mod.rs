pub mod attachment;
pub mod auth_token;
pub mod chat_log;
pub mod conversation;
pub mod presence_session;
pub mod relation;
pub mod topic;
pub mod topic_knock;
pub mod topic_member;
pub mod user;

use serde::{de::DeserializeOwned, Serialize};

pub(crate) fn encode_json<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string())
}

pub(crate) fn decode_json<T: DeserializeOwned + Default>(value: &str) -> T {
    serde_json::from_str(value).unwrap_or_default()
}
