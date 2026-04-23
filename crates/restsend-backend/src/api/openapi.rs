use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::api::auth_ctx::AuthCtx;
use crate::api::error::{ApiError, ApiResult};
use crate::app::AppState;
use crate::infra::event::{
    BackendEvent, ChatEvent, ConversationRemovedEvent, ConversationUpdateEvent,
    TopicChangeOwnerEvent, TopicSilentEvent, TopicSimpleEvent, TopicUserEvent,
};
use crate::services::DomainError;
use crate::{
    ChatLogSyncForm, ListUserResult, OpenApiAuthForm, OpenApiChatMessageForm,
    OpenApiCreateTopicForm, OpenApiDocItem, OpenApiDocSchema, OpenApiImportTopicMessageForm,
    OpenApiPushForm, OpenApiRelationEditForm, OpenApiSendChatMessageForm,
    OpenApiSendChatMessageWithFormatForm, OpenApiSendMessageResponse, OpenApiSendTopicMessageForm,
    OpenApiSendTopicMessageWithFormatForm, OpenApiSilentTopicForm, OpenApiSilentTopicMembersForm,
    OpenApiUpdateConversationForm, OpenApiUpdateTopicExtraForm, OpenApiUpdateTopicForm,
    OpenApiUpdateTopicMemberForm, OpenApiUserForm, OpenApiUserListForm, Relation, UserOnlineResult,
    UserPublicProfile,
};

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiListPageForm {
    pub offset: Option<u64>,
    pub limit: Option<u64>,
    #[serde(default)]
    pub keyword: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiPagedUsers {
    pub total: u64,
    pub offset: u64,
    pub limit: u64,
    pub items: Vec<crate::User>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiPagedTopics {
    pub total: u64,
    pub offset: u64,
    pub limit: u64,
    pub items: Vec<crate::Topic>,
}

pub async fn user_online(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(user_id): Path<String>,
) -> ApiResult<Json<UserOnlineResult>> {
    auth.ensure_user_or_staff(&user_id)?;
    if user_id.trim().is_empty() {
        return Err(ApiError::bad_request("userid is required"));
    }
    let snapshot = state.presence_hub.snapshot(&user_id).await;
    Ok(Json(UserOnlineResult {
        online: snapshot.online,
        devices: snapshot.devices,
    }))
}

pub async fn user_push(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(user_id): Path<String>,
    Json(form): Json<OpenApiPushForm>,
) -> ApiResult<Json<bool>> {
    auth.ensure_user_or_staff(&user_id)?;
    let payload = if !form.message.is_empty() {
        form.message
    } else if form.payload.is_null() {
        format!("push:{}:{}", form.r#type, form.chat_id)
    } else {
        form.payload.to_string()
    };
    crate::api::push::push_local_user(&state, &user_id, &payload).await;
    Ok(Json(true))
}

pub async fn user_push_with_cid(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path((user_id, cid)): Path<(String, String)>,
    Json(form): Json<OpenApiPushForm>,
) -> ApiResult<Json<bool>> {
    auth.ensure_user_or_staff(&user_id)?;
    let payload = if !form.message.is_empty() {
        form.message
    } else {
        form.payload.to_string()
    };
    crate::api::push::push_local_device(&state, &user_id, &cid, &payload).await;
    Ok(Json(true))
}

pub async fn user_register(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(user_id): Path<String>,
    payload: Option<Json<OpenApiUserForm>>,
) -> ApiResult<Json<UserPublicProfile>> {
    auth.ensure_staff()?;
    let form = payload.map(|v| v.0).unwrap_or_default();
    let user = state
        .user_service
        .register(&user_id, form)
        .await
        .map_err(map_domain_error)?;

    let auth_token = state
        .auth_service
        .issue_token(&user_id)
        .await
        .map_err(map_domain_error)?;

    Ok(Json(UserPublicProfile { user, auth_token }))
}

pub async fn user_list(
    State(state): State<AppState>,
    auth: AuthCtx,
    payload: Option<Json<OpenApiListPageForm>>,
) -> ApiResult<Json<OpenApiPagedUsers>> {
    auth.ensure_staff()?;
    let form = payload.map(|v| v.0).unwrap_or_default();
    let offset = form.offset.unwrap_or(0);
    let limit = form.limit.unwrap_or(20).clamp(1, 100);
    let (items, total) = state
        .user_service
        .list_users(offset, limit, Some(&form.keyword))
        .await
        .map_err(map_domain_error)?;
    tracing::info!(
        admin_user_id = %auth.user_id(),
        keyword = %form.keyword,
        offset,
        limit,
        total,
        "openapi user list"
    );
    Ok(Json(OpenApiPagedUsers {
        total,
        offset,
        limit,
        items,
    }))
}

pub async fn user_set_enabled(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(user_id): Path<String>,
    Json(form): Json<serde_json::Value>,
) -> ApiResult<Json<crate::User>> {
    auth.ensure_staff()?;
    let enabled = form
        .get("enabled")
        .and_then(|v| v.as_bool())
        .ok_or_else(|| ApiError::bad_request("enabled is required"))?;
    let user = state
        .user_service
        .set_enabled(&user_id, enabled)
        .await
        .map_err(map_domain_error)?;
    let _ = state.auth_service.revoke_by_user(&user_id).await;
    tracing::info!(
        admin_user_id = %auth.user_id(),
        target_user_id = %user_id,
        enabled,
        "openapi user enabled updated"
    );
    Ok(Json(user))
}

pub async fn user_set_staff(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(user_id): Path<String>,
    Json(form): Json<serde_json::Value>,
) -> ApiResult<Json<crate::User>> {
    auth.ensure_staff()?;
    let is_staff = form
        .get("isStaff")
        .or_else(|| form.get("is_staff"))
        .and_then(|v| v.as_bool())
        .ok_or_else(|| ApiError::bad_request("isStaff is required"))?;
    let user = state
        .user_service
        .set_staff(&user_id, is_staff)
        .await
        .map_err(map_domain_error)?;
    tracing::info!(
        admin_user_id = %auth.user_id(),
        target_user_id = %user_id,
        is_staff,
        "openapi user staff updated"
    );
    Ok(Json(user))
}

pub async fn user_auth(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(user_id): Path<String>,
    payload: Option<Json<OpenApiAuthForm>>,
) -> ApiResult<Json<UserPublicProfile>> {
    auth.ensure_user_or_staff(&user_id)?;
    let form = payload.map(|v| v.0).unwrap_or_default();

    let user = state
        .user_service
        .get_or_create_for_auth(&user_id, form.create_when_not_exist)
        .await
        .map_err(map_domain_error)?;

    let auth_token = state
        .auth_service
        .issue_token(&user_id)
        .await
        .map_err(map_domain_error)?;

    Ok(Json(UserPublicProfile { user, auth_token }))
}

pub async fn user_update(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(user_id): Path<String>,
    Json(form): Json<OpenApiUserForm>,
) -> ApiResult<Json<bool>> {
    auth.ensure_user_or_staff(&user_id)?;
    let has_password = !form.password.is_empty();
    state
        .user_service
        .update(&user_id, form)
        .await
        .map_err(map_domain_error)?;
    tracing::info!(
        admin_user_id = %auth.user_id(),
        target_user_id = %user_id,
        password_updated = has_password,
        "openapi user updated"
    );
    Ok(Json(true))
}

pub async fn user_relation_update(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path((user_id, target_id)): Path<(String, String)>,
    Json(form): Json<OpenApiRelationEditForm>,
) -> ApiResult<Json<Relation>> {
    auth.ensure_user_or_staff(&user_id)?;
    if user_id == target_id {
        return Err(ApiError::bad_request(
            "userid and targetid cannot be the same",
        ));
    }
    let relation = state
        .relation_service
        .update_relation(&user_id, &target_id, form)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(relation))
}

pub async fn user_deactive(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(user_id): Path<String>,
) -> ApiResult<Json<bool>> {
    auth.ensure_user_or_staff(&user_id)?;
    state
        .user_service
        .deactive(&user_id)
        .await
        .map_err(map_domain_error)?;
    let _ = state
        .auth_service
        .revoke_by_user(&user_id)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(true))
}

pub async fn user_blacklist_get(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(user_id): Path<String>,
) -> ApiResult<Json<Vec<String>>> {
    auth.ensure_user_or_staff(&user_id)?;
    let users = state
        .relation_service
        .list_blocked(&user_id)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(users))
}

