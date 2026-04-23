use axum::extract::{Path, State};
use axum::Json;
use sea_orm::EntityTrait;
use std::time::Instant;

use crate::api::auth_ctx::AuthCtx;
use crate::api::error::{ApiError, ApiResult};
use crate::app::AppState;

pub async fn devices(State(state): State<AppState>, auth: AuthCtx) -> ApiResult<Json<Vec<String>>> {
    let st = Instant::now();
    let snapshot = state.presence_hub.snapshot(auth.user_id()).await;
    tracing::info!(
        user_id = %auth.user_id(),
        device_count = snapshot.devices.len(),
        elapsed_ms = st.elapsed().as_millis() as u64,
        "user devices fetched"
    );
    Ok(Json(snapshot.devices))
}

pub async fn connect() -> Json<bool> {
    Json(true)
}

pub async fn kick(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(cid): Path<String>,
) -> ApiResult<Json<bool>> {
    let st = Instant::now();
    let payload = serde_json::json!({
        "type": "kick",
        "message": "You have been kicked out by other client",
    })
    .to_string();
    crate::api::push::send_to_device(&state, auth.user_id(), &cid, &payload).await;
    tracing::info!(
        user_id = %auth.user_id(),
        cid = %cid,
        elapsed_ms = st.elapsed().as_millis() as u64,
        "user kick sent"
    );
    Ok(Json(true))
}

pub async fn profiles(
    State(state): State<AppState>,
    auth: AuthCtx,
    Json(payload): Json<serde_json::Value>,
) -> ApiResult<Json<Vec<crate::User>>> {
    let st = Instant::now();
    let user_ids: Vec<String> = payload
        .get("userIds")
        .or_else(|| payload.get("ids"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let mut out = Vec::new();
    for user_id in user_ids {
        if let Ok(user) = state.user_service.get_by_user_id(&user_id).await {
            out.push(user);
        }
    }
    tracing::info!(
        user_id = %auth.user_id(),
        count = out.len(),
        elapsed_ms = st.elapsed().as_millis() as u64,
        "user profiles fetched"
    );
    Ok(Json(out))
}

pub async fn single_profile(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(userid): Path<String>,
) -> ApiResult<Json<crate::User>> {
    let st = Instant::now();
    let mut user = state
        .user_service
        .get_by_user_id(&userid)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    if userid != auth.user_id() {
        if let Ok(Some(relation)) = crate::entity::relation::Entity::find_by_id((
            auth.user_id().to_string(),
            userid.clone(),
        ))
        .one(&state.db)
        .await
        {
            user.remark = relation.remark;
            user.is_contact = relation.is_contact;
            user.is_star = relation.is_star;
            user.is_blocked = relation.is_blocked;
        }
    }
    tracing::info!(
        user_id = %auth.user_id(),
        target_user_id = %userid,
        elapsed_ms = st.elapsed().as_millis() as u64,
        "user profile fetched"
    );
    Ok(Json(user))
}

pub async fn update_profile(
    State(state): State<AppState>,
    auth: AuthCtx,
    Json(form): Json<crate::OpenApiUserForm>,
) -> ApiResult<Json<bool>> {
    let st = Instant::now();
    state
        .user_service
        .update(auth.user_id(), form)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    tracing::info!(
        user_id = %auth.user_id(),
        elapsed_ms = st.elapsed().as_millis() as u64,
        "user profile updated"
    );
    Ok(Json(true))
}

pub async fn update_relation(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(userid): Path<String>,
    Json(mut form): Json<crate::OpenApiRelationEditForm>,
) -> ApiResult<Json<bool>> {
    let st = Instant::now();
    if userid == auth.user_id() {
        return Err(ApiError::bad_request("invalid params"));
    }
    if let Some(is_star) = form.is_star {
        form.is_contact = Some(form.is_contact.unwrap_or(is_star));
    }
    let _ = state
        .relation_service
        .update_relation(auth.user_id(), &userid, form)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    tracing::info!(
        user_id = %auth.user_id(),
        target_user_id = %userid,
        elapsed_ms = st.elapsed().as_millis() as u64,
        "user relation updated"
    );
    Ok(Json(true))
}

pub async fn list_blocked(
    State(state): State<AppState>,
    auth: AuthCtx,
) -> ApiResult<Json<Vec<String>>> {
    let st = Instant::now();
    let users = state
        .relation_service
        .list_blocked(auth.user_id())
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    tracing::info!(
        user_id = %auth.user_id(),
        count = users.len(),
        elapsed_ms = st.elapsed().as_millis() as u64,
        "user blocked list fetched"
    );
    Ok(Json(users))
}

pub async fn block_user(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(userid): Path<String>,
) -> ApiResult<Json<bool>> {
    let st = Instant::now();
    let _ = state
        .relation_service
        .update_blocked(auth.user_id(), std::slice::from_ref(&userid), true)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    tracing::info!(
        user_id = %auth.user_id(),
        target_user_id = %userid,
        elapsed_ms = st.elapsed().as_millis() as u64,
        "user blocked"
    );
    Ok(Json(true))
}

pub async fn unblock_user(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(userid): Path<String>,
) -> ApiResult<Json<bool>> {
    let st = Instant::now();
    let _ = state
        .relation_service
        .update_blocked(auth.user_id(), std::slice::from_ref(&userid), false)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    tracing::info!(
        user_id = %auth.user_id(),
        target_user_id = %userid,
        elapsed_ms = st.elapsed().as_millis() as u64,
        "user unblocked"
    );
    Ok(Json(true))
}
