use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type Extra = HashMap<String, String>;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AuthInfo {
    pub endpoint: String,
    pub user_id: String,
    pub avatar: String,
    pub name: String,
    pub token: String,
    #[serde(default)]
    pub is_staff: bool,
    #[serde(default)]
    pub is_cross_domain: bool,
    #[serde(default)]
    pub private_extra: Option<Extra>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct User {
    #[serde(default)]
    pub user_id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub avatar: String,
    #[serde(default)]
    pub public_key: String,
    #[serde(default)]
    pub remark: String,
    #[serde(default)]
    pub is_contact: bool,
    #[serde(default)]
    pub is_star: bool,
    #[serde(default)]
    pub is_blocked: bool,
    #[serde(default)]
    pub locale: String,
    #[serde(default)]
    pub city: String,
    #[serde(default)]
    pub country: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub gender: String,
    #[serde(default)]
    pub memo: String,
    #[serde(default)]
    pub extra: Option<Extra>,
    #[serde(default)]
    pub is_staff: bool,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TopicNotice {
    pub text: String,
    #[serde(default)]
    pub publisher: String,
    #[serde(default)]
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Topic {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub remark: String,
    #[serde(default)]
    pub owner_id: String,
    #[serde(default)]
    pub attendee_id: String,
    #[serde(default)]
    pub admins: Vec<String>,
    #[serde(default)]
    pub members: u32,
    #[serde(default)]
    pub last_seq: i64,
    #[serde(default)]
    pub multiple: bool,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub private: bool,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub notice: Option<TopicNotice>,
    #[serde(default)]
    pub extra: Option<Extra>,
    #[serde(default)]
    pub webhooks: Vec<String>,
    #[serde(default)]
    pub knock_need_verify: bool,
    #[serde(default)]
    pub silent_white_list: Vec<String>,
    #[serde(default)]
    pub silent: bool,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TopicMember {
    pub topic_id: String,
    pub user_id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub silence_at: Option<String>,
    #[serde(default)]
    pub joined_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub extra: Option<Extra>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Relation {
    #[serde(default)]
    pub owner_id: String,
    #[serde(default)]
    pub target_id: String,
    #[serde(default)]
    pub is_contact: bool,
    #[serde(default)]
    pub is_star: bool,
    #[serde(default)]
    pub is_blocked: bool,
    #[serde(default)]
    pub remark: String,
    #[serde(default)]
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    #[serde(default)]
    pub id: String,
    #[serde(default, rename = "type")]
    pub tag_type: String,
    #[serde(default)]
    pub label: String,
}

pub type Tags = Vec<Tag>;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub size: i64,
    #[serde(default)]
    pub thumbnail: String,
    #[serde(default)]
    pub file_name: String,
    #[serde(default)]
    pub file_path: String,
    #[serde(default)]
    pub url_or_data: String,
    #[serde(default)]
    pub is_private: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Content {
    #[serde(default, rename = "type")]
    pub content_type: String,
    #[serde(default)]
    pub encrypted: bool,
    #[serde(default)]
    pub checksum: u32,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub placeholder: String,
    #[serde(default)]
    pub thumbnail: String,
    #[serde(default)]
    pub duration: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub width: f32,
    #[serde(default)]
    pub height: f32,
    #[serde(default)]
    pub mentions: Vec<String>,
    #[serde(default)]
    pub mention_all: bool,
    #[serde(default)]
    pub reply: String,
    #[serde(default)]
    pub reply_content: Option<String>,
    #[serde(default)]
    pub attachment: Option<Attachment>,
    #[serde(default)]
    pub extra: Option<Extra>,
    #[serde(default)]
    pub unreadable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChatLog {
    pub topic_id: String,
    pub id: String,
    pub seq: i64,
    pub created_at: String,
    pub sender_id: String,
    pub content: Content,
    #[serde(default)]
    pub read: bool,
    #[serde(default)]
    pub recall: bool,
    #[serde(default)]
    pub deleted_by: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    pub owner_id: String,
    pub topic_id: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub start_seq: i64,
    #[serde(default)]
    pub last_seq: i64,
    #[serde(default)]
    pub last_read_seq: i64,
    #[serde(default)]
    pub last_read_at: Option<String>,
    #[serde(default)]
    pub multiple: bool,
    #[serde(default)]
    pub attendee: String,
    #[serde(default)]
    pub members: i64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub sticky: bool,
    #[serde(default)]
    pub mute: bool,
    #[serde(default)]
    pub source: String,
    #[serde(default, alias = "unreadCount")]
    pub unread: i64,
    #[serde(default)]
    pub last_sender_id: String,
    #[serde(default)]
    pub last_message: Option<Content>,
    #[serde(default)]
    pub last_message_at: String,
    #[serde(default)]
    pub last_message_seq: Option<i64>,
    #[serde(default)]
    pub remark: Option<String>,
    #[serde(default)]
    pub extra: Option<Extra>,
    #[serde(default)]
    pub topic_extra: Option<Extra>,
    #[serde(default)]
    pub topic_owner_id: Option<String>,
    #[serde(default)]
    pub topic_created_at: Option<String>,
    #[serde(default)]
    pub tags: Option<Tags>,
}
