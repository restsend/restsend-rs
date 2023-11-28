use crate::models::{User, UserProfile};
use anyhow::Result;
use futures_util::FutureExt;

use super::api_call;

pub async fn get_user(endpoint: &str, token: &str, user_id: &str) -> Result<User> {
    api_call::<User>(
        endpoint,
        &format!("/api/profile/{}", user_id),
        Some(token),
        None,
    )
    .await
    .and_then(|mut user| {
        user.cached_at = chrono::Utc::now().to_rfc3339();
        if !user.avatar.is_empty() && !user.avatar.starts_with("http") {
            let endpoint = endpoint.trim_end_matches('/');
            user.avatar = format!("{}{}", endpoint, user.avatar);
        }
        Ok(user)
    })
}

pub async fn set_user_block(
    endpoint: &str,
    token: &str,
    user_id: String,
    block: bool,
) -> Result<()> {
    let action = if block { "block" } else { "unblock" };
    api_call::<bool>(
        endpoint,
        &format!("/api/{}/{}", action, user_id),
        Some(token),
        None,
    )
    .map(|_| Ok(()))
    .await
}

pub async fn set_user_remark(
    endpoint: &str,
    token: &str,
    user_id: String,
    remark: String,
) -> Result<()> {
    let vals = serde_json::json!({ "remark": remark });
    api_call::<bool>(
        endpoint,
        &format!("/api/relation/{}", user_id),
        Some(token),
        Some(vals.to_string()),
    )
    .map(|_| Ok(()))
    .await
}

pub async fn set_user_favorite(
    endpoint: &str,
    token: &str,
    user_id: String,
    favorite: bool,
) -> Result<()> {
    let vals = serde_json::json!({ "favorite": favorite });
    api_call::<bool>(
        endpoint,
        &format!("/api/relation/{}", user_id),
        Some(token),
        Some(vals.to_string()),
    )
    .map(|_| Ok(()))
    .await
}

pub async fn set_allow_guest_chat(endpoint: &str, token: &str, allowed: bool) -> Result<()> {
    let vals = serde_json::json!({ "allowGuest": allowed });
    api_call::<bool>(
        endpoint,
        "/api/profile/update",
        Some(token),
        Some(vals.to_string()),
    )
    .map(|_| Ok(()))
    .await
}
