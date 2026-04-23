use axum::extract::State;
use axum::response::Html;
use axum::Json;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, PaginatorTrait, QueryFilter, Set,
};
use serde::Deserialize;

use crate::api::auth_ctx::AuthCtx;
use crate::api::error::{ApiError, ApiResult};
use crate::app::AppState;

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminPerfStats {
    pub active_connections: usize,
    pub active_users: usize,
    pub online_users: u64,
    pub auth_tokens: u64,
    pub total_users: u64,
    pub enabled_users: u64,
    pub total_topics: u64,
    pub enabled_topics: u64,
    pub total_messages: u64,
    pub cluster: AdminClusterStats,
    pub pools: AdminPoolStats,
    pub metrics: crate::infra::metrics::RuntimeMetricsSnapshot,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminClusterStats {
    pub enabled: bool,
    pub current_node_id: String,
    pub current_endpoint: String,
    pub active_nodes: usize,
    pub nodes: Vec<AdminClusterNode>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminClusterNode {
    pub node_id: String,
    pub endpoint: String,
    pub sessions: u64,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminPoolStats {
    pub message: crate::infra::task_pool::TaskPoolSnapshot,
    pub push: crate::infra::task_pool::TaskPoolSnapshot,
    pub webhook: crate::infra::task_pool::TaskPoolSnapshot,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminConfigView {
    pub addr: String,
    pub endpoint: String,
    pub api_prefix: String,
    pub openapi_schema: String,
    pub openapi_prefix: String,
    pub message_worker_count: usize,
    pub message_queue_size: usize,
    pub push_worker_count: usize,
    pub push_queue_size: usize,
    pub webhook_worker_count: usize,
    pub webhook_queue_size: usize,
    pub event_bus_size: usize,
    pub max_upload_bytes: usize,
    pub webhook_timeout_secs: u64,
    pub webhook_retries: usize,
    pub webhook_targets: Vec<String>,
    pub presence_backend: String,
    pub presence_node_id: String,
    pub presence_ttl_secs: u64,
    pub presence_heartbeat_secs: u64,
    pub ws_per_user_limit: usize,
    pub ws_client_queue_size: usize,
    pub ws_typing_interval_ms: u64,
    pub ws_drop_on_backpressure: bool,
    pub has_openapi_token: bool,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminBootstrapState {
    pub initialized: bool,
    pub superuser_count: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminBootstrapInitForm {
    pub user_id: String,
    pub password: String,
    #[serde(default)]
    pub display_name: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminBootstrapInitResponse {
    pub user_id: String,
    pub token: String,
}

pub async fn spa() -> Result<Html<String>, ApiError> {
    let html = tokio::fs::read_to_string("static/admin.html")
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Html(html))
}

pub async fn config_view(
    State(state): State<AppState>,
    auth: AuthCtx,
) -> ApiResult<Json<AdminConfigView>> {
    ensure_admin(&auth)?;
    Ok(Json(AdminConfigView {
        addr: state.config.addr.clone(),
        endpoint: state.config.endpoint.clone(),
        api_prefix: state.config.api_prefix.clone(),
        openapi_schema: state.config.openapi_schema.clone(),
        openapi_prefix: state.config.openapi_prefix.clone(),
        message_worker_count: state.config.message_worker_count,
        message_queue_size: state.config.message_queue_size,
        push_worker_count: state.config.push_worker_count,
        push_queue_size: state.config.push_queue_size,
        webhook_worker_count: state.config.webhook_worker_count,
        webhook_queue_size: state.config.webhook_queue_size,
        event_bus_size: state.config.event_bus_size,
        max_upload_bytes: state.config.max_upload_bytes,
        webhook_timeout_secs: state.config.webhook_timeout_secs,
        webhook_retries: state.config.webhook_retries,
        webhook_targets: state.webhook_targets.as_ref().clone(),
        presence_backend: state.config.presence_backend.clone(),
        presence_node_id: state.config.presence_node_id.clone(),
        presence_ttl_secs: state.config.presence_ttl_secs,
        presence_heartbeat_secs: state.config.presence_heartbeat_secs,
        ws_per_user_limit: state.config.ws_per_user_limit,
        ws_client_queue_size: state.config.ws_client_queue_size,
        ws_typing_interval_ms: state.config.ws_typing_interval_ms,
        ws_drop_on_backpressure: state.config.ws_drop_on_backpressure,
        has_openapi_token: state.config.openapi_token.is_some(),
    }))
}

pub async fn bootstrap_state(
    State(state): State<AppState>,
) -> ApiResult<Json<AdminBootstrapState>> {
    let superuser_count = crate::entity::user::Entity::find()
        .filter(crate::entity::user::Column::IsStaff.eq(true))
        .count(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let initialized = crate::entity::user::Entity::find()
        .count(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        > 0;
    Ok(Json(AdminBootstrapState {
        initialized,
        superuser_count,
    }))
}

pub async fn bootstrap_init(
    State(state): State<AppState>,
    Json(form): Json<AdminBootstrapInitForm>,
) -> ApiResult<Json<AdminBootstrapInitResponse>> {
    let superuser_count = crate::entity::user::Entity::find()
        .filter(crate::entity::user::Column::IsStaff.eq(true))
        .count(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    if superuser_count > 0 {
        tracing::warn!("admin bootstrap rejected: superuser already exists");
        return Err(ApiError::Unauthorized);
    }
    if form.user_id.trim().is_empty() {
        return Err(ApiError::bad_request("userId is required"));
    }
    if form.password.is_empty() {
        return Err(ApiError::bad_request("password is required"));
    }

    let user = state
        .user_service
        .register(
            &form.user_id,
            crate::OpenApiUserForm {
                display_name: if form.display_name.trim().is_empty() {
                    form.user_id.clone()
                } else {
                    form.display_name.clone()
                },
                ..crate::OpenApiUserForm::default()
            },
        )
        .await
        .map_err(map_domain_error)?;
    let _ = state
        .user_service
        .set_staff(&user.user_id, true)
        .await
        .map_err(map_domain_error)?;

    let existing = crate::entity::user::Entity::find_by_id(user.user_id.clone())
        .one(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .ok_or_else(|| ApiError::internal("bootstrap user missing"))?;
    let mut active = existing.into_active_model();
    active.password = Set(crate::api::auth::hash_password(&form.password));
    active.enabled = Set(true);
    active.is_staff = Set(true);
    active
        .update(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    let token = state
        .auth_service
        .issue_token(&user.user_id)
        .await
        .map_err(map_domain_error)?;
    tracing::info!(user_id = %user.user_id, "admin bootstrap created first superuser");
    Ok(Json(AdminBootstrapInitResponse {
        user_id: user.user_id,
        token,
    }))
}

pub async fn perf_stats(
    State(state): State<AppState>,
    auth: AuthCtx,
) -> ApiResult<Json<AdminPerfStats>> {
    ensure_admin(&auth)?;
    let active_connections = state.ws_hub.total_sessions().await;
    let active_users = state.ws_hub.total_users().await;
    let online_users = crate::entity::presence_session::Entity::find()
        .count(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let auth_tokens = crate::entity::auth_token::Entity::find()
        .count(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let total_users = crate::entity::user::Entity::find()
        .count(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let enabled_users = crate::entity::user::Entity::find()
        .filter(crate::entity::user::Column::Enabled.eq(true))
        .count(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let total_topics = crate::entity::topic::Entity::find()
        .count(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let enabled_topics = crate::entity::topic::Entity::find()
        .filter(crate::entity::topic::Column::Enabled.eq(true))
        .count(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let total_messages = crate::entity::chat_log::Entity::find()
        .count(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let cluster_rows = crate::entity::presence_session::Entity::find()
        .all(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let mut cluster_map = std::collections::BTreeMap::<(String, String), u64>::new();
    for row in cluster_rows {
        let key = (row.node_id, row.endpoint);
        *cluster_map.entry(key).or_default() += 1;
    }
    let nodes = cluster_map
        .into_iter()
        .map(|((node_id, endpoint), sessions)| AdminClusterNode {
            node_id,
            endpoint,
            sessions,
        })
        .collect::<Vec<_>>();
    Ok(Json(AdminPerfStats {
        active_connections,
        active_users,
        online_users,
        auth_tokens,
        total_users,
        enabled_users,
        total_topics,
        enabled_topics,
        total_messages,
        cluster: AdminClusterStats {
            enabled: state.config.presence_backend == "db",
            current_node_id: state.config.presence_node_id.clone(),
            current_endpoint: state.config.endpoint.clone(),
            active_nodes: nodes.len(),
            nodes,
        },
        pools: AdminPoolStats {
            message: state.message_pool.snapshot(),
            push: state.push_pool.snapshot(),
            webhook: state.webhook_pool.snapshot(),
        },
        metrics: state.metrics.snapshot(),
    }))
}

fn ensure_admin(auth: &AuthCtx) -> Result<(), ApiError> {
    if auth.is_staff || auth.is_super_openapi {
        return Ok(());
    }
    tracing::warn!(
        user_id = %auth.user_id,
        is_staff = auth.is_staff,
        is_super_openapi = auth.is_super_openapi,
        "admin access rejected: not superuser"
    );
    Err(ApiError::Unauthorized)
}

fn map_domain_error(err: crate::services::DomainError) -> ApiError {
    match err {
        crate::services::DomainError::NotFound => ApiError::NotFound,
        crate::services::DomainError::Conflict => ApiError::bad_request("conflict"),
        crate::services::DomainError::Forbidden => ApiError::Unauthorized,
        crate::services::DomainError::Validation(msg) => ApiError::bad_request(msg),
        crate::services::DomainError::Storage(err) => ApiError::internal(err),
    }
}
