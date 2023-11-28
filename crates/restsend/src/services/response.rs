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
#[allow(unused)]
pub struct Common {
    ok: bool,
}