pub async fn user_blacklist_add(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(user_id): Path<String>,
    Json(form): Json<OpenApiUserListForm>,
) -> ApiResult<Json<Vec<String>>> {
    auth.ensure_user_or_staff(&user_id)?;
    if form.user_ids.is_empty() {
        return Err(ApiError::bad_request("userIds is required"));
    }
    let users = state
        .relation_service
        .update_blocked(&user_id, &form.user_ids, true)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(users))
}

pub async fn user_blacklist_remove(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(user_id): Path<String>,
    Json(form): Json<OpenApiUserListForm>,
) -> ApiResult<Json<Vec<String>>> {
    auth.ensure_user_or_staff(&user_id)?;
    if form.user_ids.is_empty() {
        return Err(ApiError::bad_request("userIds is required"));
    }
    let users = state
        .relation_service
        .update_blocked(&user_id, &form.user_ids, false)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(users))
}

pub async fn topic_create(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(topic_id): Path<String>,
    payload: Option<Json<OpenApiCreateTopicForm>>,
) -> ApiResult<Json<crate::Topic>> {
    let mut payload = payload.map(|v| v.0).unwrap_or_default();
    if payload.sender_id.is_empty() {
        payload.sender_id = auth.user_id().to_string();
    }

    let topic = state
        .topic_service
        .create_topic(Some(topic_id), payload)
        .await
        .map_err(map_domain_error)?;
    ensure_member_conversations(&state, &topic).await;
    state
        .event_bus
        .publish(BackendEvent::TopicCreate(TopicSimpleEvent {
            topic_id: topic.id.clone(),
            admin_id: topic.owner_id.clone(),
            source: topic.source.clone(),
            webhooks: topic.webhooks.clone(),
        }));
    Ok(Json(topic))
}

pub async fn topic_create_auto(
    State(state): State<AppState>,
    auth: AuthCtx,
    payload: Option<Json<OpenApiCreateTopicForm>>,
) -> ApiResult<Json<crate::Topic>> {
    let mut payload = payload.map(|v| v.0).unwrap_or_default();
    if payload.sender_id.is_empty() {
        payload.sender_id = auth.user_id().to_string();
    }

    let topic = state
        .topic_service
        .create_topic(None, payload)
        .await
        .map_err(map_domain_error)?;
    ensure_member_conversations(&state, &topic).await;
    state
        .event_bus
        .publish(BackendEvent::TopicCreate(TopicSimpleEvent {
            topic_id: topic.id.clone(),
            admin_id: topic.owner_id.clone(),
            source: topic.source.clone(),
            webhooks: topic.webhooks.clone(),
        }));
    Ok(Json(topic))
}

pub async fn topic_info(
    State(state): State<AppState>,
    _auth: AuthCtx,
    Path(topic_id): Path<String>,
) -> ApiResult<Json<crate::Topic>> {
    let topic = state
        .topic_service
        .get_by_id(&topic_id)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(topic))
}

pub async fn topic_list(
    State(state): State<AppState>,
    auth: AuthCtx,
    payload: Option<Json<OpenApiListPageForm>>,
) -> ApiResult<Json<OpenApiPagedTopics>> {
    auth.ensure_staff()?;
    let form = payload.map(|v| v.0).unwrap_or_default();
    let offset = form.offset.unwrap_or(0);
    let limit = form.limit.unwrap_or(20).clamp(1, 100);
    let (items, total) = state
        .topic_service
        .list_topics(offset, limit, Some(&form.keyword))
        .await
        .map_err(map_domain_error)?;
    tracing::info!(
        admin_user_id = %auth.user_id(),
        keyword = %form.keyword,
        offset,
        limit,
        total,
        "openapi topic list"
    );
    Ok(Json(OpenApiPagedTopics {
        total,
        offset,
        limit,
        items,
    }))
}

