use std::str::FromStr;

use crate::{request::ChatRequest, storage::StoreModel};

use super::{omit_empty, Content, Topic};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    pub owner_id: String,
    pub topic_id: String,

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
    pub unread: u64,

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
    pub cached_at: i64,
    #[serde(default)]
    pub is_partial: bool,
}

impl Conversation {
    pub fn new(topic_id: &str) -> Self {
        Conversation {
            topic_id: String::from(topic_id),
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
        self.cached_at
    }
}

impl From<&ChatRequest> for Conversation {
    fn from(req: &ChatRequest) -> Conversation {
        Conversation {
            topic_id: req.topic_id.clone(),
            last_seq: req.seq,
            attendee: req.attendee.clone(),
            is_partial: true,
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
