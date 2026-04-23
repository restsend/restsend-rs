use axum::extract::{Path, State};
use axum::Json;
use std::time::Instant;

use crate::api::auth_ctx::AuthCtx;
use crate::api::error::{ApiError, ApiResult};
use crate::app::AppState;
use crate::infra::event::{
    BackendEvent, TopicChangeOwnerEvent, TopicKnockEvent, TopicNoticeEvent, TopicSilentEvent,
    TopicSimpleEvent, TopicUserEvent,
};
use crate::services::DomainError;

pub async fn topic_info(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(topic_id): Path<String>,
) -> ApiResult<Json<crate::Topic>> {
    let st = Instant::now();
    let topic = state
        .topic_service
        .get_by_id(&topic_id)
        .await
        .map_err(map_domain_error)?;
    tracing::info!(
        user_id = %auth.user_id(),
        topic_id = %topic_id,
        elapsed_ms = st.elapsed().as_millis() as u64,
        "topic info fetched"
    );
    Ok(Json(topic))
}

pub async fn topic_create(
    State(state): State<AppState>,
    auth: AuthCtx,
    Json(form): Json<crate::OpenApiCreateTopicForm>,
) -> ApiResult<Json<crate::Topic>> {
    let st = Instant::now();
    let mut form = form;
    if form.sender_id.is_empty() {
        form.sender_id = auth.user_id().to_string();
    }
    let topic = state
        .topic_service
        .create_topic(None, form)
        .await
        .map_err(map_domain_error)?;
    state
        .event_bus
        .publish(BackendEvent::TopicCreate(TopicSimpleEvent {
            topic_id: topic.id.clone(),
            admin_id: topic.owner_id.clone(),
            source: topic.source.clone(),
            webhooks: topic.webhooks.clone(),
        }));
    tracing::info!(
        user_id = %auth.user_id(),
        topic_id = %topic.id,
        multiple = topic.multiple,
        elapsed_ms = st.elapsed().as_millis() as u64,
        "topic created"
    );
    Ok(Json(topic))
}

pub async fn topic_create_with_user(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(userid): Path<String>,
) -> ApiResult<Json<crate::Topic>> {
    let st = Instant::now();
    let self_user = auth.user_id();
    let topic_id = if self_user <= userid.as_str() {
        format!("{self_user}:{userid}")
    } else {
        format!("{userid}:{self_user}")
    };
    let topic = state
        .topic_service
        .create_topic(
            Some(topic_id),
            crate::OpenApiCreateTopicForm {
                sender_id: self_user.to_string(),
                members: vec![self_user.to_string(), userid],
                name: "Direct Message".to_string(),
                multiple: Some(false),
                ..crate::OpenApiCreateTopicForm::default()
            },
        )
        .await
        .map_err(map_domain_error)?;
    state
        .event_bus
        .publish(BackendEvent::TopicCreate(TopicSimpleEvent {
            topic_id: topic.id.clone(),
            admin_id: topic.owner_id.clone(),
            source: topic.source.clone(),
            webhooks: topic.webhooks.clone(),
        }));
    tracing::info!(
        user_id = %auth.user_id(),
        target_user_id = %topic.attendee_id,
        topic_id = %topic.id,
        elapsed_ms = st.elapsed().as_millis() as u64,
        "topic dm created"
    );
    Ok(Json(topic))
}

pub async fn topic_members(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(topic_id): Path<String>,
) -> ApiResult<Json<crate::ListUserResult>> {
    let st = Instant::now();
    let result = state
        .topic_service
        .list_members_detailed(&topic_id, None, None)
        .await
        .map_err(map_domain_error)?;
    tracing::info!(
        user_id = %auth.user_id(),
        topic_id = %topic_id,
        count = result.items.len(),
        elapsed_ms = st.elapsed().as_millis() as u64,
        "topic members listed"
    );
    Ok(Json(result))
}

pub async fn topic_invite(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path((topic_id, user_id)): Path<(String, String)>,
) -> ApiResult<Json<bool>> {
    let st = Instant::now();
    let topic = state
        .topic_service
        .get_by_id(&topic_id)
        .await
        .map_err(map_domain_error)?;
    auth.ensure_topic_admin(&topic)?;

    let users = state
        .topic_service
        .join_members(&topic_id, vec![user_id.clone()], "api".to_string())
        .await
        .map_err(map_domain_error)?;
    for joined in users {
        state
            .event_bus
            .publish(BackendEvent::TopicJoin(TopicUserEvent {
                topic_id: topic_id.clone(),
                admin_id: auth.user_id().to_string(),
                user_id: joined,
                source: "api".to_string(),
            }));
    }
    tracing::info!(
        user_id = %auth.user_id(),
        topic_id = %topic_id,
        target_user_id = %user_id,
        elapsed_ms = st.elapsed().as_millis() as u64,
        "topic member invited"
    );
    Ok(Json(true))
}