pub async fn topic_set_enabled(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(topic_id): Path<String>,
    Json(form): Json<serde_json::Value>,
) -> ApiResult<Json<crate::Topic>> {
    auth.ensure_staff()?;
    let enabled = form
        .get("enabled")
        .and_then(|v| v.as_bool())
        .ok_or_else(|| ApiError::bad_request("enabled is required"))?;
    let topic = state
        .topic_service
        .set_enabled(&topic_id, enabled)
        .await
        .map_err(map_domain_error)?;
    tracing::info!(
        admin_user_id = %auth.user_id(),
        topic_id = %topic_id,
        enabled,
        "openapi topic enabled updated"
    );
    Ok(Json(topic))
}

pub async fn topic_update(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(topic_id): Path<String>,
    payload: Option<Json<OpenApiUpdateTopicForm>>,
) -> ApiResult<Json<crate::Topic>> {
    auth.ensure_staff()?;
    let topic = state
        .topic_service
        .update_topic(&topic_id, payload.map(|v| v.0).unwrap_or_default())
        .await
        .map_err(map_domain_error)?;
    tracing::info!(
        admin_user_id = %auth.user_id(),
        topic_id = %topic_id,
        "openapi topic updated"
    );
    state
        .event_bus
        .publish(BackendEvent::TopicUpdate(TopicSimpleEvent {
            topic_id: topic.id.clone(),
            admin_id: auth.user_id().to_string(),
            source: topic.source.clone(),
            webhooks: topic.webhooks.clone(),
        }));
    Ok(Json(topic))
}

pub async fn topic_update_extra(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(topic_id): Path<String>,
    Json(form): Json<OpenApiUpdateTopicExtraForm>,
) -> ApiResult<Json<crate::Topic>> {
    auth.ensure_staff()?;
    let topic = state
        .topic_service
        .update_topic_extra(&topic_id, form)
        .await
        .map_err(map_domain_error)?;
    state
        .event_bus
        .publish(BackendEvent::TopicUpdate(TopicSimpleEvent {
            topic_id: topic.id.clone(),
            admin_id: auth.user_id().to_string(),
            source: topic.source.clone(),
            webhooks: topic.webhooks.clone(),
        }));
    Ok(Json(topic))
}

pub async fn topic_logs(
    State(state): State<AppState>,
    _auth: AuthCtx,
    Path(topic_id): Path<String>,
    payload: Option<Json<ChatLogSyncForm>>,
) -> ApiResult<Json<crate::ChatLogSyncResult>> {
    let result = state
        .chat_service
        .topic_logs(&topic_id, &payload.map(|v| v.0).unwrap_or_default())
        .await
        .map_err(map_domain_error)?;
    Ok(Json(result))
}

pub async fn topic_import_message(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(topic_id): Path<String>,
    Json(form): Json<OpenApiImportTopicMessageForm>,
) -> ApiResult<Json<crate::OpenApiImportTopicMessageResponse>> {
    auth.ensure_staff()?;
    let result = state
        .chat_service
        .import_topic_logs(&topic_id, form)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(result))
}

pub async fn topic_send_message(
    State(state): State<AppState>,
    _auth: AuthCtx,
    Path(topic_id): Path<String>,
    Json(form): Json<OpenApiSendTopicMessageForm>,
) -> ApiResult<Json<OpenApiSendMessageResponse>> {
    if form.ensure {
        ensure_topic_exists_for_send(&state, &topic_id, &form).await?;
    }

    let sender_id = if form.sender_id.is_empty() {
        return Err(ApiError::bad_request("senderId is required"));
    } else {
        form.sender_id.clone()
    };

    let resp = state
        .chat_service
        .send_to_topic(&topic_id, &sender_id, &form.message)
        .await
        .map_err(map_domain_error)?;

    fanout_topic_message(&state, &topic_id, &resp, &form.message).await;
    state.event_bus.publish(BackendEvent::Chat(ChatEvent {
        topic_id: topic_id.clone(),
        sender_id,
        chat_id: resp.chat_id.clone(),
        seq: resp.seq,
        created_at: form.message.created_at.clone().unwrap_or_else(now),
        content: form.message.content.clone(),
    }));
    Ok(Json(resp))
}

pub async fn topic_send_message_with_format(
    State(state): State<AppState>,
    _auth: AuthCtx,
    Path((topic_id, format)): Path<(String, String)>,
    Json(form): Json<OpenApiSendTopicMessageWithFormatForm>,
) -> ApiResult<Json<serde_json::Value>> {
    if form.ensure {
        ensure_topic_exists_for_converter_send(&state, &topic_id, &form).await?;
    }

    let Some(message) = convert_message(&format, form.message)? else {
        return Ok(Json(json!({})));
    };

    let resp = state
        .chat_service
        .send_to_topic(&topic_id, &form.sender_id, &message)
        .await
        .map_err(map_domain_error)?;

    fanout_topic_message(&state, &topic_id, &resp, &message).await;
    state.event_bus.publish(BackendEvent::Chat(ChatEvent {
        topic_id: topic_id.clone(),
        sender_id: form.sender_id.clone(),
        chat_id: resp.chat_id.clone(),
        seq: resp.seq,
        created_at: message.created_at.clone().unwrap_or_else(now),
        content: message.content.clone(),
    }));
    Ok(Json(
        serde_json::to_value(resp).unwrap_or_else(|_| json!({})),
    ))
}

pub async fn chat_send_message(
    State(state): State<AppState>,
    _auth: AuthCtx,
    Path(sender_id): Path<String>,
    Json(form): Json<OpenApiSendChatMessageForm>,
) -> ApiResult<Json<Vec<OpenApiSendMessageResponse>>> {
    if form.user_ids.is_empty() {
        return Err(ApiError::bad_request("userIds is required"));
    }

    let mut responses = Vec::with_capacity(form.user_ids.len());
    for attendee_id in form.user_ids {
        let result = state
            .chat_service
            .send_to_user(&sender_id, &attendee_id, &form.message)
            .await;
        match result {
            Ok(resp) => {
                let payload = serde_json::to_string(&resp).unwrap_or_default();
                crate::api::push::broadcast_to_user(&state, &attendee_id, &payload).await;
                crate::api::push::broadcast_to_user(&state, &sender_id, &payload).await;
                responses.push(resp);
            }
            Err(err) => responses.push(OpenApiSendMessageResponse {
                sender_id: sender_id.clone(),
                attendee_id,
                chat_id: form.message.chat_id.clone(),
                code: 500,
                message: err.to_string(),
                ..OpenApiSendMessageResponse::default()
            }),
        }
    }

    Ok(Json(responses))
}

