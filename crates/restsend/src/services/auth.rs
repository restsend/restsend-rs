use crate::error::ClientError::{self, Forbidden, HTTP};
use crate::models::conversation::Extra;
use crate::models::AuthInfo;
use crate::services::{handle_response, make_get_request, make_post_request, response};
use crate::utils::{elapsed, now_millis};
use crate::Result;
use log::info;
use restsend_macros::export_wasm_or_ffi;

#[export_wasm_or_ffi]
pub async fn login_with_token(endpoint: String, email: String, token: String) -> Result<AuthInfo> {
    let data = serde_json::json!({
        "token": token,
        "remember": true,
    });
    signin_or_signup(&endpoint, "/auth/login", &email, data.to_string()).await
}

#[export_wasm_or_ffi]
pub async fn login_with_password(
    endpoint: String,
    email: String,
    password: String,
) -> Result<AuthInfo> {
    let data = serde_json::json!({
        "email": email,
        "password": password,
        "remember": true,
    });
    signin_or_signup(&endpoint, "/auth/login", &email, data.to_string()).await
}

#[export_wasm_or_ffi]
pub async fn signup(endpoint: String, email: String, password: String) -> Result<AuthInfo> {
    let data = serde_json::json!({
        "email": email,
        "password": password,
        "remember": true,
    });
    signin_or_signup(&endpoint, "/auth/register", &email, data.to_string()).await
}

#[export_wasm_or_ffi]
pub async fn logout(endpoint: String, token: String) -> Result<()> {
    let st = now_millis();
    let uri = "/auth/logout";
    let req = make_get_request(&endpoint, uri, Some(token.to_string()), None);
    let resp = req.send().await.map_err(|e| HTTP(e.to_string()))?;
    let status = resp.status();

    info!(
        "logout url:{}{} status:{} usage: {:?}",
        endpoint,
        uri,
        status,
        elapsed(st)
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

async fn signin_or_signup(
    endpoint: &str,
    uri: &str,
    email: &str,
    body: String,
) -> Result<AuthInfo> {
    let st = now_millis();
    let req = make_post_request(
        endpoint,
        uri,
        None,
        Some("application/json"),
        Some(body),
        None,
    );
    let resp = req.send().await.map_err(|e| HTTP(e.to_string()))?;
    let status = resp.status();

    info!(
        "auth url:{} email:{} status:{} usage: {:?}",
        resp.url().to_string(),
        email,
        status,
        elapsed(st)
    );

    let r = handle_response::<response::Login>(resp).await?;
    Ok(AuthInfo {
        endpoint: endpoint.to_string(),
        user_id: r.email,
        avatar: r.profile.avatar,
        name: r.display_name,
        token: r.token,
        is_staff: r.is_staff,
        is_cross_domain: false,
        private_extra: r.profile.private_extra,
    })
}

#[export_wasm_or_ffi]
pub async fn guest_login(
    endpoint: String,
    guest_id: String,
    extra: Option<Extra>,
) -> Result<AuthInfo> {
    let mut data = serde_json::json!({
        "guestId": guest_id,
        "remember": true,
    });
    if let Some(extra) = extra {
        data["extra"] = serde_json::to_value(extra)
            .map_err(|_| ClientError::Other("invalid extra type".to_string()))?;
    }
    signin_or_signup(&endpoint, "/api/guest/login", &guest_id, data.to_string()).await
}

#[cfg(not(target_family = "wasm"))]
#[tokio::test]
async fn test_login() {
    let user_id = "alice";
    let info = login_with_password(
        super::tests::TEST_ENDPOINT.to_string(),
        user_id.to_string(),
        "bad:demo2".to_string(),
    )
    .await;
    println!("{:?}", info);
    assert!(info.unwrap_err().to_string().contains("invalid password"));

    let info = login_with_password(
        super::tests::TEST_ENDPOINT.to_string(),
        user_id.to_string(),
        format!("{}:demo", user_id),
    )
    .await;
    assert!(info.is_ok());

    let info = info.unwrap();
    assert_eq!(info.user_id, user_id);
    assert!(!info.avatar.is_empty());
    assert!(!info.token.is_empty());
    assert_eq!(info.endpoint, super::tests::TEST_ENDPOINT);

    let token = info.token;
    let info = login_with_token(
        super::tests::TEST_ENDPOINT.to_string(),
        user_id.to_string(),
        token.clone(),
    )
    .await;

    assert!(info.is_ok());
    let info = info.unwrap();
    assert_eq!(info.user_id, user_id);
    assert!(!info.avatar.is_empty());
    assert!(!info.token.is_empty());

    assert_eq!(info.token, token);
}

#[cfg(not(target_family = "wasm"))]
#[tokio::test]
async fn test_guest_login() {
    let user_id = "alice";
    let info = guest_login(
        super::tests::TEST_ENDPOINT.to_string(),
        user_id.to_string(),
        None,
    )
    .await;
    println!("{:?}", info);
    assert!(info.is_ok());
}

#[cfg(not(target_family = "wasm"))]
#[tokio::test]
async fn test_login_logout() {
    let info = login_with_password(
        super::tests::TEST_ENDPOINT.to_string(),
        "alice".to_string(),
        "alice:demo".to_string(),
    )
    .await;
    assert!(info.is_ok());

    logout(super::tests::TEST_ENDPOINT.to_string(), info.unwrap().token)
        .await
        .expect("logout failed");
}
