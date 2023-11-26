use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default)]
pub struct AuthInfo {
    pub endpoint: String,
    pub user_id: String,
    pub avatar: String,
    pub name: String,
    pub token: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct User {
    #[serde(default)]
    pub user_id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub avatar: String,
    #[serde(default)]
    pub public_key: String,
    #[serde(default)]
    pub remark: String,
    #[serde(default)]
    pub is_contact: bool,
    #[serde(default)]
    pub is_star: bool,
    #[serde(default)]
    pub is_blocked: bool,
    #[serde(default)]
    pub locale: String,
    #[serde(default)]
    pub city: String,
    #[serde(default)]
    pub country: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub gender: String, // f/female, m/male
    #[serde(skip)]
    pub cached_at: String,
}

impl User {
    pub fn new(user_id: &str) -> Self {
        User {
            user_id: String::from(user_id),
            ..Default::default()
        }
    }
    pub fn merge(&self, user: &User) -> User {
        let mut new_user = self.clone();
        if user.name != self.name {
            new_user.name = user.name.clone();
        }
        if user.avatar != String::default() {
            new_user.avatar = user.avatar.clone();
        }
        if user.public_key != String::default() {
            new_user.public_key = user.public_key.clone();
        }
        if user.remark != String::default() {
            new_user.remark = user.remark.clone();
        }
        if user.is_contact != false {
            new_user.is_contact = user.is_contact;
        }
        if user.is_star != false {
            new_user.is_star = user.is_star;
        }
        if user.is_blocked != false {
            new_user.is_blocked = user.is_blocked;
        }
        if user.locale != String::default() {
            new_user.locale = user.locale.clone();
        }
        if user.city != String::default() {
            new_user.city = user.city.clone();
        }
        if user.country != String::default() {
            new_user.country = user.country.clone();
        }
        if user.source != String::default() {
            new_user.source = user.source.clone();
        }
        if user.created_at != String::default() {
            new_user.created_at = user.created_at.clone();
        }
        new_user.cached_at = chrono::Local::now().timestamp_millis().to_string();
        new_user
    }
}