pub async fn chat_send_message_with_format(
    State(state): State<AppState>,
    _auth: AuthCtx,
    Path((sender_id, format)): Path<(String, String)>,
    Json(form): Json<OpenApiSendChatMessageWithFormatForm>,
) -> ApiResult<Json<serde_json::Value>> {
    let Some(message) = convert_message(&format, form.message)? else {
        return Ok(Json(json!([])));
    };

    let mut responses = Vec::with_capacity(form.user_ids.len());
    for attendee_id in form.user_ids {
        let result = state
            .chat_service
            .send_to_user(&sender_id, &attendee_id, &message)
            .await;
        match result {
            Ok(resp) => {
                let payload = serde_json::to_string(&resp).unwrap_or_default();
                crate::api::push::broadcast_to_user(&state, &attendee_id, &payload).await;
                crate::api::push::broadcast_to_user(&state, &sender_id, &payload).await;
                responses.push(resp);
            }
            Err(err) => responses.push(OpenApiSendMessageResponse {
                sender_id: sender_id.clone(),
                attendee_id,
                chat_id: message.chat_id.clone(),
                code: 500,
                message: err.to_string(),
                ..OpenApiSendMessageResponse::default()
            }),
        }
    }

    Ok(Json(
        serde_json::to_value(responses).unwrap_or_else(|_| json!([])),
    ))
}

pub async fn topic_members(
    State(state): State<AppState>,
    _auth: AuthCtx,
    Path(topic_id): Path<String>,
) -> ApiResult<Json<ListUserResult>> {
    let result = state
        .topic_service
        .list_members_detailed(&topic_id, None, Some(100))
        .await
        .map_err(map_domain_error)?;
    Ok(Json(result))
}

pub async fn topic_join(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(topic_id): Path<String>,
    Json(form): Json<OpenApiUserListForm>,
) -> ApiResult<Json<Vec<String>>> {
    auth.ensure_staff()?;
    if form.user_ids.is_empty() {
        return Err(ApiError::bad_request("userIds is required"));
    }
    let members = state
        .topic_service
        .join_members(&topic_id, form.user_ids, form.source)
        .await
        .map_err(map_domain_error)?;
    for user_id in &members {
        state
            .event_bus
            .publish(BackendEvent::TopicJoin(TopicUserEvent {
                topic_id: topic_id.clone(),
                admin_id: auth.user_id().to_string(),
                user_id: user_id.clone(),
                source: "openapi".to_string(),
            }));
    }
    Ok(Json(members))
}

pub async fn topic_quit(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(topic_id): Path<String>,
    Json(form): Json<OpenApiUserListForm>,
) -> ApiResult<Json<Vec<String>>> {
    auth.ensure_staff()?;
    if form.user_ids.is_empty() {
        return Err(ApiError::bad_request("userIds is required"));
    }
    let members = state
        .topic_service
        .quit_members(&topic_id, form.user_ids)
        .await
        .map_err(map_domain_error)?;
    for user_id in &members {
        state
            .event_bus
            .publish(BackendEvent::TopicQuit(TopicUserEvent {
                topic_id: topic_id.clone(),
                admin_id: auth.user_id().to_string(),
                user_id: user_id.clone(),
                source: "openapi".to_string(),
            }));
    }
    Ok(Json(members))
}

pub async fn topic_dismiss(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(topic_id): Path<String>,
) -> ApiResult<Json<bool>> {
    auth.ensure_staff()?;
    let topic = state
        .topic_service
        .get_by_id(&topic_id)
        .await
        .map_err(map_domain_error)?;
    state
        .topic_service
        .dismiss_topic(&topic_id)
        .await
        .map_err(map_domain_error)?;
    state
        .event_bus
        .publish(BackendEvent::TopicDismiss(TopicSimpleEvent {
            topic_id,
            admin_id: auth.user_id().to_string(),
            source: "openapi".to_string(),
            webhooks: topic.webhooks.clone(),
        }));
    Ok(Json(true))
}

pub async fn topic_update_member(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path((topic_id, user_id)): Path<(String, String)>,
    payload: Option<Json<OpenApiUpdateTopicMemberForm>>,
) -> ApiResult<Json<crate::TopicMember>> {
    auth.ensure_staff()?;
    let member = state
        .topic_service
        .update_member(
            &topic_id,
            &user_id,
            payload.map(|v| v.0).unwrap_or_default(),
        )
        .await
        .map_err(map_domain_error)?;
    tracing::info!(
        admin_user_id = %auth.user_id(),
        topic_id = %topic_id,
        target_user_id = %user_id,
        "openapi topic member updated"
    );
    Ok(Json(member))
}

pub async fn topic_member_info(
    State(state): State<AppState>,
    _auth: AuthCtx,
    Path((topic_id, user_id)): Path<(String, String)>,
) -> ApiResult<Json<crate::TopicMember>> {
    let member = state
        .topic_service
        .get_member(&topic_id, &user_id)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(member))
}

pub async fn topic_kickout_member(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path((topic_id, user_id)): Path<(String, String)>,
) -> ApiResult<Json<bool>> {
    auth.ensure_staff()?;
    let _ = state
        .topic_service
        .quit_members(&topic_id, vec![user_id.clone()])
        .await
        .map_err(map_domain_error)?;
    tracing::info!(
        admin_user_id = %auth.user_id(),
        topic_id = %topic_id,
        target_user_id = %user_id,
        "openapi topic member kicked"
    );
    state
        .event_bus
        .publish(BackendEvent::TopicKickout(TopicUserEvent {
            topic_id,
            admin_id: auth.user_id().to_string(),
            user_id,
            source: "openapi".to_string(),
        }));
    Ok(Json(true))
}

