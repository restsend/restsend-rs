#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Login {
    pub email: String,
    #[serde(default)]
    pub display_name: String,
    pub token: String,
    #[serde(default)]
    pub profile: crate::models::UserProfile,
}

#[derive(serde::Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Upload {
    pub path: String,
    pub file_name: String,
    #[serde(default)]
    pub thumbnail: String,
    #[serde(default)]
    pub ext: String,
    pub size: u64,
}
