use super::{Content, Topic};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    pub owner_id: String,
    pub topic_id: String,

    #[serde(default)]
    pub last_seq: u64,

    #[serde(default)]
    pub last_read_seq: u64,

    #[serde(default)]
    pub multiple: bool,

    #[serde(default)]
    pub attendee: String,

    #[serde(default)]
    pub name: String,

    #[serde(default)]
    pub icon: String,

    #[serde(default)]
    pub sticky: bool,

    #[serde(default)]
    pub mute: bool,

    #[serde(default)]
    pub source: String,

    #[serde(default)]
    pub unread: u64,

    #[serde(default)]
    pub last_sender_id: String,

    #[serde(default)]
    pub last_message: Option<Content>,

    #[serde(default)]
    pub last_message_at: String,

    #[serde(skip)]
    pub cached_at: String,
}

impl Conversation {
    pub fn new(topic_id: &str) -> Self {
        Conversation {
            topic_id: String::from(topic_id),
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
            ..Default::default()
        }
    }
}