pub async fn topic_transfer_owner(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path((topic_id, user_id)): Path<(String, String)>,
) -> ApiResult<Json<bool>> {
    auth.ensure_staff()?;
    state
        .topic_service
        .transfer_owner(&topic_id, &user_id)
        .await
        .map_err(map_domain_error)?;
    tracing::info!(
        admin_user_id = %auth.user_id(),
        topic_id = %topic_id,
        target_user_id = %user_id,
        "openapi topic owner transferred"
    );
    state
        .event_bus
        .publish(BackendEvent::TopicChangeOwner(TopicChangeOwnerEvent {
            topic_id,
            admin_id: auth.user_id().to_string(),
            user_id,
            source: "openapi".to_string(),
        }));
    Ok(Json(true))
}

pub async fn topic_add_admin(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path((topic_id, user_id)): Path<(String, String)>,
) -> ApiResult<Json<bool>> {
    auth.ensure_staff()?;
    state
        .topic_service
        .add_admin(&topic_id, &user_id)
        .await
        .map_err(map_domain_error)?;
    tracing::info!(
        admin_user_id = %auth.user_id(),
        topic_id = %topic_id,
        target_user_id = %user_id,
        "openapi topic admin added"
    );
    Ok(Json(true))
}

pub async fn topic_remove_admin(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path((topic_id, user_id)): Path<(String, String)>,
) -> ApiResult<Json<bool>> {
    auth.ensure_staff()?;
    state
        .topic_service
        .remove_admin(&topic_id, &user_id)
        .await
        .map_err(map_domain_error)?;
    tracing::info!(
        admin_user_id = %auth.user_id(),
        topic_id = %topic_id,
        target_user_id = %user_id,
        "openapi topic admin removed"
    );
    Ok(Json(true))
}

pub async fn topic_silent_member(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(topic_id): Path<String>,
    Json(form): Json<OpenApiSilentTopicMembersForm>,
) -> ApiResult<Json<Vec<String>>> {
    auth.ensure_staff()?;
    if form.user_ids.is_empty() {
        return Err(ApiError::bad_request("userIds is required"));
    }
    let users = state
        .topic_service
        .silent_member(&topic_id, form)
        .await
        .map_err(map_domain_error)?;
    tracing::info!(
        admin_user_id = %auth.user_id(),
        topic_id = %topic_id,
        count = users.len(),
        "openapi topic members silenced"
    );
    for user_id in &users {
        state
            .event_bus
            .publish(BackendEvent::TopicSilentMember(TopicSilentEvent {
                topic_id: topic_id.clone(),
                admin_id: auth.user_id().to_string(),
                user_id: user_id.clone(),
                duration: String::new(),
                source: "openapi".to_string(),
            }));
    }
    Ok(Json(users))
}

pub async fn topic_add_silent_whitelist(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(topic_id): Path<String>,
    Json(form): Json<OpenApiUserListForm>,
) -> ApiResult<Json<Vec<String>>> {
    auth.ensure_staff()?;
    if form.user_ids.is_empty() {
        return Err(ApiError::bad_request("userIds is required"));
    }
    let users = state
        .topic_service
        .add_silent_whitelist(&topic_id, form.user_ids)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(users))
}

pub async fn topic_remove_silent_whitelist(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(topic_id): Path<String>,
    Json(form): Json<OpenApiUserListForm>,
) -> ApiResult<Json<Vec<String>>> {
    auth.ensure_staff()?;
    if form.user_ids.is_empty() {
        return Err(ApiError::bad_request("userIds is required"));
    }
    let users = state
        .topic_service
        .remove_silent_whitelist(&topic_id, form.user_ids)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(users))
}

pub async fn topic_silent(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(topic_id): Path<String>,
    payload: Option<Json<OpenApiSilentTopicForm>>,
) -> ApiResult<Json<bool>> {
    auth.ensure_staff()?;
    state
        .topic_service
        .silent_topic(&topic_id, payload.map(|v| v.0).unwrap_or_default())
        .await
        .map_err(map_domain_error)?;
    tracing::info!(
        admin_user_id = %auth.user_id(),
        topic_id = %topic_id,
        "openapi topic silent updated"
    );
    state
        .event_bus
        .publish(BackendEvent::TopicSilent(TopicSilentEvent {
            topic_id,
            admin_id: auth.user_id().to_string(),
            user_id: String::new(),
            duration: String::new(),
            source: "openapi".to_string(),
        }));
    Ok(Json(true))
}

pub async fn conversation_info(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path((user_id, topic_id)): Path<(String, String)>,
) -> ApiResult<Json<crate::Conversation>> {
    auth.ensure_user_or_staff(&user_id)?;
    let conversation = state
        .conversation_service
        .get_conversation(&user_id, &topic_id)
        .await
        .map_err(map_domain_error)?;
    Ok(Json(conversation))
}

pub async fn conversation_update(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path((user_id, topic_id)): Path<(String, String)>,
    Json(form): Json<OpenApiUpdateConversationForm>,
) -> ApiResult<Json<crate::Conversation>> {
    auth.ensure_user_or_staff(&user_id)?;
    let fields = crate::api::chat::conversation_update_fields(&form);
    let conversation = state
        .conversation_service
        .update_conversation(&user_id, &topic_id, form)
        .await
        .map_err(map_domain_error)?;
    if !fields.as_object().is_some_and(|v| v.is_empty()) {
        state
            .event_bus
            .publish(BackendEvent::ConversationUpdate(ConversationUpdateEvent {
                topic_id: topic_id.clone(),
                owner_id: user_id.clone(),
                fields: fields.clone(),
            }));
        let payload =
            crate::api::chat::build_conversation_update_payload(&user_id, &topic_id, &fields);
        crate::api::push::broadcast_to_user(&state, &user_id, &payload).await;
    }
    Ok(Json(conversation))
}

pub async fn conversation_remove(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path((user_id, topic_id)): Path<(String, String)>,
) -> ApiResult<Json<bool>> {
    auth.ensure_user_or_staff(&user_id)?;
    state
        .conversation_service
        .remove_conversation(&user_id, &topic_id)
        .await
        .map_err(map_domain_error)?;
    state.event_bus.publish(BackendEvent::ConversationRemoved(
        ConversationRemovedEvent {
            topic_id: topic_id.clone(),
            owner_id: user_id.clone(),
            source: "openapi".to_string(),
        },
    ));
    let payload = crate::api::chat::build_conversation_removed_payload(&user_id, &topic_id);
    crate::api::push::broadcast_to_user(&state, &user_id, &payload).await;
    Ok(Json(true))
}

