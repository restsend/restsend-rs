//! Avatar endpoints: user letter avatars and composed topic member grids.

use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use crate::api::error::{ApiError, ApiResult};
use crate::app::AppState;

#[derive(Debug, Deserialize)]
pub struct AvatarQuery {
    pub size: Option<u32>,
}

/// GET /api/avatar/:userid — deterministic letter avatar (no auth, mirrors Go).
pub async fn user_avatar(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    Query(query): Query<AvatarQuery>,
) -> Result<Response, ApiError> {
    // user must exist (404 otherwise)
    let _ = state
        .user_service
        .get_any_by_user_id(&user_id)
        .await
        .map_err(|_| ApiError::NotFound)?;

    let size = query.size.unwrap_or(128);
    let key = (format!("user:{user_id}"), size);
    let bytes = state.avatar_cache.get_or_insert(key, || {
        crate::infra::letter_avatar::letter_avatar_png(&user_id, size)
    });
    Ok(png_response(bytes, 86400))
}

/// GET /api/topic/icon/:topicid — group icon composed from member avatars.
pub async fn topic_icon(
    State(state): State<AppState>,
    Path(topic_id): Path<String>,
    Query(query): Query<AvatarQuery>,
) -> Result<Response, ApiError> {
    let topic = state
        .topic_service
        .get_any_by_id(&topic_id)
        .await
        .map_err(|_| ApiError::NotFound)?;
    if !topic.icon.is_empty() {
        // explicit icon set: fetch and forward it
        if let Some(bytes) = fetch_avatar_bytes(&state, &topic.icon).await {
            return Ok(png_response(bytes, 86400));
        }
    }

    let member_ids = state
        .topic_service
        .list_members(&topic_id)
        .await
        .unwrap_or_default();
    let size = query.size.unwrap_or(256).clamp(64, 512);

    let mut cells = Vec::new();
    for user_id in member_ids.iter().take(9) {
        let key = (format!("user:{user_id}"), size);
        let bytes = state.avatar_cache.get_or_insert(key, || {
            crate::infra::letter_avatar::letter_avatar_png(user_id, size)
        });
        if let Ok(img) = image::load_from_memory(&bytes) {
            cells.push(img);
        }
    }

    let cache_key = (format!("topic:{topic_id}"), size);
    let bytes = state.avatar_cache.get_or_insert(cache_key, || {
        crate::infra::letter_avatar::pack_grid(cells, size)
    });
    Ok(png_response(bytes, 3600))
}

/// Resolve an avatar URL into raw image bytes.
/// Supports generated avatars (`/api/avatar/uid`), local attachments and
/// remote http(s) URLs.
async fn fetch_avatar_bytes(state: &AppState, url: &str) -> Option<Vec<u8>> {
    let url = url.trim();
    if url.is_empty() {
        return None;
    }
    if let Some(uid) = url
        .strip_prefix("/api/avatar/")
        .or_else(|| url.strip_prefix("/avatar/"))
    {
        let uid = uid.split('/').next().unwrap_or(uid);
        let key = (format!("user:{uid}"), 128);
        return Some(state.avatar_cache.get_or_insert(key, || {
            crate::infra::letter_avatar::letter_avatar_png(uid, 128)
        }));
    }
    if let Some(path) = url.strip_prefix("/api/attachment/") {
        let store_root = std::env::temp_dir().join("restsend-backend-uploads");
        let sanitized = path.replace("..", "");
        let bytes = tokio::fs::read(store_root.join(sanitized.trim_start_matches('/'))).await;
        return bytes.ok();
    }
    if url.starts_with("http://") || url.starts_with("https://") {
        return state.cluster_push_client.get(url).send().await.ok()?.bytes().await.ok().map(|b| b.to_vec());
    }
    None
}

fn png_response(bytes: Vec<u8>, max_age_secs: u64) -> Response {
    let mut resp = (StatusCode::OK, bytes).into_response();
    resp.headers_mut()
        .insert(header::CONTENT_TYPE, "image/png".parse().expect("mime"));
    resp.headers_mut().insert(
        header::CACHE_CONTROL,
        format!("public, max-age={max_age_secs}")
            .parse()
            .expect("cache header"),
    );
    resp
}
