use super::conversation::Extra;
use restsend_macros::export_wasm_or_ffi;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[export_wasm_or_ffi(#[derive(uniffi::Record)])]
pub struct TopicMember {
    pub topic_id: String,
    pub user_id: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub source: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub silence_at: Option<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub joined_at: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub updated_at: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<Extra>,
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
