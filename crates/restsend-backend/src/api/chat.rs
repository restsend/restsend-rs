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
    let (self_topic, _peer_topic) = state
        .topic_service
        .get_pair_topics(auth.user_id(), &userid)
        .await
        .map_err(map_domain_error)?;

    let conv = state
        .conversation_service
        .create_or_update(crate::Conversation {
            owner_id: auth.user_id().to_string(),
            topic_id: self_topic.id,
            attendee: userid.clone(),
            name: userid,
            kind: "dm".to_string(),
            members: 2,
            multiple: false,
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
    let limit = form.limit.unwrap_or(20).clamp(1, 1000);
    let category = form.category.trim().to_string();

    // Incremental sync mode (Go semantics): `updatedAt >=` cursor with
    // `limit + 1` hasMore probe, plus the soft-deleted removal list.
    if let Some(updated_at) = form.updated_at.as_deref().filter(|v| !v.is_empty()) {
        let (items, has_more) = state
            .conversation_service
            .list_updated_since(
                auth.user_id(),
                updated_at,
                form.last_updated_at.as_deref(),
                limit,
                &category,
            )
            .await
            .map_err(map_domain_error)?;
        let removed = state
            .conversation_service
            .list_removed(auth.user_id(), form.last_removed_at.as_deref(), 1000)
            .await
            .map_err(map_domain_error)?;
        return Ok(Json(ListConversationResult {
            total: items.len() as i64,
            has_more,
            offset,
            items,
            removed,
            last_updated_at: None,
            last_removed_at: None,
            categories: None,
        }));
    }

    let items = state
        .conversation_service
        .list_by_user_filtered(auth.user_id(), offset, limit, &category)
        .await
        .map_err(map_domain_error)?;
    let total = items.len() as i64;

    // Home request: aggregate per-category totals/unread.
    let categories = if category.is_empty() && offset == 0 {
        Some(
            state
                .conversation_service
                .build_categories(auth.user_id())
                .await
                .map_err(map_domain_error)?,
        )
    } else {
        None
    };

    Ok(Json(ListConversationResult {
        total,
        has_more: total as u64 >= limit,
        offset,
        items,
        removed: Vec::new(),
        last_updated_at: None,
        last_removed_at: None,
        categories,
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
    // HTTP send rate limit (Go PER_USER_LIMIT parity, 0 = off).
    if state.config.http_send_limit > 0
        && !state
            .http_limiter
            .allow(auth.user_id(), state.config.http_send_limit as u64)
    {
        return Err(ApiError::TooManyRequests);
    }
    let send = send_chat_message(&state, auth.user_id(), form);
    let (_effective_form, _topic_id, resp) =
        tokio::time::timeout(
            std::time::Duration::from_secs(state.config.request_timeout_secs.max(1)),
            send,
        )
        .await
        .map_err(|_| ApiError::RequestTimeout)??;
    Ok(Json(resp))
}

pub async fn chat_send_to_topic(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(topic_id): Path<String>,
    Json(mut form): Json<OpenApiChatMessageForm>,
) -> ApiResult<Json<OpenApiSendMessageResponse>> {
    form.topic_id = topic_id.clone();
    if state.config.http_send_limit > 0
        && !state
            .http_limiter
            .allow(auth.user_id(), state.config.http_send_limit as u64)
    {
        return Err(ApiError::TooManyRequests);
    }
    let send = send_chat_message(&state, auth.user_id(), form);
    let (_effective_form, _topic_id, resp) =
        tokio::time::timeout(
            std::time::Duration::from_secs(state.config.request_timeout_secs.max(1)),
            send,
        )
        .await
        .map_err(|_| ApiError::RequestTimeout)??;
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

    let fields = json!({
        "lastMessage": null,
        "lastMessageAt": "",
        "unread": 0,
        "startSeq": last_seq,
    });
    state
        .event_bus
        .publish(BackendEvent::ConversationUpdate(ConversationUpdateEvent {
            topic_id: topic_id.clone(),
            owner_id: auth.user_id().to_string(),
            fields: fields.clone(),
        }));
    let payload = build_conversation_update_payload(auth.user_id(), &topic_id, &fields);
    crate::api::push::broadcast_to_user(&state, auth.user_id(), &payload).await;

    Ok(Json(true))
}

pub(crate) fn message_content(form: &OpenApiChatMessageForm) -> Option<Content> {
    form.content.clone().or_else(|| {
        if form.message.is_empty() {
            None
        } else {
            Some(Content {
                content_type: if form.r#type.is_empty() {
                    "chat".to_string()
                } else {
                    form.r#type.clone()
                },
                text: form.message.clone(),
                ..Content::default()
            })
        }
    })
}

/// Peer-side view of the content: when `e2eContent` was supplied, the peer
/// sees the ciphertext instead of the plaintext `text`.
fn peer_view_content(form: &OpenApiChatMessageForm, content: Option<Content>) -> Option<Content> {
    let mut content = content?;
    if !form.e2e_content.trim().is_empty() {
        content.text = form.e2e_content.clone();
    }
    Some(content)
}

pub(crate) async fn update_topic_conversations(
    state: &AppState,
    topic_id: &str,
    resp: &OpenApiSendMessageResponse,
    message: &OpenApiChatMessageForm,
) {
    let content = message_content(message);
    let is_unreadable = content.as_ref().is_some_and(|c| {
        c.unreadable || c.content_type == "recall" || c.content_type == "conversation.update" || c.content_type == "conversation.removed"
    });

    // Dual DM: every participant owns their own topic. The sender's
    // conversation points at the self topic (plaintext), the attendee's at
    // the peer topic (e2e content when provided).
    let peer_topic = crate::services::topic::peer_pair_topic_id(topic_id);
    let attendee_id = if peer_topic.is_some() {
        let (a, b) = topic_id.split_once(':').unwrap_or(("", ""));
        if a == resp.sender_id {
            b.to_string()
        } else {
            a.to_string()
        }
    } else {
        String::new()
    };

    let now = Utc::now().to_rfc3339();

    if let Some(peer_topic) = peer_topic.as_deref() {
        // Sender row on the self topic.
        if is_unreadable {
            let _ = state
                .conversation_service
                .update_last_seq(&resp.sender_id, topic_id, resp.seq)
                .await;
        } else {
            let _ = state
                .conversation_service
                .advance_last_message(
                    &resp.sender_id,
                    topic_id,
                    crate::services::AdvanceLastMessage {
                        seq: resp.seq,
                        sender_id: resp.sender_id.clone(),
                        content: content.clone(),
                        last_message_at: now.clone(),
                        bump_unread: false,
                        draft: crate::Conversation {
                            owner_id: resp.sender_id.clone(),
                            topic_id: topic_id.to_string(),
                            attendee: attendee_id.clone(),
                            name: attendee_id.clone(),
                            multiple: false,
                            ..crate::Conversation::default()
                        },
                    },
                )
                .await;
        }

        // Attendee row on the peer topic.
        if !attendee_id.is_empty() {
            let peer_content = peer_view_content(message, content.clone());
            if is_unreadable {
                let _ = state
                    .conversation_service
                    .update_last_seq(&attendee_id, peer_topic, resp.seq)
                    .await;
            } else {
                let updated = state
                    .conversation_service
                    .advance_last_message(
                        &attendee_id,
                        peer_topic,
                        crate::services::AdvanceLastMessage {
                            seq: resp.seq,
                            sender_id: resp.sender_id.clone(),
                            content: peer_content.clone(),
                            last_message_at: now.clone(),
                            bump_unread: true,
                            draft: crate::Conversation {
                                owner_id: attendee_id.clone(),
                                topic_id: peer_topic.to_string(),
                                attendee: resp.sender_id.clone(),
                                name: resp.sender_id.clone(),
                                multiple: false,
                                ..crate::Conversation::default()
                            },
                        },
                    )
                    .await;
                let fields = match updated {
                    Ok(conv) => json!({
                        "unread": conv.unread,
                        "lastMessage": conv.last_message,
                        "lastMessageAt": conv.last_message_at,
                        "lastMessageSeq": conv.last_message_seq.unwrap_or_default(),
                        "lastSenderId": conv.last_sender_id,
                        "lastSeq": conv.last_seq,
                    }),
                    Err(_) => json!({
                        "lastMessage": peer_content,
                        "lastMessageAt": now,
                        "lastSenderId": resp.sender_id,
                        "lastSeq": resp.seq,
                    }),
                };
                let payload =
                    build_conversation_update_payload(&attendee_id, peer_topic, &fields);
                crate::api::push::broadcast_to_user(state, &attendee_id, &payload).await;
            }
        }
        return;
    }

    if let Ok(members) = state.topic_service.list_members(topic_id).await {
        let topic = state.topic_service.get_any_by_id(topic_id).await.ok();

        for user_id in &members {
            if is_unreadable {
                let _ = state
                    .conversation_service
                    .update_last_seq(user_id, topic_id, resp.seq)
                    .await;
            } else {
                let (mut attendee, mut name) = (String::new(), String::new());
                if let Some(ref t) = topic {
                    if !t.multiple {
                        if let Some(other) = members.iter().find(|&m| m != user_id) {
                            attendee = other.clone();
                            name = other.clone();
                        }
                    }
                }

                // Draft only seeds the row when it is created for the first
                // time; existing rows keep user settings untouched.
                let draft = crate::Conversation {
                    owner_id: user_id.clone(),
                    topic_id: topic_id.to_string(),
                    attendee: attendee.clone(),
                    name: name.clone(),
                    multiple: topic.as_ref().is_some_and(|t| t.multiple),
                    ..crate::Conversation::default()
                };

                let updated = state
                    .conversation_service
                    .advance_last_message(
                        user_id,
                        topic_id,
                        crate::services::AdvanceLastMessage {
                            seq: resp.seq,
                            sender_id: resp.sender_id.clone(),
                            content: content.clone(),
                            last_message_at: now.clone(),
                            bump_unread: user_id != &resp.sender_id,
                            draft,
                        },
                    )
                    .await;

                if user_id != &resp.sender_id {
                    // Authoritative post-update snapshot (unread may have been
                    // bumped by concurrent messages; never guess it client-side).
                    let fields = match updated {
                        Ok(conv) => json!({
                            "unread": conv.unread,
                            "lastMessage": conv.last_message,
                            "lastMessageAt": conv.last_message_at,
                            "lastMessageSeq": conv.last_message_seq.unwrap_or_default(),
                            "lastSenderId": conv.last_sender_id,
                            "lastSeq": conv.last_seq,
                        }),
                        Err(_) => json!({
                            "lastMessage": content,
                            "lastMessageAt": now,
                            "lastSenderId": resp.sender_id,
                            "lastSeq": resp.seq,
                        }),
                    };
                    let payload = build_conversation_update_payload(user_id, topic_id, &fields);
                    crate::api::push::broadcast_to_user(state, user_id, &payload).await;
                }
            }
        }
    }
}

/// Fan out a chat event after a successful send.
///
/// - sender devices: `{topicId: self_topic, content: original}`
/// - DM attendee devices: `{topicId: peer_topic, content: peer/e2e view}`
/// - group members (except sender): the self payload
pub(crate) async fn broadcast_chat_message(
    state: &AppState,
    sender_id: &str,
    form: &OpenApiChatMessageForm,
    resp: &OpenApiSendMessageResponse,
) {
    let created_at = form
        .created_at
        .clone()
        .unwrap_or_else(|| Utc::now().to_rfc3339());
    let content = message_content(form);
    let peer_content = peer_view_content(form, content.clone());

    let self_payload = serde_json::to_string(&serde_json::json!({
        "type": "chat",
        "topicId": resp.topic_id,
        "seq": resp.seq,
        "chatId": resp.chat_id,
        "attendee": sender_id,
        "createdAt": created_at,
        "content": content,
    }))
    .unwrap_or_default();

    let peer_topic = crate::services::topic::peer_pair_topic_id(&resp.topic_id);

    if let Some(peer_topic) = peer_topic {
        // Dual DM: push the sender-centric payload to the sender and the
        // peer-centric payload (e2e content) to the attendee.
        let (a, b) = resp.topic_id.split_once(':').unwrap_or(("", ""));
        let attendee = if a == sender_id { b } else { a };
        let peer_payload = serde_json::to_string(&serde_json::json!({
            "type": "chat",
            "topicId": peer_topic,
            "seq": resp.seq,
            "chatId": resp.chat_id,
            "attendee": sender_id,
            "createdAt": created_at,
            "content": peer_content,
        }))
        .unwrap_or_default();
        crate::api::push::broadcast_to_user(state, sender_id, &self_payload).await;
        if !attendee.is_empty() && attendee != sender_id {
            crate::api::push::broadcast_to_user(state, attendee, &peer_payload).await;
        }
    } else {
        crate::api::push::broadcast_to_user(state, sender_id, &self_payload).await;
        if let Ok(members) = state.topic_service.list_members(&resp.topic_id).await {
            for member in members {
                if member != sender_id {
                    crate::api::push::broadcast_to_user(state, &member, &self_payload).await;
                }
            }
        }
    }
}

/// Permission checks applied on the user send path only:
/// - DM: reject when the recipient has blocked the sender (Go relation.blocked)
/// - Group: reject when the sender is not a member
async fn enforce_send_permissions(
    state: &AppState,
    user_id: &str,
    form: &OpenApiChatMessageForm,
) -> Result<(), ApiError> {
    // recall / update.extra target existing messages authored by the same
    // sender; ownership is verified by the chat service, no extra checks.
    if form
        .content
        .as_ref()
        .map(|c| c.content_type == "recall" || c.content_type == "update.extra")
        .unwrap_or(false)
    {
        return Ok(());
    }

    let topic = match state.topic_service.get_any_by_id(&form.topic_id).await {
        Ok(topic) => topic,
        Err(_) => return Ok(()), // not found is handled by the send path
    };

    if topic.multiple {
        let members = state
            .topic_service
            .list_members(&form.topic_id)
            .await
            .unwrap_or_default();
        if !members.iter().any(|m| m == user_id) && topic.owner_id != user_id {
            return Err(ApiError::Forbidden);
        }
    } else {
        // DM: recipient-side block check (owner of the pair topic blocks?).
        // Both pair views resolve to the same two participants.
        let (a, b) = form.topic_id.split_once(':').unwrap_or(("", ""));
        let (sender, recipient) = if a == user_id {
            (a, b)
        } else {
            (b, a)
        };
        if !recipient.is_empty() && recipient != sender {
            let blocked = state
                .relation_service
                .is_blocked(recipient, sender)
                .await
                .unwrap_or(false);
            if blocked {
                return Err(ApiError::Forbidden);
            }
        }
    }
    Ok(())
}

pub(crate) async fn send_chat_message(
    state: &AppState,
    user_id: &str,
    form: OpenApiChatMessageForm,
) -> Result<(OpenApiChatMessageForm, String, OpenApiSendMessageResponse), ApiError> {
    let mut effective_form = form;
    if effective_form.r#type.is_empty() {
        effective_form.r#type = "chat".to_string();
    }
    if effective_form.topic_id.trim().is_empty() {
        if effective_form.attendee.trim().is_empty() {
            return Err(ApiError::bad_request("topicId or attendee is required"));
        }
        // Sender-centric dual DM topic: `{sender}:{attendee}`. The chat
        // service creates both pair topics when sending.
        effective_form.topic_id =
            crate::services::topic::pair_topic_id(user_id, effective_form.attendee.trim());
    }

    // User-path permission checks (Go CanChatWithUser / CanChatWithTopic
    // parity). OpenAPI server sends bypass this helper.
    enforce_send_permissions(state, user_id, &effective_form).await?;

    // The chat service resolves the sender-centric DM pair topics; the
    // response topic id is always the sender's own topic.
    let resp = match effective_form
        .content
        .as_ref()
        .map(|content| content.content_type.as_str())
    {
        Some("recall") => {
            state
                .chat_service
                .recall_in_topic(&effective_form.topic_id, user_id, &effective_form)
                .await
        }
        _ => {
            state
                .chat_service
                .send_to_topic(&effective_form.topic_id, user_id, &effective_form)
                .await
        }
    }
    .map_err(map_domain_error)?;
    let topic_id = resp.topic_id.clone();
    // Deliver the chat event first, then advance conversations/unread —
    // clients must observe the message before (or with) its unread bump.
    broadcast_chat_message(state, user_id, &effective_form, &resp).await;
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
