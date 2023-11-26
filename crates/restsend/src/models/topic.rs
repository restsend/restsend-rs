use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TopicNotice {
    pub text: String,
    #[serde(default)]
    pub publisher: String,
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
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub remark: String,
    #[serde(default)]
    pub owner_id: String,
    #[serde(default)]
    pub attendee_id: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub admins: Vec<String>,
    #[serde(default)]
    pub members: u32,
    #[serde(default)]
    pub last_seq: u64,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notice: Option<TopicNotice>,
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
