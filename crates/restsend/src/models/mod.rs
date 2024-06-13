use restsend_macros::export_wasm_or_ffi;
use serde::{Deserialize, Serialize};

#[inline]
pub fn omit_empty<T: ?Sized + Default + std::cmp::PartialEq>(value: &T) -> bool
where
    T: serde::ser::Serialize,
{
    return *value == T::default();
}

#[derive(Debug, Serialize)]
#[export_wasm_or_ffi(#[derive(uniffi::Record)])]
#[serde(rename_all = "camelCase")]
pub struct GetChatLogsResult {
    pub has_more: bool,
    pub start_seq: i64,
    pub end_seq: i64,
    pub items: Vec<ChatLog>,
}

impl GetChatLogsResult {
    pub fn from_local_logs(qr: QueryResult<ChatLog>, start_seq: i64) -> Self {
        GetChatLogsResult {
            has_more: qr.end_sort_value > start_seq + 1,
            start_seq: qr.start_sort_value,
            end_seq: qr.end_sort_value,
            items: qr.items,
        }
    }
}
impl From<ListChatLogResult> for GetChatLogsResult {
    fn from(lr: ListChatLogResult) -> Self {
        Self {
            has_more: lr.has_more,
            start_seq: lr.items.first().map(|c| c.seq).unwrap_or(0),
            end_seq: lr.items.last().map(|c| c.seq).unwrap_or(0),
            items: lr.items,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
#[export_wasm_or_ffi(#[derive(uniffi::Record)])]
pub struct ListUserResult {
    pub has_more: bool,
    pub updated_at: String,
    #[serde(default)]
    pub items: Vec<User>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ListConversationResult {
    pub total: i64,
    pub has_more: bool,
    #[serde(default)]
    pub offset: u32,
    #[serde(default)]
    pub items: Vec<Conversation>,
    #[serde(default)]
    pub last_updated_at: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ListChatLogResult {
    pub topic_id: Option<String>,
    pub has_more: bool,
    pub updated_at: String,
    pub last_seq: i64,
    #[serde(default)]
    pub items: Vec<ChatLog>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[export_wasm_or_ffi(#[derive(uniffi::Record)])]
pub struct TopicKnock {
    pub created_at: String,

    pub updated_at: String,

    pub topic_id: String,

    pub user_id: String,

    #[serde(default)]
    pub message: String,

    #[serde(default)]
    pub source: String,

    pub status: String,

    #[serde(default)]
    pub admin_id: String,
}

impl TopicKnock {
    pub fn new(topic_id: &str, user_id: &str) -> Self {
        TopicKnock {
            created_at: String::default(),
            updated_at: String::default(),
            topic_id: String::from(topic_id),
            user_id: String::from(user_id),
            message: String::default(),
            source: String::default(),
            status: String::default(),
            admin_id: String::default(),
        }
    }
}

pub mod chat_log;
pub mod conversation;
pub mod topic;
pub mod topic_member;
pub mod user;

pub use chat_log::{Attachment, AttachmentStatus, ChatLog, ChatLogStatus, Content, ContentType};
pub use conversation::Conversation;
pub use topic::Topic;
pub use topic::TopicNotice;
pub use topic_member::TopicMember;
pub use user::{AuthInfo, User, UserProfile};

use crate::storage::QueryResult;
