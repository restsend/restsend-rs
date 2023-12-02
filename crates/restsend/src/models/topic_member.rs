use serde::{de, Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct TopicMember {
    pub topic_id: String,
    pub user_id: String,
    #[serde(default)]
    pub is_owner: bool,
    #[serde(default)]
    pub is_admin: bool,
    #[serde(default)]
    pub remark: String,
    #[serde(default)]
    pub silent: bool,
    #[serde(default)]
    pub joined_at: String,

    #[serde(skip)]
    pub cached_at: i64,
}

impl TopicMember {
    pub fn new(topic_id: &str, user_id: &str) -> Self {
        TopicMember {
            topic_id: String::from(topic_id),
            user_id: String::from(user_id),
            ..Default::default()
        }
    }
}
