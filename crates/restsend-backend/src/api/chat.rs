use axum::extract::{Path, State};
use axum::Json;
use chrono::Utc;
use serde_json::json;

use crate::api::auth_ctx::AuthCtx;
use crate::api::error::{ApiError, ApiResult};
use crate::app::AppState;
use crate::infra::event::{
    BackendEvent, ChatEvent, ConversationRemovedEvent, ConversationUpdateEvent, ReadEvent,
};
use crate::services::DomainError;
use crate::{
    ChatLogSyncForm, Content, ListConversationForm, ListConversationResult, OpenApiChatMessageForm,
    OpenApiSendMessageResponse, OpenApiUpdateConversationForm, RemoveMessagesForm,
};

pub(crate) fn conversation_update_fields(
    form: &OpenApiUpdateConversationForm,
) -> serde_json::Value {
    let mut fields = serde_json::Map::new();
    if let Some(sticky) = form.sticky {
        fields.insert("sticky".to_string(), json!(sticky));
    }
    if let Some(mute) = form.mute {
        fields.insert("mute".to_string(), json!(mute));
    }
    if let Some(remark) = form.remark.clone() {
        fields.insert("remark".to_string(), json!(remark));
    }
    serde_json::Value::Object(fields)
}

pub(crate) fn build_conversation_update_payload(
    owner_id: &str,
    topic_id: &str,
    fields: &serde_json::Value,
) -> String {
    serde_json::to_string(&json!({
        "type": "chat",
        "topicId": topic_id,
        "chatId": format!("conv-updated-{}", uuid::Uuid::new_v4().simple()),
        "attendee": owner_id,
        "createdAt": Utc::now().to_rfc3339(),
        "content": {
            "type": "conversation.update",
            "text": fields.to_string(),
            "unreadable": true,
        }
    }))
    .unwrap_or_default()
}

pub(crate) fn build_conversation_removed_payload(owner_id: &str, topic_id: &str) -> String {
    serde_json::to_string(&json!({
        "type": "chat",
        "topicId": topic_id,
        "chatId": format!("conv-removed-{}", uuid::Uuid::new_v4().simple()),
        "attendee": owner_id,
        "createdAt": Utc::now().to_rfc3339(),
        "content": {
            "type": "conversation.removed",
            "text": "",
            "unreadable": true,
        }
    }))
    .unwrap_or_default()
}

pub async fn chat_create_with_user(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(userid): Path<String>,
) -> ApiResult<Json<crate::Conversation>> {
    let topic_id = dm_topic_id(auth.user_id(), &userid);
    let _ = state
        .topic_service
        .create_topic(
            Some(topic_id.clone()),
            crate::OpenApiCreateTopicForm {
                sender_id: auth.user_id().to_string(),
                members: vec![auth.user_id().to_string(), userid.clone()],
                name: format!("DM with {userid}"),
                multiple: Some(false),
                ..crate::OpenApiCreateTopicForm::default()
            },
        )
        .await;

    let conv = state
        .conversation_service
        .create_or_update(crate::Conversation {
            owner_id: auth.user_id().to_string(),
            topic_id,
            unread: 0,
            ..crate::Conversation::default()
        })
        .await
        .map_err(map_domain_error)?;
    Ok(Json(conv))
}

pub async fn chat_list(
    State(state): State<AppState>,
    auth: AuthCtx,
    payload: Option<Json<ListConversationForm>>,
) -> ApiResult<Json<ListConversationResult>> {
    let form = payload.map(|v| v.0).unwrap_or_default();
    let offset = form.offset.unwrap_or(0);
    let limit = form.limit.unwrap_or(20).clamp(1, 200);
    let items = state
        .conversation_service
        .list_by_user(auth.user_id(), offset, limit)
        .await
        .map_err(map_domain_error)?;
    let total = items.len() as i64;
    Ok(Json(ListConversationResult {
        total,
        has_more: total as u64 >= limit,
        offset,
        items,
        removed: Vec::new(),
        last_updated_at: None,
        last_removed_at: None,
    }))
}

pub async fn chat_info(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(topic_id): Path<String>,
) -> ApiResult<Json<crate::Conversation>> {
    let conv = state
        .conversation_service
        .get_conversation(auth.user_id(), &topic_id)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(conv))
}

