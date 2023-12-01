use super::omit_empty;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TopicNotice {
    pub text: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub publisher: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub updated_at: String,
}

impl TopicNotice {
    pub fn new(text: &str, publisher: &str, updated_at: &str) -> Self {
        TopicNotice {
            text: String::from(text),
            publisher: String::from(publisher),
            updated_at: String::from(updated_at),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Topic {
    // 群id
    pub id: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub name: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub icon: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub remark: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub owner_id: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub attendee_id: String,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub admins: Vec<String>,

    #[serde(skip_serializing_if = "omit_empty")]
    #[serde(default)]
    pub members: u32,

    #[serde(skip_serializing_if = "omit_empty")]
    #[serde(default)]
    pub last_seq: u64,

    #[serde(skip_serializing_if = "omit_empty")]
    #[serde(default)]
    pub multiple: bool,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub source: String,

    #[serde(skip_serializing_if = "omit_empty")]
    #[serde(default)]
    pub private: bool,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub created_at: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub updated_at: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub notice: Option<TopicNotice>,

    #[serde(skip_serializing_if = "omit_empty")]
    #[serde(default)]
    pub silent: bool,

    #[serde(skip)]
    pub cached_at: String,
}

impl Topic {
    pub fn new(topic_id: &str) -> Self {
        Topic {
            id: String::from(topic_id),
            ..Default::default()
        }
    }
}
