use super::{omit_empty, Content, Topic};
use crate::{request::ChatRequest, storage::StoreModel};
use restsend_macros::export_wasm_or_ffi;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
#[export_wasm_or_ffi(#[derive(uniffi::Record)])]
pub struct Tag {
    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub id: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub r#type: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub label: String,
}

pub type Tags = Vec<Tag>;
pub type Extra = HashMap<String, String>;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
#[export_wasm_or_ffi(#[derive(uniffi::Record)])]
pub struct Conversation {
    pub owner_id: String,
    pub topic_id: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub updated_at: String,

    #[serde(skip_serializing_if = "omit_empty")]
    #[serde(default)]
    pub start_seq: i64,

    #[serde(skip_serializing_if = "omit_empty")]
    #[serde(default)]
    pub last_seq: i64,

    #[serde(skip_serializing_if = "omit_empty")]
    #[serde(default)]
    pub last_read_seq: i64,

    #[serde(skip_serializing_if = "omit_empty")]
    #[serde(default)]
    pub multiple: bool,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub attendee: String,

    #[serde(skip_serializing_if = "omit_empty")]
    #[serde(default)]
    pub members: i64,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub name: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub icon: String,

    #[serde(skip_serializing_if = "omit_empty")]
    #[serde(default)]
    pub sticky: bool,

    #[serde(skip_serializing_if = "omit_empty")]
    #[serde(default)]
    pub mute: bool,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub source: String,

    #[serde(skip_serializing_if = "omit_empty")]
    #[serde(default)]
    pub unread: i64,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub last_sender_id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub last_message: Option<Content>,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub last_message_at: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remark: Option<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<Extra>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic_extra: Option<Extra>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Tags>,

    #[serde(default)]
    pub cached_at: i64,

    #[serde(default)]
    pub is_partial: bool,
}

impl Conversation {
    pub fn new(topic_id: &str) -> Self {
        Conversation {
            topic_id: String::from(topic_id),
            is_partial: true,
            ..Default::default()
        }
    }
}

impl FromStr for Conversation {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str::<Conversation>(s)
    }
}

impl ToString for Conversation {
    fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

impl StoreModel for Conversation {
    fn sort_key(&self) -> i64 {
        match chrono::DateTime::parse_from_rfc3339(&self.updated_at) {
            Ok(dt) => dt.timestamp_millis(),
            Err(_) => self.cached_at,
        }
    }
}

impl From<&ChatRequest> for Conversation {
    fn from(req: &ChatRequest) -> Conversation {
        Conversation {
            topic_id: req.topic_id.clone(),
            last_seq: req.seq,
            is_partial: true,
            updated_at: req.created_at.clone(),
            ..Default::default()
        }
    }
}

impl From<&Topic> for Conversation {
    fn from(topic: &Topic) -> Conversation {
        Conversation {
            topic_id: topic.id.clone(),
            owner_id: topic.owner_id.clone(),
            last_seq: topic.last_seq,
            multiple: topic.multiple,
            source: topic.source.clone(),
            name: topic.name.clone(),
            icon: topic.icon.clone(),
            attendee: topic.attendee_id.clone(),
            is_partial: false,
            ..Default::default()
        }
    }
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConversationUpdateFields {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub sticky: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub mute: Option<bool>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remark: Option<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<Extra>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Tags>,
}