pub async fn chat_remove(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(topic_id): Path<String>,
) -> ApiResult<Json<bool>> {
    state
        .conversation_service
        .remove_conversation(auth.user_id(), &topic_id)
        .await
        .map_err(map_domain_error)?;
    state.event_bus.publish(BackendEvent::ConversationRemoved(
        ConversationRemovedEvent {
            topic_id: topic_id.clone(),
            owner_id: auth.user_id().to_string(),
            source: "api".to_string(),
        },
    ));

    let payload = build_conversation_removed_payload(auth.user_id(), &topic_id);
    crate::api::push::broadcast_to_user(&state, auth.user_id(), &payload).await;

    Ok(Json(true))
}

pub async fn chat_update(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(topic_id): Path<String>,
    Json(form): Json<crate::OpenApiUpdateConversationForm>,
) -> ApiResult<Json<crate::Conversation>> {
    let fields = conversation_update_fields(&form);
    let conv = state
        .conversation_service
        .update_conversation(auth.user_id(), &topic_id, form)
        .await
        .map_err(map_domain_error)?;
    if !fields.as_object().is_some_and(|v| v.is_empty()) {
        state
            .event_bus
            .publish(BackendEvent::ConversationUpdate(ConversationUpdateEvent {
                topic_id: topic_id.clone(),
                owner_id: auth.user_id().to_string(),
                fields: fields.clone(),
            }));
        let payload = build_conversation_update_payload(auth.user_id(), &topic_id, &fields);
        crate::api::push::broadcast_to_user(&state, auth.user_id(), &payload).await;
    }
    Ok(Json(conv))
}

pub async fn chat_read(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(topic_id): Path<String>,
) -> ApiResult<Json<bool>> {
    let conv = state
        .conversation_service
        .mark_read(auth.user_id(), &topic_id, None)
        .await
        .map_err(map_domain_error)?;
    state
        .event_bus
        .publish(BackendEvent::ConversationUpdate(ConversationUpdateEvent {
            topic_id,
            owner_id: auth.user_id().to_string(),
            fields: serde_json::to_value(&conv).unwrap_or_else(|_| serde_json::json!({})),
        }));
    state.event_bus.publish(BackendEvent::Read(ReadEvent {
        topic_id: conv.topic_id,
        user_id: auth.user_id().to_string(),
        last_read_seq: conv.last_read_seq,
    }));
    Ok(Json(true))
}

pub async fn chat_unread(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(topic_id): Path<String>,
) -> ApiResult<Json<bool>> {
    let _ = state
        .conversation_service
        .mark_unread(auth.user_id(), &topic_id)
        .await
        .map_err(map_domain_error)?;
    state
        .event_bus
        .publish(BackendEvent::ConversationUpdate(ConversationUpdateEvent {
            topic_id: topic_id.clone(),
            owner_id: auth.user_id().to_string(),
            fields: serde_json::json!({"markUnread": true}),
        }));
    let payload = build_conversation_update_payload(
        auth.user_id(),
        &topic_id,
        &serde_json::json!({"markUnread": true}),
    );
    crate::api::push::broadcast_to_user(&state, auth.user_id(), &payload).await;
    Ok(Json(true))
}

pub async fn chat_read_all(State(state): State<AppState>, auth: AuthCtx) -> ApiResult<Json<bool>> {
    let _ = state
        .conversation_service
        .mark_all_read(auth.user_id())
        .await
        .map_err(map_domain_error)?;
    state
        .event_bus
        .publish(BackendEvent::ConversationUpdate(ConversationUpdateEvent {
            topic_id: String::new(),
            owner_id: auth.user_id().to_string(),
            fields: serde_json::json!({"allRead": true}),
        }));
    Ok(Json(true))
}

