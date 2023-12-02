use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use crate::{
    callback,
    client::{tests::TEST_ENDPOINT, Client},
    services::auth::login_with_password,
    utils::check_until,
    utils::init_log,
};

struct TestCallbackImpl {
    is_connected: Arc<AtomicBool>,
}

impl callback::Callback for TestCallbackImpl {
    fn on_connected(&self) {
        self.is_connected.store(true, Ordering::Relaxed);
    }
}

struct TestMessageCakllbackImpl {
    is_sent: Arc<AtomicBool>,
    is_ack: Arc<AtomicBool>,
    last_error: Arc<Mutex<String>>,
}

impl callback::MessageCallback for TestMessageCakllbackImpl {
    fn on_sent(&self) {
        self.is_sent.store(true, Ordering::Relaxed);
    }
    fn on_ack(&self, _req: crate::request::ChatRequest) {
        self.is_ack.store(true, Ordering::Relaxed);
    }
    fn on_fail(&self, reason: String) {
        *self.last_error.lock().unwrap() = reason.clone();
    }
}

#[tokio::test]
async fn test_client_connected() {
    init_log("INFO", true);

    let info = login_with_password(TEST_ENDPOINT, "bob", "bob:demo").await;
    assert!(info.is_ok());

    let c = Client::new("", "", &info.unwrap());
    let is_connected = Arc::new(AtomicBool::new(false));

    let callback = Box::new(TestCallbackImpl {
        is_connected: is_connected.clone(),
    });

    c.connect(callback).await;

    check_until(Duration::from_secs(3), || {
        is_connected.load(Ordering::Relaxed)
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_client_send_message() {
    init_log("INFO", true);
    let info = login_with_password(TEST_ENDPOINT, "bob", "bob:demo").await;
    let c = Client::new("", "", &info.unwrap());
    let is_connected = Arc::new(AtomicBool::new(false));
    let callback = Box::new(TestCallbackImpl {
        is_connected: is_connected.clone(),
    });

    c.connect(callback).await;
    check_until(Duration::from_secs(3), || {
        is_connected.load(Ordering::Relaxed)
    })
    .await
    .unwrap();

    let is_sent = Arc::new(AtomicBool::new(false));
    let is_ack = Arc::new(AtomicBool::new(false));

    let msg_cb = Box::new(TestMessageCakllbackImpl {
        is_sent: is_sent.clone(),
        is_ack: is_ack.clone(),
        last_error: Arc::new(Mutex::new("".to_string())),
    });

    c.do_send_text("bob:alice", "hello", None, None, Some(msg_cb))
        .await
        .unwrap();

    check_until(Duration::from_secs(3), || is_sent.load(Ordering::Relaxed))
        .await
        .unwrap();
}
