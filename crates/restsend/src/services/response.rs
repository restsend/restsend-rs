use restsend_macros::export_wasm_or_ffi;

use crate::models::UserProfile;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Login {
    pub email: String,
    #[serde(default)]
    pub display_name: String,
    pub token: String,
    #[serde(default)]
    pub profile: UserProfile,
}

#[derive(serde::Deserialize, serde::Serialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
#[export_wasm_or_ffi(#[derive(uniffi::Record)])]
pub struct Upload {
    pub path: String,
    pub file_name: String,
    #[serde(default)]
    pub thumbnail: String,
    #[serde(default)]
    pub ext: String,
    pub size: u64,
}

#[derive(serde::Deserialize, Default, Clone, Debug)]
#[serde(rename_all = "camelCase")]
#[export_wasm_or_ffi(#[derive(uniffi::Record)])]
pub struct APISendResponse {
    #[serde(default)]
    pub sender_id: String,

    #[serde(default)]
    pub topic_id: String,

    #[serde(default)]
    pub attendee_id: String,

    pub chat_id: String,
    pub code: u16,
    pub seq: i64,

    #[serde(default)]
    pub message: String,

    #[serde(default)]
    pub usage: i64,
}