pub async fn conversation_mark_unread(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path((user_id, topic_id)): Path<(String, String)>,
) -> ApiResult<Json<bool>> {
    auth.ensure_user_or_staff(&user_id)?;
    state
        .conversation_service
        .mark_unread(&user_id, &topic_id)
        .await
        .map_err(map_domain_error)?;
    state
        .event_bus
        .publish(BackendEvent::ConversationUpdate(ConversationUpdateEvent {
            topic_id: topic_id.clone(),
            owner_id: user_id.clone(),
            fields: serde_json::json!({"markUnread": true}),
        }));
    let payload = crate::api::chat::build_conversation_update_payload(
        &user_id,
        &topic_id,
        &serde_json::json!({"markUnread": true}),
    );
    crate::api::push::broadcast_to_user(&state, &user_id, &payload).await;
    Ok(Json(true))
}

pub async fn docs() -> Json<Vec<OpenApiDocItem>> {
    Json(vec![
        doc(
            "OpenAPI - User",
            "POST",
            "/open/user/online/:userid",
            "Get user online status",
            false,
            None,
            OpenApiDocSchema::UserOnlineResult,
        ),
        doc(
            "OpenAPI - User",
            "POST",
            "/open/user/push/:userid",
            "Push message to user all devices",
            false,
            Some(OpenApiDocSchema::String),
            OpenApiDocSchema::Bool,
        ),
        doc(
            "OpenAPI - User",
            "POST",
            "/open/user/push/:userid/:cid",
            "Push message to user with cid",
            false,
            Some(OpenApiDocSchema::String),
            OpenApiDocSchema::Bool,
        ),
        doc(
            "OpenAPI - User",
            "POST",
            "/open/user/register/:userid",
            "Register new user",
            false,
            Some(OpenApiDocSchema::User),
            OpenApiDocSchema::User,
        ),
        doc(
            "OpenAPI - User",
            "POST",
            "/open/user/list",
            "List users with paging",
            false,
            Some(OpenApiDocSchema::String),
            OpenApiDocSchema::String,
        ),
        doc(
            "OpenAPI - User",
            "POST",
            "/open/user/update/:userid",
            "Update user profile",
            false,
            Some(OpenApiDocSchema::User),
            OpenApiDocSchema::Bool,
        ),
        doc(
            "OpenAPI - User",
            "POST",
            "/open/user/enabled/:userid",
            "Enable or disable user",
            false,
            Some(OpenApiDocSchema::Bool),
            OpenApiDocSchema::User,
        ),
        doc(
            "OpenAPI - User",
            "POST",
            "/open/user/staff/:userid",
            "Grant or revoke staff",
            false,
            Some(OpenApiDocSchema::Bool),
            OpenApiDocSchema::User,
        ),
        doc(
            "OpenAPI - User",
            "POST",
            "/open/user/relation/:userid/:targetid",
            "Update user relation",
            false,
            Some(OpenApiDocSchema::Relation),
            OpenApiDocSchema::Relation,
        ),
        doc(
            "OpenAPI - User",
            "POST",
            "/open/user/auth/:userid",
            "Login with user",
            false,
            Some(OpenApiDocSchema::String),
            OpenApiDocSchema::User,
        ),
        doc(
            "OpenAPI - User",
            "POST",
            "/open/user/blacklist/get/:userid",
            "Get user blacklist",
            false,
            None,
            OpenApiDocSchema::StringArray,
        ),
        doc(
            "OpenAPI - User",
            "POST",
            "/open/user/blacklist/add/:userid",
            "Add users to blacklist",
            false,
            Some(OpenApiDocSchema::StringArray),
            OpenApiDocSchema::StringArray,
        ),
        doc(
            "OpenAPI - User",
            "POST",
            "/open/user/blacklist/remove/:userid",
            "Remove users from blacklist",
            false,
            Some(OpenApiDocSchema::StringArray),
            OpenApiDocSchema::StringArray,
        ),
        doc(
            "OpenAPI - Topic",
            "POST",
            "/open/topic/create",
            "Create topic with users",
            false,
            Some(OpenApiDocSchema::Topic),
            OpenApiDocSchema::Topic,
        ),
        doc(
            "OpenAPI - Topic",
            "POST",
            "/open/topic/create/:topicid",
            "Create topic with users",
            false,
            Some(OpenApiDocSchema::Topic),
            OpenApiDocSchema::Topic,
        ),
        doc(
            "OpenAPI - Topic",
            "POST",
            "/open/topic/list",
            "List topics with paging",
            false,
            Some(OpenApiDocSchema::String),
            OpenApiDocSchema::String,
        ),
        doc(
            "OpenAPI - Topic",
            "POST",
            "/open/topic/info/:topicid",
            "Get topic detail",
            false,
            None,
            OpenApiDocSchema::Topic,
        ),
        doc(
            "OpenAPI - Topic",
            "POST",
            "/open/topic/update/:topicid",
            "Update topic meta",
            false,
            Some(OpenApiDocSchema::Topic),
            OpenApiDocSchema::Topic,
        ),
        doc(
            "OpenAPI - Topic",
            "POST",
            "/open/topic/enabled/:topicid",
            "Enable or disable topic",
            false,
            Some(OpenApiDocSchema::Bool),
            OpenApiDocSchema::Topic,
        ),
        doc(
            "OpenAPI - Topic",
            "POST",
            "/open/topic/update_extra/:topicid",
            "Update topic extra",
            false,
            Some(OpenApiDocSchema::String),
            OpenApiDocSchema::Topic,
        ),
        doc(
            "OpenAPI - Topic",
            "POST",
            "/open/topic/logs/:topicid",
            "Get topic chat logs",
            false,
            Some(OpenApiDocSchema::String),
            OpenApiDocSchema::ChatLogSyncResult,
        ),
        doc(
            "OpenAPI - Topic",
            "POST",
            "/open/topic/import/:topicid",
            "Import messages to topic",
            false,
            Some(OpenApiDocSchema::String),
            OpenApiDocSchema::StringArray,
        ),
        doc(
            "OpenAPI - Topic",
            "POST",
            "/open/topic/send/:topicid",
            "Send message to topic",
            false,
            Some(OpenApiDocSchema::String),
            OpenApiDocSchema::OpenApiSendMessageResponse,
        ),
        doc(
            "OpenAPI - Topic",
            "POST",
            "/open/topic/send/:topicid/:format",
            "Send message with converter",
            false,
            Some(OpenApiDocSchema::String),
            OpenApiDocSchema::OpenApiSendMessageResponse,
        ),
        doc(
            "OpenAPI - Conversation",
            "POST",
            "/open/chat/:senderid",
            "Send chat message to users",
            false,
            Some(OpenApiDocSchema::String),
            OpenApiDocSchema::OpenApiSendMessageResponse,
        ),
        doc(
            "OpenAPI - Conversation",
            "POST",
            "/open/chat/:senderid/:format",
            "Send chat with converter",
            false,
            Some(OpenApiDocSchema::String),
            OpenApiDocSchema::OpenApiSendMessageResponse,
        ),
        doc(
            "OpenAPI - Conversation",
            "POST",
            "/open/conversation/update/:userid/:topicid",
            "Update conversation",
            false,
            Some(OpenApiDocSchema::Conversation),
            OpenApiDocSchema::Conversation,
        ),
        doc(
            "OpenAPI - Conversation",
            "POST",
            "/open/conversation/info/:userid/:topicid",
            "Get conversation",
            false,
            None,
            OpenApiDocSchema::Conversation,
        ),
        doc(
            "OpenAPI - Topic",
            "POST",
            "/open/topic/members/:topicid",
            "Get topic members",
            false,
            None,
            OpenApiDocSchema::StringArray,
        ),
        doc(
            "OpenAPI - Topic",
            "POST",
            "/open/topic/join/:topicid",
            "Join topic with users",
            false,
            Some(OpenApiDocSchema::StringArray),
            OpenApiDocSchema::StringArray,
        ),
        doc(
            "OpenAPI - Topic",
            "POST",
            "/open/topic/quit/:topicid",
            "Quit topic with users",
            false,
            Some(OpenApiDocSchema::StringArray),
            OpenApiDocSchema::StringArray,
        ),
        doc(
            "OpenAPI - Topic",
            "POST",
            "/open/topic/dismiss/:topicid",
            "Dismiss a topic",
            false,
            None,
            OpenApiDocSchema::Bool,
        ),
        doc(
            "OpenAPI - Topic",
            "POST",
            "/open/topic/member/:topicid/:userid",
            "Update topic member",
            false,
            Some(OpenApiDocSchema::TopicMember),
            OpenApiDocSchema::TopicMember,
        ),
        doc(
            "OpenAPI - Topic",
            "POST",
            "/open/topic/member_info/:topicid/:userid",
            "Get topic member",
            false,
            None,
            OpenApiDocSchema::TopicMember,
        ),
        doc(
            "OpenAPI - Topic",
            "POST",
            "/open/topic/transfer/:topicid/:userid",
            "Transfer topic owner",
            false,
            None,
            OpenApiDocSchema::Bool,
        ),
        doc(
            "OpenAPI - Topic",
            "POST",
            "/open/topic/kickout/:topicid/:userid",
            "Kickout topic member",
            false,
            None,
            OpenApiDocSchema::Bool,
        ),
        doc(
            "OpenAPI - Topic",
            "POST",
            "/open/topic/admin/add/:topicid/:userid",
            "Add admin",
            false,
            None,
            OpenApiDocSchema::Bool,
        ),
        doc(
            "OpenAPI - Topic",
            "POST",
            "/open/topic/admin/remove/:topicid/:userid",
            "Remove admin",
            false,
            None,
            OpenApiDocSchema::Bool,
        ),
        doc(
            "OpenAPI - Topic",
            "POST",
            "/open/topic/silent/member/:topicid",
            "Silent members",
            false,
            Some(OpenApiDocSchema::String),
            OpenApiDocSchema::StringArray,
        ),
        doc(
            "OpenAPI - Topic",
            "POST",
            "/open/topic/silent/whitelist/add/:topicid",
            "Add silent whitelist",
            false,
            Some(OpenApiDocSchema::StringArray),
            OpenApiDocSchema::StringArray,
        ),
        doc(
            "OpenAPI - Topic",
            "POST",
            "/open/topic/silent/whitelist/remove/:topicid",
            "Remove silent whitelist",
            false,
            Some(OpenApiDocSchema::StringArray),
            OpenApiDocSchema::StringArray,
        ),
        doc(
            "OpenAPI - Topic",
            "POST",
            "/open/topic/silent/topic/:topicid",
            "Silent topic",
            false,
            Some(OpenApiDocSchema::String),
            OpenApiDocSchema::Bool,
        ),
        doc(
            "OpenAPI - Attachment",
            "POST",
            "/api/attachment/upload",
            "Upload attachment",
            false,
            Some(OpenApiDocSchema::String),
            OpenApiDocSchema::String,
        ),
    ])
}

