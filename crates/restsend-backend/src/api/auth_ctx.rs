use axum::extract::FromRef;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use sea_orm::EntityTrait;

use crate::api::error::ApiError;
use crate::app::AppState;
use crate::entity::user;

#[derive(Clone, Debug)]
pub struct AuthUserId(pub String);

#[derive(Clone, Debug)]
pub struct AuthToken(pub String);

#[derive(Clone, Debug)]
pub struct AuthCtx {
    pub user_id: String,
    pub token: String,
    pub is_staff: bool,
    pub is_super_openapi: bool,
}

impl AuthCtx {
    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    pub fn ensure_user_or_staff(&self, target_user_id: &str) -> Result<(), ApiError> {
        if self.user_id == target_user_id || self.is_staff || self.is_super_openapi {
            return Ok(());
        }
        Err(ApiError::Unauthorized)
    }

    pub fn ensure_staff(&self) -> Result<(), ApiError> {
        if self.is_staff || self.is_super_openapi {
            return Ok(());
        }
        Err(ApiError::Unauthorized)
    }

    pub fn ensure_topic_admin(&self, topic: &crate::Topic) -> Result<(), ApiError> {
        if self.is_staff
            || self.is_super_openapi
            || topic.owner_id == self.user_id
            || topic.admins.iter().any(|v| v == &self.user_id)
        {
            return Ok(());
        }
        Err(ApiError::Unauthorized)
    }
}

#[axum::async_trait]
impl<S> FromRequestParts<S> for AuthCtx
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let state = AppState::from_ref(state);

        let user_id = parts
            .extensions
            .get::<AuthUserId>()
            .map(|v| v.0.clone())
            .unwrap_or_else(|| "api-user".to_string());
        let token = parts
            .extensions
            .get::<AuthToken>()
            .map(|v| v.0.clone())
            .unwrap_or_default();

        let mut is_staff = false;
        if user_id != "api-user" {
            if let Ok(Some(model)) = user::Entity::find_by_id(user_id.clone())
                .one(&state.db)
                .await
            {
                is_staff = model.is_staff;
            }
        }

        let super_tokens = super_token_set();
        let is_super_openapi = parts.extensions.get::<AuthToken>().is_some_and(|token| {
            super_tokens.contains(&token.0)
                || state
                    .config
                    .openapi_token
                    .as_ref()
                    .is_some_and(|v| v == &token.0)
        });

        Ok(AuthCtx {
            user_id,
            token,
            is_staff,
            is_super_openapi,
        })
    }
}

fn super_token_set() -> std::collections::HashSet<String> {
    std::env::var("RS_SUPER_TOKENS")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
        .collect()
}
