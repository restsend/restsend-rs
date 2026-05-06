use axum::extract::State;
use axum::http::{header, Request};
use axum::middleware::Next;
use axum::response::Response;

use crate::api::access_log::AccessLogUserId;
use crate::api::auth_ctx::{AuthToken, AuthUserId};
use crate::api::error::ApiError;
use crate::app::AppState;
use crate::services::parse_bearer_token;

pub async fn openapi_auth(
    State(state): State<AppState>,
    mut req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, ApiError> {
    let expected = state.config.openapi_token.as_deref();
    let Some(header_val) = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    else {
        if expected.is_some() {
            tracing::warn!("openapi auth rejected: missing authorization header");
            return Err(ApiError::Unauthorized);
        }
        return Ok(next.run(req).await);
    };

    let token = parse_bearer_token(header_val).ok_or_else(|| {
        tracing::warn!("openapi auth rejected: invalid bearer format");
        ApiError::Unauthorized
    })?;
    let token = token.to_string();
    if expected.is_some_and(|expected| token == expected) {
        req.extensions_mut().insert(AuthToken(token));
        if let Some(slot) = req.extensions().get::<AccessLogUserId>() {
            if let Ok(mut guard) = slot.0.lock() {
                *guard = Some("openapi-super-token".to_string());
            }
        }
        return Ok(next.run(req).await);
    }

    let valid = state
        .auth_service
        .validate(&token)
        .await
        .map_err(|err| ApiError::internal(err.to_string()))?;
    let Some(user_id) = valid else {
        tracing::warn!("openapi auth rejected: invalid token");
        return Err(ApiError::InvalidToken);
    };
    let user_for_log = user_id.clone();
    req.extensions_mut().insert(AuthToken(token));
    req.extensions_mut().insert(AuthUserId(user_id));
    if let Some(slot) = req.extensions().get::<AccessLogUserId>() {
        if let Ok(mut guard) = slot.0.lock() {
            *guard = Some(user_for_log);
        }
    }

    Ok(next.run(req).await)
}

pub async fn user_auth(
    State(state): State<AppState>,
    mut req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, ApiError> {
    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(parse_bearer_token)
        .map(str::to_string)
        .or_else(|| {
            req.headers()
                .get(header::COOKIE)
                .and_then(|v| v.to_str().ok())
                .and_then(|cookies| {
                    cookies.split(';').find_map(|pair| {
                        let pair = pair.trim();
                        pair.strip_prefix("token=").map(str::to_string)
                    })
                })
        })
        .ok_or_else(|| {
            tracing::warn!("user auth rejected: missing authorization header or token cookie");
            ApiError::Unauthorized
        })?;
    let valid = state
        .auth_service
        .validate(&token)
        .await
        .map_err(|err| ApiError::internal(err.to_string()))?;
    let Some(user_id) = valid else {
        tracing::warn!("user auth rejected: invalid token");
        return Err(ApiError::InvalidToken);
    };
    let user_for_log = user_id.clone();

    req.extensions_mut().insert(AuthToken(token));
    req.extensions_mut().insert(AuthUserId(user_id));
    if let Some(slot) = req.extensions().get::<AccessLogUserId>() {
        if let Ok(mut guard) = slot.0.lock() {
            *guard = Some(user_for_log);
        }
    }
    Ok(next.run(req).await)
}