pub async fn topic_dismiss(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(topic_id): Path<String>,
) -> ApiResult<Json<bool>> {
    let st = Instant::now();
    let topic = state
        .topic_service
        .get_by_id(&topic_id)
        .await
        .map_err(map_domain_error)?;
    if topic.owner_id != auth.user_id() && !auth.is_staff {
        return Err(ApiError::Unauthorized);
    }
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
            source: "api".to_string(),
            webhooks: topic.webhooks.clone(),
        }));
    tracing::info!(
        user_id = %auth.user_id(),
        elapsed_ms = st.elapsed().as_millis() as u64,
        "topic dismissed"
    );
    Ok(Json(true))
}

pub async fn topic_quit(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(topic_id): Path<String>,
) -> ApiResult<Json<bool>> {
    let st = Instant::now();
    let _ = state
        .topic_service
        .quit_members(&topic_id, vec![auth.user_id().to_string()])
        .await
        .map_err(map_domain_error)?;
    state
        .event_bus
        .publish(BackendEvent::TopicQuit(TopicUserEvent {
            topic_id,
            admin_id: auth.user_id().to_string(),
            user_id: auth.user_id().to_string(),
            source: "api".to_string(),
        }));
    tracing::info!(
        user_id = %auth.user_id(),
        elapsed_ms = st.elapsed().as_millis() as u64,
        "topic quit"
    );
    Ok(Json(true))
}

pub async fn topic_knock(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(topic_id): Path<String>,
    Json(form): Json<crate::TopicKnockForm>,
) -> ApiResult<Json<bool>> {
    let st = Instant::now();
    let message = form.message.clone();
    let source = form.source.clone();
    state
        .topic_service
        .add_knock(&topic_id, auth.user_id(), form)
        .await
        .map_err(map_domain_error)?;
    state
        .event_bus
        .publish(BackendEvent::TopicKnock(TopicKnockEvent {
            topic_id,
            admin_id: String::new(),
            user_id: auth.user_id().to_string(),
            message,
            source,
        }));
    tracing::info!(
        user_id = %auth.user_id(),
        elapsed_ms = st.elapsed().as_millis() as u64,
        "topic knock sent"
    );
    Ok(Json(true))
}

pub async fn topic_admin_add_member(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path((topic_id, user_id)): Path<(String, String)>,
) -> ApiResult<Json<crate::TopicMember>> {
    let st = Instant::now();
    let topic = state
        .topic_service
        .get_by_id(&topic_id)
        .await
        .map_err(map_domain_error)?;
    auth.ensure_topic_admin(&topic)?;
    let users = state
        .topic_service
        .join_members(&topic_id, vec![user_id.clone()], "api".to_string())
        .await
        .map_err(map_domain_error)?;
    if users.is_empty() {
        return Err(ApiError::internal("failed to add member"));
    }
    let member = state
        .topic_service
        .get_member(&topic_id, &user_id)
        .await
        .map_err(map_domain_error)?;
    for joined in users {
        state
            .event_bus
            .publish(BackendEvent::TopicJoin(TopicUserEvent {
                topic_id: topic_id.clone(),
                admin_id: auth.user_id().to_string(),
                user_id: joined,
                source: "api".to_string(),
            }));
    }
    tracing::info!(
        user_id = %auth.user_id(),
        topic_id = %topic_id,
        target_user_id = %user_id,
        elapsed_ms = st.elapsed().as_millis() as u64,
        "topic member added by admin"
    );
    Ok(Json(member))
}

pub async fn topic_admin_update(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(topic_id): Path<String>,
    Json(form): Json<crate::OpenApiUpdateTopicForm>,
) -> ApiResult<Json<crate::Topic>> {
    let st = Instant::now();
    let topic = state
        .topic_service
        .get_by_id(&topic_id)
        .await
        .map_err(map_domain_error)?;
    auth.ensure_topic_admin(&topic)?;
    let topic = state
        .topic_service
        .update_topic(&topic_id, form)
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
    tracing::info!(
        user_id = %auth.user_id(),
        topic_id = %topic.id,
        elapsed_ms = st.elapsed().as_millis() as u64,
        "topic updated by admin"
    );
    Ok(Json(topic))
}