pub async fn chat_sync(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(topic_id): Path<String>,
    Json(form): Json<ChatLogSyncForm>,
) -> ApiResult<Json<crate::ChatLogSyncResult>> {
    let st = std::time::Instant::now();
    let start_seq = state
        .conversation_service
        .get_conversation(auth.user_id(), &topic_id)
        .await
        .ok()
        .map(|conv| conv.start_seq)
        .unwrap_or_default();
    let r = state
        .chat_service
        .topic_logs(&topic_id, &form)
        .await
        .map_err(map_domain_error)?;
    let items = r
        .items
        .into_iter()
        .filter(|item| item.seq > start_seq)
        .map(|mut item| {
            if item.deleted_by.iter().any(|v| v == auth.user_id()) {
                item.content = Content::default();
            }
            item
        })
        .collect();
    let result = crate::ChatLogSyncResult { items, ..r };
    tracing::info!(
        user_id = %auth.user_id(),
        topic_id = %topic_id,
        limit = form.limit,
        has_more = result.has_more,
        item_count = result.items.len(),
        elapsed_ms = st.elapsed().as_millis() as u64,
        "chat sync completed"
    );
    Ok(Json(result))
}

pub async fn chat_batch_sync(
    State(state): State<AppState>,
    auth: AuthCtx,
    Json(forms): Json<Vec<ChatLogSyncForm>>,
) -> ApiResult<Json<Vec<crate::ChatLogSyncResult>>> {
    let st = std::time::Instant::now();
    let req_count = forms.len();
    let mut out = Vec::new();
    for form in forms {
        if let Some(topic_id) = form.topic_id.clone() {
            if let Ok(r) = state.chat_service.topic_logs(&topic_id, &form).await {
                let start_seq = state
                    .conversation_service
                    .get_conversation(auth.user_id(), &topic_id)
                    .await
                    .ok()
                    .map(|c| c.start_seq)
                    .unwrap_or_default();
                out.push(crate::ChatLogSyncResult {
                    items: r
                        .items
                        .into_iter()
                        .filter(|item| item.seq > start_seq)
                        .map(|mut item| {
                            if item.deleted_by.iter().any(|v| v == auth.user_id()) {
                                item.content = Content::default();
                            }
                            item
                        })
                        .collect(),
                    ..r
                });
            }
        }
    }
    tracing::info!(
        user_id = %auth.user_id(),
        req_count = req_count,
        resp_count = out.len(),
        elapsed_ms = st.elapsed().as_millis() as u64,
        "chat batch sync completed"
    );
    Ok(Json(out))
}

pub async fn chat_send(
    State(state): State<AppState>,
    auth: AuthCtx,
    Json(form): Json<OpenApiChatMessageForm>,
) -> ApiResult<Json<OpenApiSendMessageResponse>> {
    if form.r#type != "chat" {
        return Err(ApiError::bad_request("type must be chat"));
    }
    let (_effective_form, _topic_id, resp) =
        send_chat_message(&state, auth.user_id(), form).await?;
    let payload = serde_json::to_string(&resp).unwrap_or_default();
    crate::api::push::broadcast_to_user(&state, auth.user_id(), &payload).await;
    Ok(Json(resp))
}

pub async fn chat_send_to_topic(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(topic_id): Path<String>,
    Json(mut form): Json<OpenApiChatMessageForm>,
) -> ApiResult<Json<OpenApiSendMessageResponse>> {
    form.topic_id = topic_id.clone();
    let (_effective_form, _topic_id, resp) =
        send_chat_message(&state, auth.user_id(), form).await?;
    let payload = serde_json::to_string(&resp).unwrap_or_default();
    crate::api::push::broadcast_to_user(&state, auth.user_id(), &payload).await;
    Ok(Json(resp))
}

pub async fn chat_remove_messages(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(topic_id): Path<String>,
    Json(form): Json<RemoveMessagesForm>,
) -> ApiResult<Json<bool>> {
    if form.chat_ids.is_empty() {
        return Err(ApiError::bad_request("ids is required"));
    }
    state
        .chat_service
        .remove_conversation_messages(&topic_id, auth.user_id(), &form.chat_ids)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(true))
}

pub async fn chat_clear_messages(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(topic_id): Path<String>,
) -> ApiResult<Json<bool>> {
    let last_seq = state
        .chat_service
        .clear_conversation_messages(&topic_id)
        .await
        .map_err(map_domain_error)?;
    let _ = state
        .conversation_service
        .clear_messages(auth.user_id(), &topic_id, last_seq)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(true))
}

fn dm_topic_id(a: &str, b: &str) -> String {
    if a <= b {
        format!("{a}:{b}")
    } else {
        format!("{b}:{a}")
    }
}

