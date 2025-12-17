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
    #[serde(rename = "type")]
    pub tag_type: String,

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

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub last_read_at: Option<String>,

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

    #[serde(default)]
    #[serde(alias = "unreadCount")]
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

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub last_message_seq: Option<i64>,

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
    pub topic_owner_id: Option<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic_created_at: Option<String>,

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

    pub fn merge_local_read_state(&mut self, local: &Conversation) {
        if local.last_read_seq > self.last_read_seq {
            self.last_read_seq = local.last_read_seq;
            self.last_read_at = local.last_read_at.clone();
            self.unread = local.unread;
        } else if local.last_read_seq == self.last_read_seq
            && self.last_read_at.is_none()
            && local.last_read_at.is_some()
        {
            self.last_read_at = local.last_read_at.clone();
        }

        if self.last_read_at.is_none() && local.last_read_at.is_some() {
            self.last_read_at = local.last_read_at.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Conversation, Topic};

    #[test]
    fn merge_local_prefers_newer_local_state() {
        let mut remote = Conversation {
            last_seq: 120,
            last_read_seq: 40,
            unread: 80,
            ..Conversation::default()
        };

        let local = Conversation {
            last_read_seq: 80,
            last_read_at: Some("2024-01-01T00:00:00Z".to_string()),
            unread: 0,
            ..Conversation::default()
        };

        remote.merge_local_read_state(&local);

        assert_eq!(remote.last_read_seq, 80);
        assert_eq!(remote.last_read_at, local.last_read_at);
        assert_eq!(remote.unread, 0);
    }

    #[test]
    fn merge_local_keeps_remote_state_when_newer() {
        let mut remote = Conversation {
            last_read_seq: 90,
            unread: 5,
            ..Conversation::default()
        };

        let local = Conversation {
            last_read_seq: 80,
            last_read_at: Some("2024-01-02T00:00:00Z".to_string()),
            unread: 1,
            ..Conversation::default()
        };

        remote.merge_local_read_state(&local);

        assert_eq!(remote.last_read_seq, 90);
        assert_eq!(remote.unread, 5);
        assert_eq!(
            remote.last_read_at,
            Some("2024-01-02T00:00:00Z".to_string())
        );
    }

    #[test]
    fn conversation_from_topic_copies_topic_owner() {
        let topic = Topic {
            id: "topic-id".to_string(),
            owner_id: "owner-1".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            ..Topic::default()
        };

        let conversation = Conversation::from(&topic);

        assert_eq!(conversation.owner_id, "owner-1");
        assert_eq!(conversation.topic_owner_id.as_deref(), Some("owner-1"));
        assert_eq!(
            conversation.topic_created_at.as_deref(),
            Some("2024-01-01T00:00:00Z")
        );
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
            topic_owner_id: if topic.owner_id.is_empty() {
                None
            } else {
                Some(topic.owner_id.clone())
            },
            topic_created_at: if topic.created_at.is_empty() {
                None
            } else {
                Some(topic.created_at.clone())
            },
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

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mark_unread: Option<bool>,
}
