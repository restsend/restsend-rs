use axum::extract::State;
use axum::response::{Html, IntoResponse};
use axum::Json;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};
use serde::Deserialize;

use crate::api::auth_ctx::AuthCtx;
use crate::api::error::{ApiError, ApiResult};
use crate::app::{AppConfig, AppState};
use crate::infra::event::BackendEvent;

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

pub fn hinit_static_path(file: &str) -> Option<String> {
    for dir in [
        std::env::current_dir().ok(),
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf())),
    ] {
        if let Some(dir) = dir {
            let path = dir.join("static").join(file);
            if path.exists() {
                return Some(path.to_string_lossy().to_string());
            }
        }
    }
    None
}

fn static_path(file: &str) -> String {
    hinit_static_path(file).unwrap_or_else(|| {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("static")
            .join(file)
            .to_string_lossy()
            .to_string()
    })
}

pub async fn spa(State(_state): State<AppState>) -> Result<Html<String>, ApiError> {
    let path = static_path("admin.html");
    let html = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let html = if AppConfig::is_demo() {
        html.replace(
            "</head>",
            r#"<script>window.__DEMO_MODE__=true</script></head>"#,
        )
    } else {
        html
    };
    Ok(Html(html))
}

pub async fn demo_spa() -> Result<Html<String>, ApiError> {
    let path = static_path("demo.html");
    let html = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Html(html))
}

pub async fn chat_spa() -> Result<Html<String>, ApiError> {
    let path = static_path("chat.html");
    let html = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Html(html))
}

pub async fn desk_chat_spa() -> Result<Html<String>, ApiError> {
    let path = static_path("desk-chat/index.html");
    match tokio::fs::read_to_string(&path).await {
        Ok(html) => Ok(Html(html)),
        Err(_) => Ok(Html("Desk Chat not built yet. Run `pnpm build` in frontend/".to_string())),
    }
}

pub async fn livechat_spa() -> Result<Html<String>, ApiError> {
    let path = static_path("livechat-page/index.html");
    match tokio::fs::read_to_string(&path).await {
        Ok(html) => Ok(Html(html)),
        Err(_) => Ok(Html("Live Chat not built yet. Run `pnpm build` in frontend/".to_string())),
    }
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

/// Daily aggregated stats (`?days=N`, default 7).
pub async fn daily_stats(
    State(state): State<AppState>,
    auth: AuthCtx,
    axum::extract::Query(query): axum::extract::Query<StatsQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    ensure_admin(&auth)?;
    let days = query.days.unwrap_or(7).clamp(1, 365);
    let stats = state.stats.recent_daily(days).await;
    Ok(Json(serde_json::json!({ "items": stats })))
}

#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    pub days: Option<u32>,
}

