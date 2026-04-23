use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use axum::body::Body;
use axum::extract::{Multipart, Path as AxumPath, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::Response;
use axum::Json;
use chrono::Utc;
use fast_image_resize as fr;
use image::DynamicImage;
use sea_orm::ActiveModelTrait;
use sea_orm::EntityTrait;
use tokio::io::AsyncWriteExt;

use crate::api::auth_ctx::AuthCtx;
use crate::api::error::{ApiError, ApiResult};
use crate::app::AppState;
use crate::entity::attachment;
use crate::infra::event::{BackendEvent, UploadFileEvent};

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadResponse {
    pub path: String,
    pub file_name: String,
    #[serde(default)]
    pub thumbnail: String,
    #[serde(default)]
    pub ext: String,
    pub size: u64,
    #[serde(default)]
    pub external: bool,
    #[serde(rename = "publicUrl")]
    pub public_url: String,
}

fn parse_thumbnail_size(size: &str) -> u32 {
    let parsed = match size {
        "sm" => 512,
        "md" => 768,
        "lg" => 1024,
        _ => size.parse::<u32>().ok().filter(|v| *v > 0).unwrap_or(512),
    };
    parsed.min(MAX_THUMBNAIL_EDGE)
}

fn build_thumbnail_path(store_path: &str, size: u32) -> String {
    format!("{store_path}_{size}.jpeg")
}

fn should_thumbnail(ext: &str) -> bool {
    matches!(ext, ".png" | ".jpg" | ".jpeg")
}

const MAX_THUMBNAIL_SOURCE_PIXELS: u64 = 64 * 1024 * 1024;
const MAX_THUMBNAIL_EDGE: u32 = 2048;
const THUMBNAIL_TTL_SECS: u64 = 24 * 60 * 60;
const TMP_FILE_TTL_SECS: u64 = 30 * 60;

fn resize_image_to_jpeg(input: Vec<u8>, target_width: u32) -> Result<Vec<u8>, ApiError> {
    let decoded = image::load_from_memory(&input).map_err(|e| ApiError::internal(e.to_string()))?;
    let rgba = decoded.to_rgba8();
    let src_width = rgba.width();
    let src_height = rgba.height();
    if src_width == 0 || src_height == 0 {
        return Err(ApiError::bad_request("invalid image size"));
    }
    if src_width as u64 * src_height as u64 > MAX_THUMBNAIL_SOURCE_PIXELS {
        return Err(ApiError::bad_request("image too large for thumbnail"));
    }

    let dst_width = target_width.min(src_width).max(1);
    let dst_height = ((src_height as u64 * dst_width as u64) / src_width as u64).max(1) as u32;

    let src_image =
        fr::images::Image::from_vec_u8(src_width, src_height, rgba.into_raw(), fr::PixelType::U8x4)
            .map_err(|e| ApiError::internal(e.to_string()))?;
    let mut dst_image = fr::images::Image::new(dst_width, dst_height, fr::PixelType::U8x4);
    let mut resizer = fr::Resizer::new();
    resizer
        .resize(
            &src_image,
            &mut dst_image,
            &fr::ResizeOptions::new()
                .resize_alg(fr::ResizeAlg::Convolution(fr::FilterType::Lanczos3)),
        )
        .map_err(|e| ApiError::internal(e.to_string()))?;

    let output = image::RgbaImage::from_raw(dst_width, dst_height, dst_image.into_vec())
        .ok_or_else(|| ApiError::internal("invalid resized image buffer"))?;
    let mut encoded = Vec::new();
    DynamicImage::ImageRgba8(output)
        .write_to(
            &mut std::io::Cursor::new(&mut encoded),
            image::ImageFormat::Jpeg,
        )
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(encoded)
}

async fn read_or_create_thumbnail(store_path: String, size: String) -> Result<Vec<u8>, ApiError> {
    cleanup_attachment_artifacts(&store_path).await;
    let target_width = parse_thumbnail_size(&size);
    let thumbnail_path = build_thumbnail_path(&store_path, target_width);
    if Path::new(&thumbnail_path).exists() {
        return tokio::fs::read(&thumbnail_path)
            .await
            .map_err(|e| ApiError::internal(e.to_string()));
    }

    let thumbnail_bytes = tokio::task::spawn_blocking(move || {
        let source = std::fs::read(&store_path).map_err(|e| ApiError::internal(e.to_string()))?;
        let thumbnail_bytes = resize_image_to_jpeg(source, target_width)?;

        let tmp_path = PathBuf::from(format!(
            "{thumbnail_path}.tmp-{}",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::write(&tmp_path, &thumbnail_bytes)
            .map_err(|e| ApiError::internal(e.to_string()))?;
        std::fs::rename(&tmp_path, &thumbnail_path).or_else(|rename_err| {
            if Path::new(&thumbnail_path).exists() {
                let _ = std::fs::remove_file(&tmp_path);
                Ok(())
            } else {
                Err(ApiError::internal(rename_err.to_string()))
            }
        })?;
        Ok::<Vec<u8>, ApiError>(thumbnail_bytes)
    })
    .await
    .map_err(|e| ApiError::internal(e.to_string()))?;

    thumbnail_bytes
}

fn query_param(request: &axum::extract::Request, key: &str) -> Option<String> {
    request.uri().query().and_then(|q| {
        q.split('&')
            .find_map(|pair| pair.strip_prefix(&format!("{key}=")))
            .map(str::to_string)
    })
}

async fn remove_uploaded_file(path: &Path) {
    let _ = tokio::fs::remove_file(path).await;
}

async fn cleanup_attachment_artifacts(store_path: &str) {
    let store_path = store_path.to_string();
    let _ = tokio::task::spawn_blocking(move || {
        let base = PathBuf::from(&store_path);
        let parent = match base.parent() {
            Some(parent) => parent.to_path_buf(),
            None => return,
        };
        let file_name = match base.file_name().and_then(|v| v.to_str()) {
            Some(name) => name.to_string(),
            None => return,
        };
        let now = SystemTime::now();

        let Ok(entries) = std::fs::read_dir(parent) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(candidate) = path.file_name().and_then(|v| v.to_str()) else {
                continue;
            };
            let is_related_tmp =
                candidate.starts_with(&format!("{file_name}_")) && candidate.contains(".tmp-");
            let is_related_thumb =
                candidate.starts_with(&format!("{file_name}_")) && candidate.ends_with(".jpeg");
            if !is_related_tmp && !is_related_thumb {
                continue;
            }
            let ttl = if is_related_tmp {
                Duration::from_secs(TMP_FILE_TTL_SECS)
            } else {
                Duration::from_secs(THUMBNAIL_TTL_SECS)
            };
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            let Ok(modified) = meta.modified() else {
                continue;
            };
            if now
                .duration_since(modified)
                .ok()
                .is_some_and(|age| age >= ttl)
            {
                let _ = std::fs::remove_file(path);
            }
        }
    })
    .await;
}

async fn cleanup_upload_dir(dir: &Path) {
    let dir = dir.to_path_buf();
    let _ = tokio::task::spawn_blocking(move || {
        let now = SystemTime::now();
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(candidate) = path.file_name().and_then(|v| v.to_str()) else {
                continue;
            };
            if !candidate.contains(".tmp-") {
                continue;
            }
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            let Ok(modified) = meta.modified() else {
                continue;
            };
            if now
                .duration_since(modified)
                .ok()
                .is_some_and(|age| age >= Duration::from_secs(TMP_FILE_TTL_SECS))
            {
                let _ = std::fs::remove_file(path);
            }
        }
    })
    .await;
}

pub async fn upload(
    State(state): State<AppState>,
    auth: AuthCtx,
    mut multipart: Multipart,
) -> ApiResult<Json<UploadResponse>> {
    let mut file_name = String::new();
    let mut stored_size = 0usize;
    let mut topic_id = String::new();
    let mut is_private = false;
    let mut tags = String::new();
    let mut remark = String::new();
    let upload_dir = std::env::temp_dir().join("restsend-backend-uploads");
    tokio::fs::create_dir_all(&upload_dir)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    cleanup_upload_dir(&upload_dir).await;
    let mut store_path = None;

    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::bad_request(e.to_string()))?
    {
        match field.name() {
            Some("file") => {
                file_name = field
                    .file_name()
                    .map(str::to_string)
                    .unwrap_or_else(|| "upload.bin".to_string());
                let safe_name = file_name.replace('/', "_");
                let attachment_path = format!("{}-{}", uuid::Uuid::new_v4().simple(), safe_name);
                let candidate_path = upload_dir.join(&attachment_path);
                let mut output = tokio::fs::File::create(&candidate_path)
                    .await
                    .map_err(|e| ApiError::internal(e.to_string()))?;
                let mut total = 0usize;
                while let Some(chunk) = field
                    .chunk()
                    .await
                    .map_err(|e| ApiError::bad_request(e.to_string()))?
                {
                    total = total.saturating_add(chunk.len());
                    if total > state.config.max_upload_bytes {
                        tracing::warn!(
                            user_id = %auth.user_id(),
                            file_name = %file_name,
                            size = total,
                            max_upload_bytes = state.config.max_upload_bytes,
                            "attachment upload rejected: file too large"
                        );
                        drop(output);
                        remove_uploaded_file(&candidate_path).await;
                        return Err(ApiError::bad_request("file too large"));
                    }
                    output
                        .write_all(&chunk)
                        .await
                        .map_err(|e| ApiError::internal(e.to_string()))?;
                }
                output
                    .flush()
                    .await
                    .map_err(|e| ApiError::internal(e.to_string()))?;
                stored_size = total;
                store_path = Some((attachment_path, candidate_path));
            }
            Some("topicid") => {
                topic_id = field.text().await.unwrap_or_default();
            }
            Some("private") => {
                let value = field.text().await.unwrap_or_default().to_ascii_lowercase();
                is_private = value == "1" || value == "true";
            }
            Some("tags") => {
                tags = field.text().await.unwrap_or_default();
            }
            Some("remark") => {
                remark = field.text().await.unwrap_or_default();
            }
            _ => {}
        }
    }

    if file_name.is_empty() {
        return Err(ApiError::bad_request("file is required"));
    }
    let (attachment_path, store_path) =
        store_path.ok_or_else(|| ApiError::bad_request("file is required"))?;

    if !topic_id.is_empty() {
        let topic = match state.topic_service.get_by_id(&topic_id).await {
            Ok(topic) => topic,
            Err(err) => {
                remove_uploaded_file(&store_path).await;
                return Err(ApiError::bad_request(err.to_string()));
            }
        };
        let members = state
            .topic_service
            .list_members(&topic_id)
            .await
            .unwrap_or_default();
        if topic.owner_id != auth.user_id()
            && !members.iter().any(|member| member == auth.user_id())
        {
            remove_uploaded_file(&store_path).await;
            return Err(ApiError::bad_request("not member of topic"));
        }
    }

    let ext = Path::new(&file_name)
        .extension()
        .and_then(|v| v.to_str())
        .map(|v| format!(".{v}"))
        .unwrap_or_default();
    let now = Utc::now().to_rfc3339();
    let event_topic_id = topic_id.clone();
    let active = attachment::ActiveModel {
        path: sea_orm::ActiveValue::Set(attachment_path.clone()),
        file_name: sea_orm::ActiveValue::Set(file_name.clone()),
        store_path: sea_orm::ActiveValue::Set(store_path.to_string_lossy().to_string()),
        owner_id: sea_orm::ActiveValue::Set(auth.user_id().to_string()),
        topic_id: sea_orm::ActiveValue::Set(topic_id),
        size: sea_orm::ActiveValue::Set(stored_size as i64),
        ext: sea_orm::ActiveValue::Set(ext.clone()),
        private: sea_orm::ActiveValue::Set(is_private),
        external: sea_orm::ActiveValue::Set(false),
        tags: sea_orm::ActiveValue::Set(tags),
        remark: sea_orm::ActiveValue::Set(remark),
        created_at: sea_orm::ActiveValue::Set(now),
    };
    if let Err(err) = active.insert(&state.db).await {
        remove_uploaded_file(&store_path).await;
        return Err(ApiError::internal(err.to_string()));
    }

    let upload_payload = serde_json::json!({
        "path": attachment_path.clone(),
        "fileName": file_name.clone(),
        "ext": ext.clone(),
        "size": stored_size as u64,
        "external": false,
        "private": is_private,
    });
    state
        .event_bus
        .publish(BackendEvent::UploadFile(UploadFileEvent {
            topic_id: event_topic_id,
            user_id: auth.user_id().to_string(),
            data: upload_payload,
        }));

    Ok(Json(UploadResponse {
        path: attachment_path.clone(),
        file_name,
        thumbnail: String::new(),
        ext,
        size: stored_size as u64,
        external: false,
        public_url: format!("/api/attachment/{attachment_path}"),
    }))
}

