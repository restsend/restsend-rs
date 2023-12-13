use super::omit_empty;
use crate::storage::StoreModel;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Serialize, Debug, Clone, Default, uniffi::Record)]
pub struct AuthInfo {
    pub endpoint: String,
    pub user_id: String,
    pub avatar: String,
    pub name: String,
    pub token: String,
}

impl AuthInfo {
    pub fn new(endpoint: &str, user_id: &str, token: &str) -> Self {
        AuthInfo {
            endpoint: endpoint.to_string(),
            user_id: user_id.to_string(),
            token: token.to_string(),
            name: String::default(),
            avatar: String::default(),
        }
    }
}

#[derive(serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UserProfile {
    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub avatar: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub gender: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub city: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub region: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub country: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct User {
    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub user_id: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub name: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub avatar: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub public_key: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub remark: String,

    #[serde(skip_serializing_if = "omit_empty")]
    #[serde(default)]
    pub is_contact: bool,

    #[serde(skip_serializing_if = "omit_empty")]
    #[serde(default)]
    pub is_star: bool,

    #[serde(skip_serializing_if = "omit_empty")]
    #[serde(default)]
    pub is_blocked: bool,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub locale: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub city: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub country: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub source: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub created_at: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub gender: String, // f/female, m/male

    #[serde(default)]
    pub cached_at: i64,
    #[serde(default)]
    pub is_partial: bool,
}

impl User {
    pub fn new(user_id: &str) -> Self {
        User {
            user_id: String::from(user_id),
            is_partial: true,
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
        new_user.is_contact = user.is_contact;
        new_user.is_star = user.is_star;
        new_user.is_blocked = user.is_blocked;
        new_user.locale = user.locale.clone();
        new_user.city = user.city.clone();
        new_user.country = user.country.clone();
        new_user.source = user.source.clone();
        new_user.created_at = user.created_at.clone();
        new_user
    }
}

impl FromStr for User {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str::<User>(s)
    }
}

impl ToString for User {
    fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

impl StoreModel for User {
    fn sort_key(&self) -> i64 {
        self.cached_at as i64
    }
}
