use log;
use super::Result;
use restsend_sdk::models::AuthInfo;
/// Signin with userId and password or token
pub async fn signin(
    endpoint: String,
    user_id: String,
    password: Option<String>,
    token: Option<String>,
) -> Result<AuthInfo> {
    match password {
        Some(password) => {
            let info = restsend_sdk::services::auth::login_with_password(endpoint, user_id, password).await;
            if info.is_err() {
                log::info!("Login with password failed, try with token");
                Err(restsend_sdk::Error::HTTP(info.err().unwrap().to_string()))
            }
            else {
                info
            }            
        }
        None => {
            restsend_sdk::services::auth::login_with_token(
                endpoint,
                user_id,
                token.unwrap_or_default(),
            )
            .await
        }
    }
}

pub async fn hello() -> Result<String> {
    let url = "http://192.168.3.152:8000/v1";
    let data = reqwest::get(url).await?.text().await?;
    Ok(data)
}
/// Signup with userId and password
pub async fn signup(
    endpoint: String,
    user_id: String,
    password: String,
) -> Result<AuthInfo> {
    restsend_sdk::services::auth::signup(endpoint, user_id, password)
        .await
}

/// Logout with token
pub async fn logout(endpoint: String, token: String) -> Result<()> {
   restsend_sdk::services::auth::logout(endpoint, token)
        .await
}