pub async fn get_attachment(
    State(state): State<AppState>,
    auth: AuthCtx,
    AxumPath(filepath): AxumPath<String>,
    request: axum::extract::Request,
) -> Result<Response, ApiError> {
    let attachment = attachment::Entity::find_by_id(filepath.clone())
        .one(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .ok_or(ApiError::NotFound)?;

    if attachment.private && attachment.owner_id != auth.user_id() {
        return Err(ApiError::Unauthorized);
    }

    if attachment.external {
        let mut location = attachment.store_path.clone();
        if let Some(size) = query_param(&request, "size") {
            if location.contains('?') {
                location.push('&');
            } else {
                location.push('?');
            }
            location.push_str("size=");
            location.push_str(&size);
        }
        let mut resp = Response::new(Body::empty());
        *resp.status_mut() = StatusCode::FOUND;
        resp.headers_mut().insert(
            header::LOCATION,
            HeaderValue::from_str(&location).map_err(|e| ApiError::internal(e.to_string()))?,
        );
        return Ok(resp);
    }

    let requested_size = query_param(&request, "size");
    let (bytes, content_type) = match requested_size {
        Some(size) if should_thumbnail(&attachment.ext) => (
            read_or_create_thumbnail(attachment.store_path.clone(), size).await?,
            "image/jpeg",
        ),
        _ => (
            tokio::fs::read(&attachment.store_path)
                .await
                .map_err(|e| ApiError::internal(e.to_string()))?,
            match attachment.ext.as_str() {
                ".png" => "image/png",
                ".jpg" | ".jpeg" => "image/jpeg",
                ".gif" => "image/gif",
                _ => "application/octet-stream",
            },
        ),
    };

    let mut resp = Response::new(Body::from(bytes));
    *resp.status_mut() = StatusCode::OK;
    resp.headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    Ok(resp)
}