async fn fanout_topic_message(
    state: &AppState,
    topic_id: &str,
    resp: &OpenApiSendMessageResponse,
    message: &OpenApiChatMessageForm,
) {
    let payload = serde_json::to_string(resp).unwrap_or_default();
    let _ = state
        .conversation_service
        .create_or_update(crate::Conversation {
            owner_id: resp.sender_id.clone(),
            topic_id: topic_id.to_string(),
            unread: 0,
            last_seq: resp.seq,
            updated_at: now(),
            ..crate::Conversation::default()
        })
        .await;

    if let Ok(members) = state.topic_service.list_members(topic_id).await {
        for user_id in members {
            let unread = if user_id == resp.sender_id { 0 } else { 1 };
            let _ = state
                .conversation_service
                .create_or_update(crate::Conversation {
                    owner_id: user_id.clone(),
                    topic_id: topic_id.to_string(),
                    unread,
                    last_seq: resp.seq,
                    last_sender_id: resp.sender_id.clone(),
                    last_message: message.content.clone().or_else(|| {
                        if message.message.is_empty() {
                            None
                        } else {
                            Some(crate::Content {
                                content_type: if message.r#type.is_empty() {
                                    "chat".to_string()
                                } else {
                                    message.r#type.clone()
                                },
                                text: message.message.clone(),
                                ..crate::Content::default()
                            })
                        }
                    }),
                    updated_at: now(),
                    ..crate::Conversation::default()
                })
                .await;
            crate::api::push::broadcast_to_user(state, &user_id, &payload).await;
        }
    } else {
        crate::api::push::broadcast_to_user(state, &resp.sender_id, &payload).await;
    }
}

