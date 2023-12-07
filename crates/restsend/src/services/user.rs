use super::api_call;
use crate::Result;
use crate::{models::User, utils::now_timestamp};

pub async fn get_user(endpoint: &str, token: &str, user_id: &str) -> Result<User> {
    api_call(endpoint, &format!("/profile/{}", user_id), token, None)
        .await
        .map(|mut user: User| {
            user.cached_at = now_timestamp();
            if !user.avatar.is_empty() && !user.avatar.starts_with("http") {
                user.avatar = format!("{}{}", endpoint.trim_end_matches('/'), user.avatar);
            }
            user
        })
}

pub async fn get_users(endpoint: &str, token: &str, user_ids: Vec<&str>) -> Result<Vec<User>> {
    let data = serde_json::json!({
        "userIds": user_ids,
    })
    .to_string();

    api_call(endpoint, "/profile", token, Some(data))
        .await
        .map(|mut users: Vec<User>| {
            users.iter_mut().for_each(|user| {
                user.cached_at = now_timestamp();
                if !user.avatar.is_empty() && !user.avatar.starts_with("http") {
                    user.avatar = format!("{}{}", endpoint.trim_end_matches('/'), user.avatar);
                }
            });
            users
        })
}

pub async fn set_user_block(endpoint: &str, token: &str, user_id: &str, block: bool) -> Result<()> {
    let action = if block { "block" } else { "unblock" };
    api_call(endpoint, &format!("/{}/{}", action, user_id), token, None)
        .await
        .map(|_: bool| ())
}

pub async fn set_user_remark(
    endpoint: &str,
    token: &str,
    user_id: &str,
    remark: &str,
) -> Result<()> {
    let data = serde_json::json!({ "remark": remark }).to_string();
    api_call(
        endpoint,
        &format!("/relation/{}", user_id),
        token,
        Some(data),
    )
    .await
    .map(|_: bool| ())
}

pub async fn set_user_star(
    endpoint: &str,
    token: &str,
    user_id: &str,
    favorite: bool,
) -> Result<()> {
    let data = serde_json::json!({ "favorite": favorite }).to_string();
    api_call(
        endpoint,
        &format!("/relation/{}", user_id),
        token,
        Some(data),
    )
    .await
    .map(|_: bool| ())
}

pub async fn set_allow_guest_chat(endpoint: &str, token: &str, allowed: bool) -> Result<()> {
    let data = serde_json::json!({ "allowGuest": allowed }).to_string();
    api_call(endpoint, "/profile/update", token, Some(data))
        .await
        .map(|_: bool| ())
}
