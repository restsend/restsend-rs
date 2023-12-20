use crate::error::ClientError::{Forbidden, InvalidPassword, HTTP};
use crate::utils::{elapsed, now_millis};
use crate::Result;
#[cfg(not(target_family = "wasm"))]
use crate::USER_AGENT;
use log::{debug, warn};
use reqwest::{
    header::{HeaderValue, AUTHORIZATION, CONTENT_TYPE},
    ClientBuilder, RequestBuilder, Response,
};
use std::time::Duration;

pub mod auth;
pub mod conversation;
pub mod response;
#[cfg(test)]
mod tests;
pub mod topic;
pub mod topic_admin;
pub mod user;

#[cfg(not(target_family = "wasm"))]
const API_TIMEOUT_SECS: u64 = 60; // 1 minute
const API_PREFIX: &str = "/api";
const LOGS_LIMIT: u32 = 100;
const USERS_LIMIT: u32 = 100;

#[allow(unused_variables)]
pub(super) fn make_get_request(
    endpoint: &str,
    uri: &str,
    auth_token: Option<String>,
    timeout: Option<Duration>,
) -> RequestBuilder {
    let url = format!("{}{}", endpoint.trim_end_matches("/"), uri);
    #[cfg(not(target_family = "wasm"))]
    let req = ClientBuilder::new()
        .user_agent(USER_AGENT)
        .timeout(timeout.unwrap_or(Duration::from_secs(API_TIMEOUT_SECS)));

    #[cfg(target_family = "wasm")]
    let req = ClientBuilder::new();

    let req = req.build().unwrap().get(&url).header(
        CONTENT_TYPE,
        HeaderValue::from_bytes(b"application/json").unwrap(),
    );

    match auth_token {
        Some(token) => req.header(AUTHORIZATION, format!("Bearer {}", token)),
        None => req,
    }
}

#[allow(unused_variables)]
pub(super) fn make_post_request(
    endpoint: &str,
    uri: &str,
    auth_token: Option<&str>,
    content_type: Option<&str>,
    body: Option<String>,
    timeout: Option<Duration>,
) -> RequestBuilder {
    let url = format!("{}{}", endpoint.trim_end_matches("/"), uri);
    #[cfg(not(target_family = "wasm"))]
    let req = ClientBuilder::new()
        .user_agent(USER_AGENT)
        .timeout(timeout.unwrap_or(Duration::from_secs(API_TIMEOUT_SECS)));

    #[cfg(target_family = "wasm")]
    let req = ClientBuilder::new();

    let req = req.build().unwrap().post(&url);

    let req = match content_type {
        Some(content_type) => req.header(CONTENT_TYPE, content_type),
        None => req,
    };

    let req = match auth_token {
        Some(token) => req.header(AUTHORIZATION, format!("Bearer {}", token)),
        None => req,
    };
    match body {
        Some(body) => req.body(body),
        None => req,
    }
}

pub(crate) async fn handle_response<T>(resp: Response) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let status = resp.status();
    let url = resp.url().to_string();

    match status {
        reqwest::StatusCode::OK => Ok(resp.json::<T>().await?),
        _ => {
            let body = resp.text().await?;
            let msg = serde_json::from_str::<serde_json::Value>(&body)
                .map(|v| {
                    let msg = v["error"].as_str().unwrap_or_default();
                    msg.to_string()
                })
                .unwrap_or(status.to_string());

            warn!("response with {} error: {}", url, msg);

            match status {
                reqwest::StatusCode::FORBIDDEN => Err(Forbidden(msg.to_string()).into()),
                reqwest::StatusCode::UNAUTHORIZED => Err(InvalidPassword(msg.to_string()).into()),
                reqwest::StatusCode::BAD_REQUEST => Err(HTTP(msg.to_string()).into()),
                _ => Err(HTTP(msg.to_string()).into()),
            }
        }
    }
}

pub(super) async fn api_call<R>(
    endpoint: &str,
    uri: &str,
    auth_token: &str,
    body: Option<String>,
) -> Result<R>
where
    R: serde::de::DeserializeOwned,
{
    let st = now_millis();
    let req = make_post_request(
        endpoint,
        &format!("{}{}", API_PREFIX, uri),
        Some(auth_token),
        Some("application/json"),
        body,
        None,
    );

    let resp = req.send().await.map_err(|e| HTTP(e.to_string()))?;
    let status = resp.status();

    debug!(
        "api url:{} status:{} usage: {:?}",
        resp.url().to_string(),
        status,
        elapsed(st)
    );
    handle_response::<R>(resp).await
}
