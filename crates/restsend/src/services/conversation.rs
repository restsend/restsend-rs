use serde::Serialize;

use super::{api_call, response::APISendResponse};
use crate::Result;
use crate::{
    models::{Conversation, ListChatLogResult, ListConversationResult},
    request::ChatRequest,
    services::LOGS_LIMIT,
    utils::now_millis,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchSyncChatLogs {
    pub topic_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_seq: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

pub async fn create_chat(endpoint: &str, token: &str, user_id: &str) -> Result<Conversation> {
    api_call(endpoint, &format!("/chat/create/{}", user_id), token, None).await
}

pub async fn get_conversations(
    endpoint: &str,
    token: &str,
    updated_at: &str,
    last_updated_at: Option<String>,
    offset: u32,
    limit: u32,
) -> Result<ListConversationResult> {
    let mut data = serde_json::json!({
        "offset": offset,
        "limit": limit,
    });
    if !updated_at.is_empty() {
        data["updatedAt"] = serde_json::json!(updated_at);
    }
    if let Some(last_updated_at) = last_updated_at {
        data["lastUpdatedAt"] = serde_json::json!(last_updated_at);
    }
    let now = now_millis();
    api_call(endpoint, "/chat/list", token, Some(data.to_string()))
        .await
        .map(|mut lr: ListConversationResult| {
            lr.items.iter_mut().for_each(|c| {
                c.cached_at = now;
                if !c.icon.is_empty() && !c.icon.starts_with("http") {
                    c.icon = format!("{}{}", endpoint.trim_end_matches('/'), c.icon);
                }
            });
            lr
        })
}

pub async fn get_conversation(endpoint: &str, token: &str, topic_id: &str) -> Result<Conversation> {
    api_call(endpoint, &format!("/chat/info/{}", topic_id), token, None)
        .await
        .map(|mut c: Conversation| {
            c.cached_at = now_millis();
            if !c.icon.is_empty() && !c.icon.starts_with("http") {
                c.icon = format!("{}{}", endpoint.trim_end_matches('/'), c.icon);
            }
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
) -> Result<Conversation> {
    api_call(
        endpoint,
        &format!("/chat/update/{}", topic_id),
        token,
        Some(data.to_string()),
    )
    .await
}

pub async fn set_conversation_remark(
    endpoint: &str,
    token: &str,
    topic_id: &str,
    remark: Option<String>,
) -> Result<Conversation> {
    let data = serde_json::json!({
        "remark": remark,
    });
    update_conversation(endpoint, token, topic_id, &data).await
}

pub async fn set_conversation_sticky(
    endpoint: &str,
    token: &str,
    topic_id: &str,
    sticky: bool,
) -> Result<Conversation> {
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
) -> Result<Conversation> {
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

pub async fn set_all_conversations_read(endpoint: &str, token: &str) -> Result<()> {
    api_call(endpoint, "/chat/readall", token, None)
        .await
        .map(|_: bool| ())
}

pub async fn clean_messages(endpoint: &str, token: &str, topic_id: &str) -> Result<()> {
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
    chatlog_ids: Vec<String>,
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
    last_seq: Option<i64>,
    limit: u32,
) -> Result<ListChatLogResult> {
    let mut data = serde_json::json!({
        "topicId": topic_id,
        "limit": limit.min(LOGS_LIMIT)
    });

    if last_seq.is_some() {
        data["lastSeq"] = serde_json::json!(last_seq);
    }

    api_call(
        endpoint,
        &format!("/chat/sync/{}", topic_id),
        token,
        Some(data.to_string()),
    )
    .await
    .map(|mut lr: ListChatLogResult| {
        lr.items.iter_mut().for_each(|c| {
            c.cached_at = now_millis();
        });
        lr
    })
}

pub async fn batch_get_chat_logs_desc(
    endpoint: &str,
    token: &str,
    conversations: Vec<BatchSyncChatLogs>,
) -> Result<Vec<ListChatLogResult>> {
    let data = serde_json::json!(conversations);

    api_call(endpoint, "/chat/batch_sync", token, Some(data.to_string()))
        .await
        .map(|mut lrs: Vec<ListChatLogResult>| {
            lrs.iter_mut().for_each(|lr| {
                lr.items.iter_mut().for_each(|c| {
                    c.cached_at = now_millis();
                });
            });
            lrs
        })
}

pub async fn send_request(
    endpoint: &str,
    token: &str,
    topic_id: &str,
    req: ChatRequest,
) -> Result<APISendResponse> {
    api_call(
        endpoint,
        &format!("/chat/send/{}", topic_id),
        token,
        Some(req.into()),
    )
    .await
}
