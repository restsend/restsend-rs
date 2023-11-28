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

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Upload {
    pub path: String,
    pub file_name: String,
    pub ext: String,
    pub size: i64,
}
