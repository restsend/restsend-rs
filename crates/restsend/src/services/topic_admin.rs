use super::api_call;
use crate::models::TopicKnock;
use crate::Result;

pub async fn update_topic_notice(
    endpoint: &str,
    token: &str,
    topic_id: &str,
    text: &str,
) -> Result<()> {
    let data = serde_json::json!({
        "topicId": topic_id,
        "text": text
    })
    .to_string();
    api_call(
        endpoint,
        &format!("/topic/admin/notice/{}", topic_id),
        token,
        Some(data),
    )
    .await
    .map(|_: bool| ())
}

pub async fn silent_topic(
    endpoint: &str,
    token: &str,
    topic_id: &str,
    duration: Option<String>,
) -> Result<()> {
    let data = serde_json::json!({ "duration": duration }).to_string();
    api_call(
        endpoint,
        &format!("/topic/admin/silent_topic/{}", topic_id),
        token,
        Some(data),
    )
    .await
    .map(|_: bool| ())
}

pub async fn silent_topic_member(
    endpoint: &str,
    token: &str,
    topic_id: &str,
    user_id: &str,
    duration: Option<String>,
) -> Result<()> {
    let data = serde_json::json!({ "duration": duration }).to_string();
    api_call(
        endpoint,
        &format!("/topic/admin/silent_topic/{}/{}", topic_id, user_id),
        token,
        Some(data),
    )
    .await
    .map(|_: bool| ())
}

pub async fn quit_topic(endpoint: &str, token: &str, topic_id: &str) -> Result<()> {
    api_call(endpoint, &format!("/topic/quit/{}", topic_id), token, None)
        .await
        .map(|_: bool| ())
}

pub async fn dismiss_topic(endpoint: &str, token: &str, topic_id: &str) -> Result<()> {
    api_call(
        endpoint,
        &format!("/topic/dismiss/{}", topic_id),
        token,
        None,
    )
    .await
    .map(|_: bool| ())
}

pub async fn get_topic_knocks(
    endpoint: &str,
    token: &str,
    topic_id: &str,
) -> Result<Vec<TopicKnock>> {
    api_call(
        endpoint,
        &format!("/topic/admin/list_knock/{}", topic_id),
        token,
        None,
    )
    .await
}

pub async fn accept_topic_join(
    endpoint: &str,
    token: &str,
    topic_id: &str,
    user_id: &str,
    memo: &str,
) -> Result<()> {
    let data = serde_json::json!({ "memo": memo }).to_string();
    api_call(
        endpoint,
        &format!("/topic/admin/knock/accept/{}/{}", topic_id, user_id),
        token,
        Some(data),
    )
    .await
    .map(|_: bool| ())
}

pub async fn decline_topic_join(
    endpoint: &str,
    token: &str,
    topic_id: &str,
    user_id: &str,
    message: Option<String>,
) -> Result<()> {
    let data = serde_json::json!({
        "message": message,
    })
    .to_string();

    api_call(
        endpoint,
        &format!("/topic/admin/knock/reject/{}/{}", topic_id, user_id),
        token,
        Some(data),
    )
    .await
    .map(|_: bool| ())
}

pub async fn remove_topic_member(
    endpoint: &str,
    token: &str,
    topic_id: &str,
    user_id: &str,
) -> Result<()> {
    api_call(
        endpoint,
        &format!("/topic/admin/kickout/{}/{}", topic_id, user_id),
        token,
        None,
    )
    .await
    .map(|_: bool| ())
}

pub async fn update_topic(
    endpoint: &str,
    token: &str,
    topic_id: &str,
    name: Option<String>,
    icon: Option<String>,
) -> Result<()> {
    let data = serde_json::json!({
        "name":name,
        "icon":icon,
    })
    .to_string();

    api_call(
        endpoint,
        &format!("/topic/admin/update/{}", topic_id),
        token,
        Some(data),
    )
    .await
    .map(|_: bool| ())
}

pub async fn add_topic_admin(
    endpoint: &str,
    token: &str,
    topic_id: &str,
    user_id: &str,
) -> Result<()> {
    api_call(
        endpoint,
        &format!("/topic/admin/add_admin/{}/{}", topic_id, user_id),
        token,
        None,
    )
    .await
    .map(|_: bool| ())
}

pub async fn remove_topic_admin(
    endpoint: &str,
    token: &str,
    topic_id: &str,
    user_id: &str,
) -> Result<()> {
    api_call(
        endpoint,
        &format!("/topic/admin/remove_admin/{}/{}", topic_id, user_id),
        token,
        None,
    )
    .await
    .map(|_: bool| ())
}

pub async fn transfer_topic(
    endpoint: &str,
    token: &str,
    topic_id: &str,
    user_id: &str,
) -> Result<()> {
    api_call(
        endpoint,
        &format!("/topic/admin/transfer/{}/{}", topic_id, user_id),
        token,
        None,
    )
    .await
    .map(|_: bool| ())
}
