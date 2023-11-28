use std::time::Duration;

use crate::error::ClientError::{Forbidden, HTTPError, InvalidPassword};
use crate::models::AuthInfo;
use anyhow::Result;
use log::{info, warn};
use reqwest::header::HeaderValue;

pub async fn login_with_password(endpoint: &str, email: &str, password: &str) -> Result<AuthInfo> {
    let data = serde_json::json!({
        "email": email,
        "password": password,
        "remember": true,
    });
    let st = std::time::Instant::now();
    let url = format!("{}/auth/login", endpoint);
    let req = reqwest::ClientBuilder::new()
        .user_agent(crate::USER_AGENT)
        .build()?
        .post(&url)
        .header(
            reqwest::header::CONTENT_TYPE,
            HeaderValue::from_bytes(b"application/json").unwrap(),
        )
        .body(data.to_string())
        .timeout(Duration::from_secs(super::API_TIMEOUT_SECS));

    let resp = req.send().await.map_err(|e| HTTPError(e.to_string()))?;
    let status = resp.status();

    info!(
        "login url:{} email:{} status:{} usage: {:?}",
        url,
        email,
        status,
        st.elapsed()
    );

    match status {
        reqwest::StatusCode::OK => {
            let resp: super::response::Login = resp.json().await?;
            Ok(AuthInfo {
                endpoint: endpoint.to_string(),
                user_id: resp.email,
                avatar: resp.profile.avatar,
                name: resp.display_name,
                token: resp.token,
            })
        }
        _ => {
            let body = resp.text().await?;
            let msg = serde_json::from_str::<serde_json::Value>(&body)
                .map(|v| {
                    let msg = v["error"].as_str().unwrap_or_default();
                    msg.to_string()
                })
                .unwrap_or(status.to_string());

            warn!("login with {} error: {}", email, msg);

            match status {
                reqwest::StatusCode::FORBIDDEN => Err(Forbidden(msg.to_string()).into()),
                reqwest::StatusCode::UNAUTHORIZED => Err(InvalidPassword(msg.to_string()).into()),
                reqwest::StatusCode::BAD_REQUEST => Err(HTTPError(msg.to_string()).into()),
                _ => Err(HTTPError(msg.to_string()).into()),
            }
        }
    }
}

#[tokio::test]
async fn test_login_with_password() {
    let info = login_with_password(super::tests::TEST_ENDPOINT, "bob", "bob:demo").await;
    assert!(info.is_ok());
    let info = login_with_password(super::tests::TEST_ENDPOINT, "bob", "bob:demo2").await;
    println!("{:?}", info);
    assert!(info.unwrap_err().to_string().contains("invalid password"));
}