pub async fn topic_admin_transfer_owner(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path((topic_id, user_id)): Path<(String, String)>,
) -> ApiResult<Json<bool>> {
    let st = Instant::now();
    let topic = state
        .topic_service
        .get_by_id(&topic_id)
        .await
        .map_err(map_domain_error)?;
    auth.ensure_topic_admin(&topic)?;
    state
        .topic_service
        .transfer_owner(&topic_id, &user_id)
        .await
        .map_err(map_domain_error)?;
    state
        .event_bus
        .publish(BackendEvent::TopicChangeOwner(TopicChangeOwnerEvent {
            topic_id,
            admin_id: auth.user_id().to_string(),
            user_id,
            source: "api".to_string(),
        }));
    tracing::info!(
        user_id = %auth.user_id(),
        elapsed_ms = st.elapsed().as_millis() as u64,
        "topic owner transferred"
    );
    Ok(Json(true))
}

pub async fn topic_admin_add_admin(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path((topic_id, user_id)): Path<(String, String)>,
) -> ApiResult<Json<bool>> {
    let st = Instant::now();
    let topic = state
        .topic_service
        .get_by_id(&topic_id)
        .await
        .map_err(map_domain_error)?;
    auth.ensure_topic_admin(&topic)?;
    state
        .topic_service
        .add_admin(&topic_id, &user_id)
        .await
        .map_err(map_domain_error)?;
    tracing::info!(
        user_id = %auth.user_id(),
        topic_id = %topic_id,
        target_user_id = %user_id,
        elapsed_ms = st.elapsed().as_millis() as u64,
        "topic admin added"
    );
    Ok(Json(true))
}

pub async fn topic_admin_remove_admin(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path((topic_id, user_id)): Path<(String, String)>,
) -> ApiResult<Json<bool>> {
    let st = Instant::now();
    let topic = state
        .topic_service
        .get_by_id(&topic_id)
        .await
        .map_err(map_domain_error)?;
    auth.ensure_topic_admin(&topic)?;
    state
        .topic_service
        .remove_admin(&topic_id, &user_id)
        .await
        .map_err(map_domain_error)?;
    tracing::info!(
        user_id = %auth.user_id(),
        topic_id = %topic_id,
        target_user_id = %user_id,
        elapsed_ms = st.elapsed().as_millis() as u64,
        "topic admin removed"
    );
    Ok(Json(true))
}

pub async fn topic_admin_list_knock(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(topic_id): Path<String>,
) -> ApiResult<Json<Vec<crate::TopicKnock>>> {
    let st = Instant::now();
    let topic = state
        .topic_service
        .get_by_id(&topic_id)
        .await
        .map_err(map_domain_error)?;
    auth.ensure_topic_admin(&topic)?;
    let knocks = state
        .topic_service
        .list_pending_knocks(&topic_id)
        .await
        .map_err(map_domain_error)?;
    tracing::info!(
        user_id = %auth.user_id(),
        topic_id = %topic_id,
        count = knocks.len(),
        elapsed_ms = st.elapsed().as_millis() as u64,
        "topic pending knocks listed"
    );
    Ok(Json(knocks))
}

pub async fn topic_admin_accept_knock(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path((topic_id, user_id)): Path<(String, String)>,
    payload: Option<Json<crate::TopicKnockAcceptedForm>>,
) -> ApiResult<Json<bool>> {
    let st = Instant::now();
    let topic = state
        .topic_service
        .get_by_id(&topic_id)
        .await
        .map_err(map_domain_error)?;
    auth.ensure_topic_admin(&topic)?;
    state
        .topic_service
        .accept_knock(
            &topic_id,
            auth.user_id(),
            &user_id,
            payload.map(|v| v.0).unwrap_or_default(),
        )
        .await
        .map_err(map_domain_error)?;
    state
        .event_bus
        .publish(BackendEvent::TopicKnockAccept(TopicKnockEvent {
            topic_id,
            admin_id: auth.user_id().to_string(),
            user_id,
            message: String::new(),
            source: "api".to_string(),
        }));
    tracing::info!(
        user_id = %auth.user_id(),
        elapsed_ms = st.elapsed().as_millis() as u64,
        "topic knock accepted"
    );
    Ok(Json(true))
}

