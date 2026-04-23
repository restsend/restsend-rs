use axum::http::StatusCode;
use reqwest::Url;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::app::AppState;

pub async fn broadcast_to_user(state: &AppState, user_id: &str, payload: &str) {
    let state = state.clone();
    let push_pool = state.push_pool.clone();
    let user_id = user_id.to_string();
    let payload = payload.to_string();
    let log_user_id = user_id.clone();
    if let Err(err) = push_pool
        .submit(async move {
            push_local_user_now(&state, &user_id, &payload).await;
            push_remote_user_now(&state, &user_id, None, &payload).await;
        })
        .await
    {
        tracing::warn!(user_id = %log_user_id, error = %err, "enqueue broadcast push failed");
    }
}

pub async fn send_to_device(state: &AppState, user_id: &str, device: &str, payload: &str) {
    let state = state.clone();
    let push_pool = state.push_pool.clone();
    let user_id = user_id.to_string();
    let device = device.to_string();
    let payload = payload.to_string();
    let log_user_id = user_id.clone();
    let log_device = device.clone();
    if let Err(err) = push_pool
        .submit(async move {
            push_local_device_now(&state, &user_id, &device, &payload).await;
            push_remote_user_now(&state, &user_id, Some(&device), &payload).await;
        })
        .await
    {
        tracing::warn!(user_id = %log_user_id, device = %log_device, error = %err, "enqueue device push failed");
    }
}

pub async fn push_local_user(state: &AppState, user_id: &str, payload: &str) {
    let state = state.clone();
    let push_pool = state.push_pool.clone();
    let user_id = user_id.to_string();
    let payload = payload.to_string();
    let log_user_id = user_id.clone();
    if let Err(err) = push_pool
        .submit(async move {
            push_local_user_now(&state, &user_id, &payload).await;
        })
        .await
    {
        tracing::warn!(user_id = %log_user_id, error = %err, "enqueue local user push failed");
    }
}

pub async fn push_local_device(state: &AppState, user_id: &str, device: &str, payload: &str) {
    let state = state.clone();
    let push_pool = state.push_pool.clone();
    let user_id = user_id.to_string();
    let device = device.to_string();
    let payload = payload.to_string();
    let log_user_id = user_id.clone();
    let log_device = device.clone();
    if let Err(err) = push_pool
        .submit(async move {
            push_local_device_now(&state, &user_id, &device, &payload).await;
        })
        .await
    {
        tracing::warn!(user_id = %log_user_id, device = %log_device, error = %err, "enqueue local device push failed");
    }
}

async fn push_local_user_now(state: &AppState, user_id: &str, payload: &str) {
    state
        .ws_hub
        .broadcast_to_user(user_id, payload, state.config.ws_drop_on_backpressure)
        .await;
}

async fn push_local_device_now(state: &AppState, user_id: &str, device: &str, payload: &str) {
    state
        .ws_hub
        .send_to_device(
            user_id,
            device,
            payload,
            state.config.ws_drop_on_backpressure,
        )
        .await;
}

async fn push_remote_user_now(
    state: &AppState,
    user_id: &str,
    device: Option<&str>,
    payload: &str,
) {
    if state.config.presence_backend != "db" || state.config.endpoint.is_empty() {
        return;
    }

    let cutoff = chrono::Utc::now().timestamp() - state.config.presence_ttl_secs as i64;
    let mut query = crate::entity::presence_session::Entity::find()
        .filter(crate::entity::presence_session::Column::UserId.eq(user_id.to_string()))
        .filter(crate::entity::presence_session::Column::UpdatedAtUnix.gte(cutoff));
    if let Some(device) = device {
        query =
            query.filter(crate::entity::presence_session::Column::Device.eq(device.to_string()));
    }

    let rows = match query.all(&state.db).await {
        Ok(rows) => rows,
        Err(err) => {
            tracing::warn!(user_id = %user_id, error = %err, "cluster push lookup failed");
            return;
        }
    };

    for row in rows {
        if row.endpoint.trim().is_empty() || row.endpoint == state.config.endpoint {
            continue;
        }
        if let Err(err) =
            send_remote_push(state, &row.endpoint, &row.user_id, &row.device, payload).await
        {
            tracing::warn!(
                user_id = %row.user_id,
                device = %row.device,
                endpoint = %row.endpoint,
                error = %err,
                "cluster push failed"
            );
        } else {
            tracing::info!(
                user_id = %row.user_id,
                device = %row.device,
                endpoint = %row.endpoint,
                "cluster push forwarded"
            );
        }
    }
}

async fn send_remote_push(
    state: &AppState,
    endpoint: &str,
    user_id: &str,
    device: &str,
    payload: &str,
) -> Result<(), String> {
    let mut url = Url::parse(&format!("{}://{}", state.config.openapi_schema, endpoint))
        .map_err(|err| err.to_string())?;
    {
        let mut segments = url
            .path_segments_mut()
            .map_err(|_| "invalid endpoint".to_string())?;
        for segment in state
            .config
            .openapi_prefix
            .trim_start_matches('/')
            .split('/')
        {
            if !segment.is_empty() {
                segments.push(segment);
            }
        }
        segments.push("user");
        segments.push("push");
        segments.push(user_id);
        segments.push(device);
    }

    let mut req = state
        .cluster_push_client
        .post(url)
        .json(&crate::OpenApiPushForm {
            message: payload.to_string(),
            ..crate::OpenApiPushForm::default()
        });
    if let Some(token) = state.config.openapi_token.as_deref() {
        req = req.bearer_auth(token);
    }
    let resp = req.send().await.map_err(|err| err.to_string())?;
    if resp.status() != StatusCode::OK {
        return Err(format!("unexpected status {}", resp.status()));
    }
    Ok(())
}