/// Prometheus text exposition (mounted at PROMETHEUS_PREFIX, default /metrics).
pub async fn metrics_endpoint(State(state): State<AppState>) -> axum::response::Response {
    use axum::http::{header, StatusCode};
    let body = state.stats.render_prometheus();
    (
        StatusCode::OK,
        [(
            header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    )
        .into_response()
}

// ---------------------------------------------------------------------
// DB runtime configs (Go Config table parity)
// ---------------------------------------------------------------------

pub async fn list_configs(
    State(state): State<AppState>,
    auth: AuthCtx,
) -> ApiResult<Json<serde_json::Value>> {
    ensure_admin(&auth)?;
    let entries = state.config_service.list().await.map_err(map_domain_error)?;
    let items: Vec<serde_json::Value> = entries
        .into_iter()
        .map(|(key, value)| serde_json::json!({"key": key, "value": value}))
        .collect();
    Ok(Json(serde_json::json!({ "items": items })))
}

#[derive(Debug, Deserialize)]
pub struct UpdateConfigForm {
    #[serde(default)]
    pub value: String,
}

pub async fn update_config(
    State(state): State<AppState>,
    auth: AuthCtx,
    axum::extract::Path(key): axum::extract::Path<String>,
    Json(form): Json<UpdateConfigForm>,
) -> ApiResult<Json<serde_json::Value>> {
    ensure_admin(&auth)?;
    state
        .config_service
        .set(&key, &form.value)
        .await
        .map_err(map_domain_error)?;
    tracing::info!(key = %key, admin = %auth.user_id, "runtime config updated");
    Ok(Json(serde_json::json!({"key": key, "value": form.value})))
}

// ---------------------------------------------------------------------
// Admin object CRUD (topics/users/conversations/attachments/knocks/relations/messages)
// ---------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ObjectListQuery {
    pub offset: Option<u64>,
    pub limit: Option<u64>,
    pub keyword: Option<String>,
}

pub async fn object_list(
    State(state): State<AppState>,
    auth: AuthCtx,
    axum::extract::Path(name): axum::extract::Path<String>,
    axum::extract::Query(query): axum::extract::Query<ObjectListQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    ensure_admin(&auth)?;
    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(20).clamp(1, 200);
    let keyword = query.keyword.unwrap_or_default();

    let json = match name.as_str() {
        "users" => {
            let mut q = crate::entity::user::Entity::find();
            if !keyword.is_empty() {
                q = q.filter(
                    sea_orm::Condition::any()
                        .add(crate::entity::user::Column::UserId.contains(keyword.clone()))
                        .add(crate::entity::user::Column::DisplayName.contains(keyword)),
                );
            }
            let total = q.clone().count(&state.db).await.map_err(db_err)?;
            let rows = q.offset(offset).limit(limit).all(&state.db).await.map_err(db_err)?;
            serde_json::json!({
                "total": total,
                "items": rows.iter().map(|u| serde_json::json!({
                    "userId": u.user_id, "displayName": u.display_name, "avatar": u.avatar,
                    "isStaff": u.is_staff, "enabled": u.enabled, "createdAt": u.created_at,
                })).collect::<Vec<_>>(),
            })
        }
        "topics" => {
            let (rows, total) = state
                .topic_service
                .list_topics(offset, limit, Some(&keyword))
                .await
                .map_err(map_domain_error)?;
            serde_json::json!({
                "total": total,
                "items": rows,
            })
        }
        "conversations" => {
            let q = crate::entity::conversation::Entity::find()
                .filter(crate::entity::conversation::Column::OwnerId.contains(keyword.clone()));
            let total = q.clone().count(&state.db).await.map_err(db_err)?;
            let rows = q.offset(offset).limit(limit).all(&state.db).await.map_err(db_err)?;
            serde_json::json!({
                "total": total,
                "items": rows.iter().map(crate::Conversation::from).collect::<Vec<_>>(),
            })
        }
        "attachments" => {
            let q = crate::entity::attachment::Entity::find().filter(
                sea_orm::Condition::any()
                    .add(crate::entity::attachment::Column::FileName.contains(keyword.clone()))
                    .add(crate::entity::attachment::Column::Path.contains(keyword)),
            );
            let total = q.clone().count(&state.db).await.map_err(db_err)?;
            let rows = q.offset(offset).limit(limit).all(&state.db).await.map_err(db_err)?;
            let items: Vec<serde_json::Value> = rows
                .into_iter()
                .map(|row| {
                    serde_json::json!({
                        "path": row.path,
                        "fileName": row.file_name,
                        "ownerId": row.owner_id,
                        "topicId": row.topic_id,
                        "size": row.size,
                        "ext": row.ext,
                        "private": row.private,
                        "external": row.external,
                        "createdAt": row.created_at,
                    })
                })
                .collect();
            serde_json::json!({
                "total": total,
                "items": items,
            })
        }
        "knocks" => {
            let q = crate::entity::topic_knock::Entity::find();
            let total = q.clone().count(&state.db).await.map_err(db_err)?;
            let rows = q.offset(offset).limit(limit).all(&state.db).await.map_err(db_err)?;
            let items: Vec<crate::TopicKnock> = rows
                .into_iter()
                .map(|row| crate::TopicKnock {
                    topic_id: row.topic_id,
                    user_id: row.user_id,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                    message: row.message,
                    source: row.source,
                    status: row.status,
                    admin_id: row.admin_id,
                })
                .collect();
            serde_json::json!({ "total": total, "items": items })
        }
        "relations" => {
            let q = crate::entity::relation::Entity::find()
                .filter(crate::entity::relation::Column::OwnerId.contains(keyword));
            let total = q.clone().count(&state.db).await.map_err(db_err)?;
            let rows = q.offset(offset).limit(limit).all(&state.db).await.map_err(db_err)?;
            let items: Vec<serde_json::Value> = rows
                .into_iter()
                .map(|row| {
                    serde_json::json!({
                        "ownerId": row.owner_id,
                        "targetId": row.target_id,
                        "isContact": row.is_contact,
                        "isStar": row.is_star,
                        "isBlocked": row.is_blocked,
                        "remark": row.remark,
                        "source": row.source,
                        "updatedAt": row.updated_at,
                    })
                })
                .collect();
            serde_json::json!({ "total": total, "items": items })
        }
        "messages" => {
            let q = crate::entity::chat_log::Entity::find()
                .filter(crate::entity::chat_log::Column::TopicId.contains(keyword));
            let total = q.clone().count(&state.db).await.map_err(db_err)?;
            let rows = q
                .order_by_desc(crate::entity::chat_log::Column::Seq)
                .offset(offset)
                .limit(limit)
                .all(&state.db)
                .await
                .map_err(db_err)?;
            serde_json::json!({
                "total": total,
                "items": rows.iter().map(crate::ChatLog::from).collect::<Vec<_>>(),
            })
        }
        other => return Err(ApiError::bad_request(format!("unknown object type: {other}"))),
    };
    Ok(Json(json))
}

pub async fn object_delete(
    State(state): State<AppState>,
    auth: AuthCtx,
    axum::extract::Path((name, id)): axum::extract::Path<(String, String)>,
) -> ApiResult<Json<bool>> {
    ensure_admin(&auth)?;
    match name.as_str() {
        "users" => {
            state
                .user_service
                .deactive(&id)
                .await
                .map_err(map_domain_error)?;
        }
        "topics" => {
            state
                .topic_service
                .dismiss_topic(&id)
                .await
                .map_err(map_domain_error)?;
            state
                .event_bus
                .publish(BackendEvent::TopicDismiss(crate::infra::event::TopicSimpleEvent {
                    topic_id: id.clone(),
                    admin_id: auth.user_id.clone(),
                    source: "admin".to_string(),
                    webhooks: vec![],
                }));
        }
        "conversations" => {
            // id format: ownerId:topicId
            let (owner_id, topic_id) = id
                .split_once(':')
                .ok_or_else(|| ApiError::bad_request("id must be ownerId:topicId"))?;
            state
                .conversation_service
                .remove_conversation(owner_id, topic_id)
                .await
                .map_err(map_domain_error)?;
        }
        "attachments" => {
            crate::entity::attachment::Entity::delete_by_id(id.clone())
                .exec(&state.db)
                .await
                .map_err(db_err)?;
        }
        "knocks" => {
            let (topic_id, user_id) = id
                .split_once(':')
                .ok_or_else(|| ApiError::bad_request("id must be topicId:userId"))?;
            crate::entity::topic_knock::Entity::delete_by_id((
                topic_id.to_string(),
                user_id.to_string(),
            ))
            .exec(&state.db)
            .await
            .map_err(db_err)?;
        }
        "relations" => {
            let (owner_id, target_id) = id
                .split_once(':')
                .ok_or_else(|| ApiError::bad_request("id must be ownerId:targetId"))?;
            crate::entity::relation::Entity::delete_by_id((
                owner_id.to_string(),
                target_id.to_string(),
            ))
            .exec(&state.db)
            .await
            .map_err(db_err)?;
        }
        "messages" => {
            let (topic_id, chat_id) = id
                .split_once(':')
                .ok_or_else(|| ApiError::bad_request("id must be topicId:chatId"))?;
            crate::entity::chat_log::Entity::delete_by_id((
                topic_id.to_string(),
                chat_id.to_string(),
            ))
            .exec(&state.db)
            .await
            .map_err(db_err)?;
        }
        other => return Err(ApiError::bad_request(format!("unknown object type: {other}"))),
    }
    tracing::info!(object = %name, id = %id, admin = %auth.user_id, "admin object deleted");
    Ok(Json(true))
}

fn db_err(err: sea_orm::DbErr) -> ApiError {
    ApiError::internal(err.to_string())
}

pub async fn demo_users(State(state): State<AppState>) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let demo_ids = ["alice", "bob", "guido", "jinti"];
    let mut users = Vec::new();
    for user_id in &demo_ids {
        if let Ok(u) = state.user_service.get_any_by_user_id(user_id).await {
            let password = format!("{}:demo", user_id);
            users.push(serde_json::json!({
                "userId": u.user_id,
                "displayName": u.name,
                "avatar": u.avatar,
                "password": password,
            }));
        }
    }
    Ok(Json(users))
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
