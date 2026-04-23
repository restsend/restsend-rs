use axum::extract::State;
use axum::Json;
use sea_orm::{ActiveModelTrait, EntityTrait, IntoActiveModel, Set};
use std::time::Instant;

use crate::api::error::{ApiError, ApiResult};
use crate::app::AppState;
use crate::entity::user;
use crate::infra::event::{BackendEvent, UserGuestCreateEvent};

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthRegisterForm {
    pub email: String,
    pub password: String,
    #[serde(default)]
    pub remember: bool,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthLoginForm {
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub password: String,
    #[serde(default, alias = "token")]
    pub auth_token: String,
    #[serde(default)]
    pub remember: bool,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuestLoginForm {
    #[serde(default)]
    pub guest_id: String,
    #[serde(default)]
    pub remember: bool,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthLoginResponse {
    pub email: String,
    pub display_name: String,
    pub token: String,
    pub profile: AuthUserProfile,
    pub is_staff: bool,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthUserProfile {
    pub avatar: String,
    pub gender: String,
    pub city: String,
    pub region: String,
    pub country: String,
    pub private_extra: Option<std::collections::HashMap<String, String>>,
}

pub async fn register(
    State(state): State<AppState>,
    Json(form): Json<AuthRegisterForm>,
) -> ApiResult<Json<AuthLoginResponse>> {
    let st = Instant::now();
    if form.email.trim().is_empty() {
        return Err(ApiError::bad_request("email is required"));
    }
    if form.password.is_empty() {
        return Err(ApiError::bad_request("empty password"));
    }

    let created = state
        .user_service
        .register(
            &form.email,
            crate::OpenApiUserForm {
                display_name: form.email.clone(),
                ..crate::OpenApiUserForm::default()
            },
        )
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    let existing = user::Entity::find_by_id(form.email.clone())
        .one(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .ok_or_else(|| ApiError::internal("registered user missing"))?;
    let mut active = existing.into_active_model();
    active.password = Set(hash_password(&form.password));
    let saved = active
        .update(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    let token = state
        .auth_service
        .issue_token(&created.user_id)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    tracing::info!(
        user_id = %created.user_id,
        remember = form.remember,
        elapsed_ms = st.elapsed().as_millis() as u64,
        "auth register succeeded"
    );
    Ok(Json(to_auth_login_response(saved.into(), token)))
}

pub async fn login(
    State(state): State<AppState>,
    Json(form): Json<AuthLoginForm>,
) -> ApiResult<Json<AuthLoginResponse>> {
    let st = Instant::now();
    if !form.auth_token.is_empty() {
        let user_id = state
            .auth_service
            .validate(&form.auth_token)
            .await
            .map_err(|e| ApiError::internal(e.to_string()))?
            .ok_or_else(|| {
                tracing::warn!(
                    login_type = "token",
                    elapsed_ms = st.elapsed().as_millis() as u64,
                    "auth login rejected: invalid token"
                );
                ApiError::InvalidToken
            })?;
        let user = state
            .user_service
            .get_by_user_id(&user_id)
            .await
            .map_err(|e| ApiError::internal(e.to_string()))?;
        tracing::info!(
            user_id = %user_id,
            login_type = "token",
            remember = form.remember,
            elapsed_ms = st.elapsed().as_millis() as u64,
            "auth login succeeded"
        );
        return Ok(Json(to_auth_login_response(user, form.auth_token)));
    }

    if form.email.trim().is_empty() {
        return Err(ApiError::bad_request("email is required"));
    }
    if form.password.is_empty() {
        return Err(ApiError::bad_request("empty password"));
    }

    let model = user::Entity::find_by_id(form.email.clone())
        .one(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .ok_or_else(|| {
            tracing::warn!(
                email = %form.email,
                login_type = "password",
                elapsed_ms = st.elapsed().as_millis() as u64,
                "auth login rejected: user not found"
            );
            ApiError::InvalidToken
        })?;
    if !verify_password(&model.password, &form.password) {
        tracing::warn!(
            email = %form.email,
            login_type = "password",
            elapsed_ms = st.elapsed().as_millis() as u64,
            "auth login rejected: invalid password"
        );
        return Err(ApiError::bad_request("invalid password"));
    }

    let user: crate::User = model.into();
    let token = state
        .auth_service
        .issue_token(&user.user_id)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    tracing::info!(
        user_id = %user.user_id,
        login_type = "password",
        remember = form.remember,
        elapsed_ms = st.elapsed().as_millis() as u64,
        "auth login succeeded"
    );
    Ok(Json(to_auth_login_response(user, token)))
}

pub async fn logout(
    State(state): State<AppState>,
    auth: crate::api::auth_ctx::AuthCtx,
) -> ApiResult<Json<bool>> {
    let st = Instant::now();
    let _ = state
        .auth_service
        .revoke_token(&auth.token)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    tracing::info!(
        user_id = %auth.user_id,
        elapsed_ms = st.elapsed().as_millis() as u64,
        "auth logout succeeded"
    );
    Ok(Json(true))
}

pub async fn guest_login(
    State(state): State<AppState>,
    Json(form): Json<GuestLoginForm>,
) -> ApiResult<Json<AuthLoginResponse>> {
    let st = Instant::now();
    if form.guest_id.trim().is_empty() {
        return Err(ApiError::bad_request("guestId is required"));
    }

    let mut created_guest = false;
    let user = match state.user_service.get_by_user_id(&form.guest_id).await {
        Ok(user) => user,
        Err(_) => {
            created_guest = true;
            state
                .user_service
                .register(
                    &form.guest_id,
                    crate::OpenApiUserForm {
                        display_name: form.guest_id.clone(),
                        source: "guest".to_string(),
                        ..crate::OpenApiUserForm::default()
                    },
                )
                .await
                .map_err(|e| ApiError::internal(e.to_string()))?
        }
    };

    if created_guest {
        state
            .event_bus
            .publish(BackendEvent::UserGuestCreate(UserGuestCreateEvent {
                user_id: user.user_id.clone(),
            }));
    }

    let token = state
        .auth_service
        .issue_token(&user.user_id)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    tracing::info!(
        user_id = %user.user_id,
        remember = form.remember,
        elapsed_ms = st.elapsed().as_millis() as u64,
        "auth guest login succeeded"
    );
    Ok(Json(to_auth_login_response(user, token)))
}

fn to_auth_login_response(user: crate::User, token: String) -> AuthLoginResponse {
    let crate::User {
        user_id,
        name,
        avatar,
        gender,
        city,
        country,
        is_staff,
        ..
    } = user;
    let display_name = if name.is_empty() {
        user_id.clone()
    } else {
        name
    };
    let profile_avatar = if avatar.is_empty() {
        format!("/avatar/{user_id}")
    } else {
        avatar
    };
    AuthLoginResponse {
        email: user_id,
        display_name,
        token,
        profile: AuthUserProfile {
            avatar: profile_avatar,
            gender,
            city,
            region: String::new(),
            country,
            private_extra: None,
        },
        is_staff,
    }
}

pub(crate) fn hash_password(password: &str) -> String {
    if password.is_empty() {
        return String::new();
    }
    let salt = std::env::var("PASSWORD_SALT").unwrap_or_default();
    hash_password_with_salt(password, &salt)
}

pub(crate) fn verify_password(password_hash: &str, password: &str) -> bool {
    if password_hash.is_empty() {
        return password.is_empty();
    }
    let expected = hash_password(password);
    subtle_constant_time_eq(password_hash.as_bytes(), expected.as_bytes())
}

pub(crate) fn hash_password_with_salt(password: &str, salt: &str) -> String {
    if password.is_empty() {
        return String::new();
    }
    let digest = sha256_hex(&(salt.to_string() + password));
    format!("sha256${digest}")
}

fn sha256_hex(input: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn subtle_constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut diff = 0u8;
    for (a, b) in left.iter().zip(right.iter()) {
        diff |= a ^ b;
    }
    diff == 0
}
