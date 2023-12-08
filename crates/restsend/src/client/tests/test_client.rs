use crate::{
    callback,
    client::{tests::TEST_ENDPOINT, Client},
    models::{ChatLogStatus, Conversation},
    request::ChatRequest,
    services::auth::login_with_password,
    utils::check_until,
    utils::init_log,
};
use log::warn;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

struct TestCallbackImpl {
    last_topic_id: Arc<Mutex<String>>,
    is_connected: Arc<AtomicBool>,
    is_recv_message: Arc<AtomicBool>,
    is_update_conversation: Arc<AtomicBool>,
}

impl callback::Callback for TestCallbackImpl {
    fn on_connected(&self) {
        self.is_connected.store(true, Ordering::Relaxed);
    }
    // if return true, will send `has read` to server
    fn on_new_message(&self, topic_id: String, message: ChatRequest) -> bool {
        warn!(
            "on_topic_message topic_id:{} message: {:?}",
            topic_id, message
        );
        self.is_recv_message.store(true, Ordering::Relaxed);
        return false;
    }
    fn on_topic_read(&self, topic_id: String, message: ChatRequest) {
        warn!("on_topic_read: topic_id:{} message:{:?}", topic_id, message);
    }
    fn on_conversations_updated(&self, conversations: Vec<Conversation>) {
        warn!("on_conversation_updated: {:?}", conversations);
        *self.last_topic_id.lock().unwrap() = conversations[0].topic_id.clone();
        self.is_update_conversation.store(true, Ordering::Relaxed);
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
        last_topic_id: Arc::new(Mutex::new("".to_string())),
        is_connected: is_connected.clone(),
        is_recv_message: Arc::new(AtomicBool::new(false)),
        is_update_conversation: Arc::new(AtomicBool::new(false)),
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
    let info = login_with_password(TEST_ENDPOINT, "guido", "guido:demo").await;
    let c = Client::new("", "", &info.unwrap());

    let is_connected = Arc::new(AtomicBool::new(false));
    let is_recv_message = Arc::new(AtomicBool::new(false));
    let is_update_conversation = Arc::new(AtomicBool::new(false));
    let last_topic_id = Arc::new(Mutex::new("".to_string()));

    let callback = Box::new(TestCallbackImpl {
        last_topic_id: last_topic_id.clone(),
        is_connected: is_connected.clone(),
        is_recv_message: is_recv_message.clone(),
        is_update_conversation: is_update_conversation.clone(),
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

    c.do_send_text("guido:alice", "hello", None, None, Some(msg_cb))
        .await
        .unwrap();

    check_until(Duration::from_secs(3), || is_sent.load(Ordering::Relaxed))
        .await
        .unwrap();

    check_until(Duration::from_secs(3), || {
        is_recv_message.load(Ordering::Relaxed) && is_update_conversation.load(Ordering::Relaxed)
    })
    .await
    .unwrap();

    // check local storage
    let topic_id = last_topic_id.lock().unwrap().clone();

    let logs = c.store.get_chat_logs(&topic_id, 0, 10).unwrap();

    assert!(logs.items.len() == 1);
    assert_eq!(logs.items[0].sender_id, "guido");
    assert_eq!(logs.items[0].status, ChatLogStatus::Sent);
}