pub async fn topic_admin_reject_knock(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path((topic_id, user_id)): Path<(String, String)>,
    payload: Option<Json<crate::TopicKnockRejectedForm>>,
) -> ApiResult<Json<bool>> {
    let st = Instant::now();
    let topic = state
        .topic_service
        .get_by_id(&topic_id)
        .await
        .map_err(map_domain_error)?;
    auth.ensure_topic_admin(&topic)?;
    let message = payload
        .as_ref()
        .map(|v| v.0.message.clone())
        .unwrap_or_default();
    state
        .topic_service
        .reject_knock(
            &topic_id,
            auth.user_id(),
            &user_id,
            payload.map(|v| v.0).unwrap_or_default(),
        )
        .await
        .map_err(map_domain_error)?;
    state
        .event_bus
        .publish(BackendEvent::TopicKnockReject(TopicKnockEvent {
            topic_id,
            admin_id: auth.user_id().to_string(),
            user_id,
            message,
            source: "api".to_string(),
        }));
    tracing::info!(
        user_id = %auth.user_id(),
        elapsed_ms = st.elapsed().as_millis() as u64,
        "topic knock rejected"
    );
    Ok(Json(true))
}

pub async fn topic_admin_notice(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(topic_id): Path<String>,
    Json(form): Json<crate::UpdateNoticeForm>,
) -> ApiResult<Json<bool>> {
    let st = Instant::now();
    let topic = state
        .topic_service
        .get_by_id(&topic_id)
        .await
        .map_err(map_domain_error)?;
    auth.ensure_topic_admin(&topic)?;
    state
        .topic_service
        .update_notice(&topic_id, auth.user_id(), form)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    state
        .event_bus
        .publish(BackendEvent::TopicNotice(TopicNoticeEvent {
            topic_id,
            admin_id: auth.user_id().to_string(),
            message: "notice.update".to_string(),
        }));
    tracing::info!(
        user_id = %auth.user_id(),
        elapsed_ms = st.elapsed().as_millis() as u64,
        "topic notice updated"
    );
    Ok(Json(true))
}

pub async fn topic_admin_kickout(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path((topic_id, user_id)): Path<(String, String)>,
) -> ApiResult<Json<bool>> {
    let st = Instant::now();
    let topic = state
        .topic_service
        .get_by_id(&topic_id)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    auth.ensure_topic_admin(&topic)?;
    let _ = state
        .topic_service
        .quit_members(&topic_id, vec![user_id.clone()])
        .await
        .map_err(map_domain_error)?;
    state
        .event_bus
        .publish(BackendEvent::TopicKickout(TopicUserEvent {
            topic_id,
            admin_id: auth.user_id().to_string(),
            user_id,
            source: "api".to_string(),
        }));
    tracing::info!(
        user_id = %auth.user_id(),
        elapsed_ms = st.elapsed().as_millis() as u64,
        "topic member kicked out"
    );
    Ok(Json(true))
}

pub async fn topic_admin_silent_user(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path((topic_id, user_id)): Path<(String, String)>,
    Json(payload): Json<serde_json::Value>,
) -> ApiResult<Json<bool>> {
    let st = Instant::now();
    let topic = state
        .topic_service
        .get_by_id(&topic_id)
        .await
        .map_err(map_domain_error)?;
    auth.ensure_topic_admin(&topic)?;
    let duration = payload
        .get("duration")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let _ = state
        .topic_service
        .silent_member(
            &topic_id,
            crate::OpenApiSilentTopicMembersForm {
                user_ids: vec![user_id.clone()],
                duration: duration.clone(),
                ..crate::OpenApiSilentTopicMembersForm::default()
            },
        )
        .await
        .map_err(map_domain_error)?;
    state
        .event_bus
        .publish(BackendEvent::TopicSilentMember(TopicSilentEvent {
            topic_id,
            admin_id: auth.user_id().to_string(),
            user_id,
            duration,
            source: "api".to_string(),
        }));
    tracing::info!(
        user_id = %auth.user_id(),
        elapsed_ms = st.elapsed().as_millis() as u64,
        "topic member silenced"
    );
    Ok(Json(true))
}

pub async fn topic_admin_silent_topic(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(topic_id): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> ApiResult<Json<bool>> {
    let st = Instant::now();
    let topic = state
        .topic_service
        .get_by_id(&topic_id)
        .await
        .map_err(map_domain_error)?;
    auth.ensure_topic_admin(&topic)?;
    let duration = payload
        .get("duration")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    state
        .topic_service
        .silent_topic(
            &topic_id,
            crate::OpenApiSilentTopicForm {
                duration: duration.clone(),
                ..crate::OpenApiSilentTopicForm::default()
            },
        )
        .await
        .map_err(map_domain_error)?;
    state
        .event_bus
        .publish(BackendEvent::TopicSilent(TopicSilentEvent {
            topic_id,
            admin_id: auth.user_id().to_string(),
            user_id: String::new(),
            duration,
            source: "api".to_string(),
        }));
    tracing::info!(
        user_id = %auth.user_id(),
        elapsed_ms = st.elapsed().as_millis() as u64,
        "topic silenced"
    );
    Ok(Json(true))
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
