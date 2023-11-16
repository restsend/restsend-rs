use crate::{client::Client, login};
use tokio::time::{Duration, Instant};

mod http_server;
mod test_chat;
mod test_clients;
mod test_media;
mod test_net;
mod test_service;
mod test_websocket;

const TEST_SERVER: &str = "http://chat.rddoc.cn";

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
