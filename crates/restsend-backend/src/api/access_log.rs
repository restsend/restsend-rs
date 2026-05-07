use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use axum::extract::connect_info::ConnectInfo;
use axum::extract::State;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;

use crate::app::AppState;

#[derive(Clone, Debug)]
pub struct AccessLogUserId(pub Arc<Mutex<Option<String>>>);

pub async fn request_access_log(
    State(state): State<AppState>,
    mut req: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let started = Instant::now();
    let method = req.method().to_string();
    let path = req.uri().path().to_string();

    let client_ip = extract_client_ip(&req).unwrap_or_else(|| {
        req.extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map(|ConnectInfo(addr)| addr.ip().to_string())
            .unwrap_or_else(|| "-".to_string())
    });

    let user_slot = Arc::new(Mutex::new(None));
    req.extensions_mut()
        .insert(AccessLogUserId(user_slot.clone()));

    let resp = next.run(req).await;
    let status = resp.status().as_u16();
    let elapsed_ms = started.elapsed().as_millis() as u64;
    let user_id = user_slot
        .lock()
        .ok()
        .and_then(|v| v.clone())
        .unwrap_or_else(|| "-".to_string());
    let is_openapi = path.starts_with(&state.config.openapi_prefix);

    if is_openapi {
        tracing::info!(
            kind = "openapi",
            method = %method,
            path = %path,
            status = status,
            user_id = %user_id,
            client_ip = %client_ip,
            elapsed_ms = elapsed_ms,
            "openapi request"
        );
    } else {
        tracing::info!(
            kind = "http",
            method = %method,
            path = %path,
            status = status,
            user_id = %user_id,
            client_ip = %client_ip,
            elapsed_ms = elapsed_ms,
            "http request"
        );
    }

    resp
}

pub fn extract_client_ip(req: &Request<axum::body::Body>) -> Option<String> {
    for key in ["x-forwarded-for", "x-real-ip", "forwarded"] {
        if let Some(val) = req.headers().get(key).and_then(|v| v.to_str().ok()) {
            if !val.trim().is_empty() {
                return Some(val.trim().to_string());
            }
        }
    }
    None
}