async fn update_topic_conversations(
    state: &AppState,
    topic_id: &str,
    resp: &OpenApiSendMessageResponse,
    message: &OpenApiChatMessageForm,
) {
    let content = message.content.clone().or_else(|| {
        if message.message.is_empty() {
            None
        } else {
            Some(Content {
                content_type: if message.r#type.is_empty() {
                    "chat".to_string()
                } else {
                    message.r#type.clone()
                },
                text: message.message.clone(),
                ..Content::default()
            })
        }
    });

    if let Ok(members) = state.topic_service.list_members(topic_id).await {
        for user_id in members {
            let unread = if user_id == resp.sender_id { 0 } else { 1 };
            let _ = state
                .conversation_service
                .create_or_update(crate::Conversation {
                    owner_id: user_id,
                    topic_id: topic_id.to_string(),
                    unread,
                    last_seq: resp.seq,
                    last_sender_id: resp.sender_id.clone(),
                    last_message: content.clone(),
                    last_message_at: Utc::now().to_rfc3339(),
                    last_message_seq: Some(resp.seq),
                    updated_at: Utc::now().to_rfc3339(),
                    ..crate::Conversation::default()
                })
                .await;
        }
    }
}

async fn ensure_dm_topic(
    state: &AppState,
    user_id: &str,
    attendee: &str,
) -> Result<String, ApiError> {
    let topic_id = dm_topic_id(user_id, attendee);
    if state.topic_service.get_by_id(&topic_id).await.is_err() {
        let _ = state
            .topic_service
            .create_topic(
                Some(topic_id.clone()),
                crate::OpenApiCreateTopicForm {
                    sender_id: user_id.to_string(),
                    members: vec![user_id.to_string(), attendee.to_string()],
                    name: format!("DM with {attendee}"),
                    multiple: Some(false),
                    ..crate::OpenApiCreateTopicForm::default()
                },
            )
            .await
            .map_err(map_domain_error)?;
    }
    Ok(topic_id)
}

pub(crate) async fn send_chat_message(
    state: &AppState,
    user_id: &str,
    form: OpenApiChatMessageForm,
) -> Result<(OpenApiChatMessageForm, String, OpenApiSendMessageResponse), ApiError> {
    let (topic_id, mut effective_form) = if !form.topic_id.is_empty() {
        (form.topic_id.clone(), form)
    } else if !form.attendee.is_empty() {
        let topic_id = ensure_dm_topic(state, user_id, &form.attendee).await?;
        let mut form = form;
        form.topic_id = topic_id.clone();
        (topic_id, form)
    } else {
        return Err(ApiError::bad_request("topicId or attendee is required"));
    };

    if effective_form.r#type.is_empty() {
        effective_form.r#type = "chat".to_string();
    }

    let resp = match effective_form
        .content
        .as_ref()
        .map(|content| content.content_type.as_str())
    {
        Some("recall") => {
            state
                .chat_service
                .recall_in_topic(&topic_id, user_id, &effective_form)
                .await
        }
        _ => {
            state
                .chat_service
                .send_to_topic(&topic_id, user_id, &effective_form)
                .await
        }
    }
    .map_err(map_domain_error)?;
    update_topic_conversations(state, &topic_id, &resp, &effective_form).await;
    state.event_bus.publish(BackendEvent::Chat(ChatEvent {
        topic_id: topic_id.clone(),
        sender_id: user_id.to_string(),
        chat_id: resp.chat_id.clone(),
        seq: resp.seq,
        created_at: effective_form
            .created_at
            .clone()
            .unwrap_or_else(|| Utc::now().to_rfc3339()),
        content: effective_form.content.clone().or_else(|| {
            if effective_form.message.is_empty() {
                None
            } else {
                Some(Content {
                    content_type: effective_form.r#type.clone(),
                    text: effective_form.message.clone(),
                    ..Content::default()
                })
            }
        }),
    }));
    Ok((effective_form, topic_id, resp))
}

fn map_domain_error(err: DomainError) -> ApiError {
    match err {
        DomainError::NotFound => ApiError::NotFound,
        DomainError::Validation(msg) => ApiError::bad_request(msg),
        DomainError::Conflict => ApiError::bad_request("resource already exists"),
        DomainError::Forbidden => ApiError::Unauthorized,
        DomainError::Storage(msg) => ApiError::internal(msg),
    }
}
