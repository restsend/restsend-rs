use super::api_call;
use crate::{
    models::{Conversation, ListChatLogResult, ListConversationResult},
    services::LOGS_LIMIT,
};
use anyhow::Result;

pub async fn get_conversations(
    endpoint: &str,
    token: &str,
    updated_at: &str,
    limit: u32,
) -> Result<ListConversationResult> {
    let data = serde_json::json!({
        "limit": limit,
        "updatedAt": updated_at,
    })
    .to_string();

    let now = chrono::Utc::now().timestamp();

    api_call(endpoint, "/chat/list", token, Some(data))
        .await
        .map(|mut lr: ListConversationResult| {
            lr.items.iter_mut().for_each(|c| {
                c.cached_at = now;
            });
            lr
        })
}

pub async fn get_conversation(endpoint: &str, token: &str, topic_id: &str) -> Result<Conversation> {
    api_call(endpoint, &format!("/chat/info/{}", topic_id), token, None)
        .await
        .map(|mut c: Conversation| {
            c.cached_at = chrono::Utc::now().timestamp();
            c
        })
}

pub async fn remove_conversation(endpoint: &str, token: &str, topic_id: &str) -> Result<()> {
    api_call(endpoint, &format!("/chat/remove/{}", topic_id), token, None)
        .await
        .map(|_: bool| ())
}

pub async fn update_conversation(
    endpoint: &str,
    token: &str,
    topic_id: &str,
    data: &serde_json::Value,
) -> Result<()> {
    api_call(
        endpoint,
        &format!("/chat/update/{}", topic_id),
        token,
        Some(data.to_string()),
    )
    .await
    .map(|_: bool| ())
}

pub async fn set_conversation_sticky(
    endpoint: &str,
    token: &str,
    topic_id: &str,
    sticky: bool,
) -> Result<()> {
    let data = serde_json::json!({
        "sticky": sticky,
    });
    update_conversation(endpoint, token, topic_id, &data).await
}

pub async fn set_conversation_mute(
    endpoint: &str,
    token: &str,
    topic_id: &str,
    mute: bool,
) -> Result<()> {
    let data = serde_json::json!({
        "mute": mute,
    });
    update_conversation(endpoint, token, topic_id, &data).await
}

pub async fn set_conversation_read(endpoint: &str, token: &str, topic_id: &str) -> Result<()> {
    api_call(endpoint, &format!("/chat/read/{}", topic_id), token, None)
        .await
        .map(|_: bool| ())
}

pub async fn clean_history(endpoint: &str, token: &str, topic_id: &str) -> Result<()> {
    api_call(
        endpoint,
        &format!("/chat/clear_messages/{}", topic_id),
        token,
        None,
    )
    .await
    .map(|_: bool| ())
}

pub async fn remove_messages(
    endpoint: &str,
    token: &str,
    topic_id: &str,
    chatlog_ids: Vec<&str>,
) -> Result<()> {
    let data = serde_json::json!({ "ids": chatlog_ids }).to_string();
    api_call(
        endpoint,
        &format!("/chat/remove_messages/{}", topic_id),
        token,
        Some(data),
    )
    .await
    .map(|_: bool| ())
}

pub async fn get_chat_logs_desc(
    endpoint: &str,
    token: &str,
    topic_id: &str,
    start_seq: u64,
    end_seq: u64,
    limit: u32,
) -> Result<(ListChatLogResult, u64)> {
    let data = serde_json::json!({
        "topicId": topic_id,
        "lastSeq": start_seq,
        "maxSeq": end_seq,
        "limit": limit.min(LOGS_LIMIT)
    })
    .to_string();

    let now = chrono::Utc::now().timestamp();
    api_call(
        endpoint,
        &format!("/chat/sync/{}", topic_id),
        token,
        Some(data),
    )
    .await
    .map(|mut lr: ListChatLogResult| {
        lr.items.iter_mut().for_each(|c| {
            c.cached_at = now;
        });
        let last_seq = lr.items.iter().map(|c| c.seq).max().unwrap_or(0);
        (lr, last_seq)
    })
}
