use serde::{Deserialize, Serialize};

#[inline]
pub fn omit_empty<T: ?Sized + Default + std::cmp::PartialEq>(value: &T) -> bool
where
    T: serde::ser::Serialize,
{
    return *value == T::default();
}

type CachedTime = String;
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ListUserResult {
    pub has_more: bool,
    pub updated_at: String,
    #[serde(default)]
    pub items: Vec<User>,
    #[serde(default)]
    pub removed: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ListConversationResult {
    pub has_more: bool,
    pub updated_at: String,
    #[serde(default)]
    pub items: Vec<Conversation>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ListChatLogResult {
    pub has_more: bool,
    pub updated_at: String,
    pub last_seq: u64,
    #[serde(default)]
    pub items: Vec<ChatLog>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
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

pub use chat_log::ChatLog;
pub use chat_log::Content;
pub use chat_log::ContentType;
pub use conversation::Conversation;
pub use topic::Topic;
pub use topic::TopicNotice;
pub use topic_member::TopicMember;
pub use user::{AuthInfo, User, UserProfile};
