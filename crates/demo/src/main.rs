use std::sync::Arc;

use restsend_sdk::{callback, client::Client, services::auth::login_with_password};
use tokio::sync::Notify;
struct TestCallbackImpl {
    is_connected: Arc<Notify>,
}

impl callback::Callback for TestCallbackImpl {
    fn on_connecting(&self) {
        println!("on_connecting");
    }

    fn on_connected(&self) {
        println!("on_connected");
        self.is_connected.notify_one();
    }
    fn on_net_broken(&self, reason: String) {
        println!("on_net_broken: {}", reason);
        self.is_connected.notify_one();
    }
}

#[tokio::main]
async fn main() {
    let info = login_with_password(
        "https://chat.ruzhila.cn".to_string(),
        "bob".to_string(),
        "bob:demo".to_string(),
    )
    .await;
    assert!(info.is_ok());

    let c = Client::new("".to_string(), "".to_string(), &info.unwrap());
    let is_connected = Arc::new(Notify::new());

    let callback = Box::new(TestCallbackImpl {
        is_connected: is_connected.clone(),
    });

    c.set_callback(Some(callback));
    c.connect().await;
    is_connected.notified().await;
}
