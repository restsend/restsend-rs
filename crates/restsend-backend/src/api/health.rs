use axum::extract::State;
use axum::Json;
use sea_orm::ConnectionTrait;
use serde::Serialize;

use crate::api::error::ApiResult;
use crate::app::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    pub status: &'static str,
    pub service: &'static str,
}

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "restsend-backend",
    })
}

pub async fn live() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "alive",
        service: "restsend-backend",
    })
}

pub async fn ready(State(state): State<AppState>) -> ApiResult<Json<HealthResponse>> {
    state
        .db
        .execute_unprepared("SELECT 1")
        .await
        .map_err(|err| crate::api::error::ApiError::internal(err.to_string()))?;

    Ok(Json(HealthResponse {
        status: "ready",
        service: "restsend-backend",
    }))
}
