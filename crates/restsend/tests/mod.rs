use crate::{client::Client, login};
use reqwest::header::HeaderValue;
use tokio::time::{Duration, Instant};

mod http_server;
mod test_chat;
mod test_clients;
mod test_media;
mod test_net;
mod test_service;
mod test_websocket;

const TEST_SERVER: &str = "https://chat.ruzhila.cn";

pub(crate) fn login_with(user_id: &str, password: &str) -> crate::Client {
    let c = Client::new(
        crate::models::MEMORY_DSN.to_string(),
        TEST_SERVER.to_string(),
    );
    c.prepare().expect("prepare failed");
    let info = login(
        TEST_SERVER.to_string(),
        user_id.to_string(),
        password.to_string(),
    )
    .expect("login failed");
    c.attach(info).expect("login failed");
    c.set_allow_guest_chat(true)
        .expect("set_allow_guest_chat failed");
    c
}
pub(crate) fn signup_demo_user(user_id: &str) -> crate::Result<()> {
    let data = serde_json::json!({
        "email": user_id,
        "password": format!("{}:demo", user_id),
    });
    let url = format!("{}/auth/register", TEST_SERVER);
    let req = reqwest::ClientBuilder::new()
        .user_agent(crate::USER_AGENT)
        .build()?
        .post(&url)
        .header(
            reqwest::header::CONTENT_TYPE,
            HeaderValue::from_bytes(b"application/json").unwrap(),
        )
        .body(data.to_string());

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let resp = req.send().await?;
            match resp.status() {
                reqwest::StatusCode::OK => Ok(()),
                _ => {
                    let err = resp.text().await?;
                    if err.contains("email has exists") {
                        return Ok(());
                    }
                    Err(crate::ClientError::HTTPError(err))
                }
            }
        })
}

fn check_until(duration: Duration, f: impl Fn() -> bool) -> crate::Result<()> {
    let st = Instant::now();
    loop {
        if f() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(100));
        if st.elapsed() > duration {
            return Err(crate::ClientError::StdError(format!(
                "check_until timeout: {:?}",
                duration
            )));
        }
    }
}
