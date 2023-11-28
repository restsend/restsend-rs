use std::time::Duration;

use crate::error::ClientError::{Forbidden, HTTPError, InvalidPassword};
use crate::models::AuthInfo;
use anyhow::Result;
use log::{info, warn};

pub async fn login_with_token(endpoint: &str, email: &str, token: &str) -> Result<AuthInfo> {
    let data = serde_json::json!({
        "token": token,
        "remember": true,
    });
    login(endpoint, email, data.to_string()).await
}

pub async fn login_with_password(endpoint: &str, email: &str, password: &str) -> Result<AuthInfo> {
    let data = serde_json::json!({
        "email": email,
        "password": password,
        "remember": true,
    });
    login(endpoint, email, data.to_string()).await
}

pub async fn logout(endpoint: &str, token: &str) -> Result<()> {
    let st = std::time::Instant::now();
    let url = format!("{}/auth/logout", endpoint);
    let req = super::make_get_request(endpoint, "/auth/logout", Some(token.to_string()), None);
    let resp = req.send().await.map_err(|e| HTTPError(e.to_string()))?;
    let status = resp.status();

    info!(
        "logout url:{} status:{} usage: {:?}",
        url,
        status,
        st.elapsed()
    );

    match status {
        reqwest::StatusCode::OK => Ok(()),
        _ => {
            let body = resp.text().await?;
            let msg = serde_json::from_str::<serde_json::Value>(&body)
                .map(|v| {
                    let msg = v["error"].as_str().unwrap_or_default();
                    msg.to_string()
                })
                .unwrap_or_else(|_| body);
            Err(Forbidden(msg).into())
        }
    }
}

async fn login(endpoint: &str, email: &str, body: String) -> Result<AuthInfo> {
    let st = std::time::Instant::now();
    let url = format!("{}/auth/login", endpoint);
    let req = super::make_post_request(
        endpoint,
        "/auth/login",
        None,
        Some("application/json"),
        Some(body),
        None,
    );
    let resp = req.send().await.map_err(|e| HTTPError(e.to_string()))?;
    let status = resp.status();

    info!(
        "login url:{} email:{} status:{} usage: {:?}",
        url,
        email,
        status,
        st.elapsed()
    );

    let r = super::handle_response::<super::response::Login>(resp).await?;
    Ok(AuthInfo {
        endpoint: endpoint.to_string(),
        user_id: r.email,
        avatar: r.profile.avatar,
        name: r.display_name,
        token: r.token,
    })
}

#[tokio::test]
async fn test_login() {
    let info = login_with_password(super::tests::TEST_ENDPOINT, "bob", "bob:demo2").await;
    println!("{:?}", info);
    assert!(info.unwrap_err().to_string().contains("invalid password"));

    let info = login_with_password(super::tests::TEST_ENDPOINT, "bob", "bob:demo").await;
    assert!(info.is_ok());

    let info = info.unwrap();
    assert_eq!(info.user_id, "bob");
    assert!(!info.avatar.is_empty());
    assert!(!info.token.is_empty());
    assert_eq!(info.endpoint, super::tests::TEST_ENDPOINT);

    let token = info.token;
    let info = login_with_token(super::tests::TEST_ENDPOINT, "bob", &token).await;

    assert!(info.is_ok());
    let info = info.unwrap();
    assert_eq!(info.user_id, "bob");
    assert!(!info.avatar.is_empty());
    assert!(!info.token.is_empty());

    assert_eq!(info.token, token);
}

#[tokio::test]
async fn test_login_logout() {
    let info = login_with_password(super::tests::TEST_ENDPOINT, "bob", "bob:demo").await;
    assert!(info.is_ok());

    logout(super::tests::TEST_ENDPOINT, info.unwrap().token.as_str())
        .await
        .expect("logout failed");
}