async fn ensure_topic_exists_for_send(
    state: &AppState,
    topic_id: &str,
    form: &OpenApiSendTopicMessageForm,
) -> ApiResult<()> {
    if state.topic_service.get_by_id(topic_id).await.is_ok() {
        return Ok(());
    }

    state
        .topic_service
        .create_topic(
            Some(topic_id.to_string()),
            OpenApiCreateTopicForm {
                sender_id: form.sender_id.clone(),
                members: form.members.clone(),
                name: form.name.clone(),
                icon: form.icon.clone(),
                multiple: Some(true),
                ..OpenApiCreateTopicForm::default()
            },
        )
        .await
        .map_err(map_domain_error)?;
    Ok(())
}

async fn ensure_topic_exists_for_converter_send(
    state: &AppState,
    topic_id: &str,
    form: &OpenApiSendTopicMessageWithFormatForm,
) -> ApiResult<()> {
    if state.topic_service.get_by_id(topic_id).await.is_ok() {
        return Ok(());
    }

    state
        .topic_service
        .create_topic(
            Some(topic_id.to_string()),
            OpenApiCreateTopicForm {
                sender_id: form.sender_id.clone(),
                members: form.members.clone(),
                name: form.name.clone(),
                icon: form.icon.clone(),
                source: form.source.clone(),
                multiple: Some(true),
                ..OpenApiCreateTopicForm::default()
            },
        )
        .await
        .map_err(map_domain_error)?;
    Ok(())
}

async fn ensure_member_conversations(state: &AppState, topic: &crate::Topic) {
    let Ok(members) = state.topic_service.list_members(&topic.id).await else {
        return;
    };

    for user_id in members {
        let _ = state
            .conversation_service
            .create_or_update(crate::Conversation {
                owner_id: user_id,
                topic_id: topic.id.clone(),
                multiple: topic.multiple,
                attendee: topic.attendee_id.clone(),
                members: topic.members as i64,
                name: topic.name.clone(),
                icon: topic.icon.clone(),
                kind: topic.kind.clone(),
                source: topic.source.clone(),
                updated_at: now(),
                ..crate::Conversation::default()
            })
            .await;
    }
}

fn doc(
    group: &str,
    method: &str,
    path: &str,
    desc: &str,
    auth_required: bool,
    request: Option<OpenApiDocSchema>,
    response: OpenApiDocSchema,
) -> OpenApiDocItem {
    OpenApiDocItem {
        group: group.to_string(),
        method: method.to_string(),
        path: path.to_string(),
        desc: desc.to_string(),
        auth_required,
        request,
        response,
    }
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

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn convert_message(
    format: &str,
    message: serde_json::Value,
) -> ApiResult<Option<OpenApiChatMessageForm>> {
    if format.eq_ignore_ascii_case("rongcloud") {
        return decode_rongcloud_message(message).map(Some);
    }
    Ok(None)
}

fn decode_rongcloud_message(message: serde_json::Value) -> ApiResult<OpenApiChatMessageForm> {
    let object_name = message
        .get("objectName")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::bad_request("invalid rongcloud message, objectName is empty"))?;

    let content_str = message
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or_default();

    let mut content = crate::Content::default();
    let req_type: String;

    match object_name {
        "RCD:GroupNoticeMessage" => {
            let msg: RongCloudGroupNoticeMessage = serde_json::from_str(content_str)
                .map_err(|e| ApiError::bad_request(e.to_string()))?;
            req_type = "chat".to_string();
            content.text = msg.notice;
            content.mentions = vec![msg.user_id];
            content.content_type = match msg.r#type {
                0 => "topic.silent.member",
                1 => "topic.notice",
                2 | 3 => "topic.silent",
                _ => "text",
            }
            .to_string();
            if msg.r#type == 2 {
                content.duration = "forever".to_string();
            }
        }
        "RC:GrpNtf" => {
            let msg: RongCloudGroupNotify = serde_json::from_str(content_str)
                .map_err(|e| ApiError::bad_request(e.to_string()))?;
            req_type = "chat".to_string();
            content.text = msg.message;
            content.mentions = vec![msg.operator_user_id];
            content.content_type = match msg.operation.as_str() {
                "Add" => "topic.join",
                "Create" => "topic.create",
                _ => "text",
            }
            .to_string();
        }
        "RC:TxtMsg" => {
            let msg: RongCloudTextMessage = serde_json::from_str(content_str)
                .map_err(|e| ApiError::bad_request(e.to_string()))?;
            req_type = "chat".to_string();
            content.content_type = "text".to_string();
            content.text = msg.content;
            if !msg.user.id.is_empty() {
                content.mentions = vec![msg.user.id];
            }
        }
        _ => {
            if let Some(v) = object_name.strip_prefix("RCD:") {
                req_type = "chat".to_string();
                content.content_type = v.to_ascii_lowercase();
            } else {
                req_type = "system".to_string();
                content.content_type = object_name.to_string();
            }
            content.text = content_str.to_string();
        }
    }

    Ok(OpenApiChatMessageForm {
        r#type: req_type,
        chat_id: format!("chat-{}", uuid::Uuid::new_v4().simple()),
        content: Some(content),
        source: "openapi-rongcloud".to_string(),
        ..OpenApiChatMessageForm::default()
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RongCloudGroupNoticeMessage {
    user_id: String,
    notice: String,
    #[serde(default)]
    r#type: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RongCloudGroupNotify {
    operator_user_id: String,
    operation: String,
    message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RongCloudTextMessage {
    content: String,
    #[serde(default)]
    user: RongCloudMsgUser,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct RongCloudMsgUser {
    #[serde(default)]
    id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct _Unused;
