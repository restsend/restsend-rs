#![cfg(not(target_arch = "wasm32"))]

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Duration;

use crate::{
    callback::{self, ChatRequestStatus},
    client::Client,
    models::Conversation,
    request::ChatRequest,
    services::auth::{login_with_password, signup},
    utils::{check_until, init_log},
};
use restsend_backend::app::{build_router, AppConfig};
use tokio::net::TcpListener;

const DEMO_PASS: &str = "demo";
const WS_TIMEOUT: Duration = Duration::from_secs(8);

struct DemoServer {
    endpoint: String,
    server: tokio::task::JoinHandle<()>,
}

impl DemoServer {
    async fn start() -> Self {
        let config = AppConfig {
            addr: "127.0.0.1:0".to_string(),
            endpoint: "127.0.0.1:0".to_string(),
            database_url: format!(
                "sqlite:file:restsend-demo-dm-{}?mode=memory&cache=shared",
                crate::utils::random_text(8)
            ),
            openapi_schema: "http".to_string(),
            openapi_prefix: "/open".to_string(),
            api_prefix: "/api".to_string(),
            log_file: format!("logs/demo-dm-{}.log", crate::utils::random_text(8)),
            openapi_token: Some("test-token".to_string()),
            run_migrations: true,
            migrate_only: false,
            webhook_timeout_secs: 5,
            webhook_retries: 2,
            webhook_targets: vec![],
            event_bus_size: 256,
            message_worker_count: 2,
            message_queue_size: 64,
            push_worker_count: 2,
            push_queue_size: 64,
            webhook_worker_count: 2,
            webhook_queue_size: 64,
            max_upload_bytes: 10 * 1024 * 1024,
            presence_backend: "memory".to_string(),
            presence_node_id: "demo-dm-node".to_string(),
            presence_ttl_secs: 90,
            presence_heartbeat_secs: 10,
            ws_per_user_limit: 0,
            ws_client_queue_size: 0,
            ws_typing_interval_ms: 1000,
            ws_drop_on_backpressure: true,
        };

        let (app, state) = build_router(config).await.expect("build router");
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let endpoint = format!("http://{}", addr);
        let server = tokio::spawn(async move {
            axum::serve(listener, app.with_state(state)).await.unwrap();
        });

        tokio::time::sleep(Duration::from_millis(200)).await;
        Self { endpoint, server }
    }
}

impl Drop for DemoServer {
    fn drop(&mut self) {
        self.server.abort();
    }
}

struct DemoWsCallback {
    connected: Arc<AtomicBool>,
    received_topic_id: Arc<Mutex<Vec<String>>>,
    conv_unreads: Arc<Mutex<Vec<(String, i64)>>>,
}

impl callback::RsCallback for DemoWsCallback {
    fn on_connected(&self) {
        self.connected.store(true, Ordering::Relaxed);
    }
    fn on_new_message(&self, topic_id: String, _message: ChatRequest) -> ChatRequestStatus {
        self.received_topic_id.lock().unwrap().push(topic_id);
        ChatRequestStatus::default()
    }
    fn on_conversations_updated(&self, conversations: Vec<Conversation>, _total: Option<i64>) {
        for conv in &conversations {
            self.conv_unreads
                .lock()
                .unwrap()
                .push((conv.topic_id.clone(), conv.unread));
        }
    }
}

#[tokio::test]
async fn test_demo_dm_alice_bob_websocket_delivery() {
    init_log("INFO".to_string(), true);
    let server = DemoServer::start().await;
    let endpoint = server.endpoint.clone();

    // signup alice and bob
    signup(endpoint.clone(), "alice".to_string(), DEMO_PASS.to_string())
        .await
        .expect("signup alice");
    signup(endpoint.clone(), "bob".to_string(), DEMO_PASS.to_string())
        .await
        .expect("signup bob");

    // login
    let info_alice =
        login_with_password(endpoint.clone(), "alice".to_string(), DEMO_PASS.to_string())
            .await
            .expect("login alice");
    let info_bob = login_with_password(endpoint.clone(), "bob".to_string(), DEMO_PASS.to_string())
        .await
        .expect("login bob");

    // create clients
    let client_alice = Client::new("".to_string(), "".to_string(), &info_alice);
    let client_bob = Client::new("".to_string(), "".to_string(), &info_bob);

    let alice_connected = Arc::new(AtomicBool::new(false));
    let bob_connected = Arc::new(AtomicBool::new(false));
    let alice_received_topic_id: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let bob_received_topic_id: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    let alice_conv_unreads: Arc<Mutex<Vec<(String, i64)>>> = Arc::new(Mutex::new(Vec::new()));
    let bob_conv_unreads: Arc<Mutex<Vec<(String, i64)>>> = Arc::new(Mutex::new(Vec::new()));

    client_alice.set_callback(Some(Box::new(DemoWsCallback {
        connected: alice_connected.clone(),
        received_topic_id: alice_received_topic_id.clone(),
        conv_unreads: alice_conv_unreads.clone(),
    })));
    client_bob.set_callback(Some(Box::new(DemoWsCallback {
        connected: bob_connected.clone(),
        received_topic_id: bob_received_topic_id.clone(),
        conv_unreads: bob_conv_unreads.clone(),
    })));

    // connect WebSocket
    client_alice.connect().await;
    client_bob.connect().await;

    check_until(WS_TIMEOUT, || alice_connected.load(Ordering::Relaxed))
        .await
        .expect("alice ws connected");
    check_until(WS_TIMEOUT, || bob_connected.load(Ordering::Relaxed))
        .await
        .expect("bob ws connected");

    // === 1. Alice creates DM chat ===
    let conv = client_alice
        .create_chat("bob".to_string())
        .await
        .expect("alice creates chat");

    assert_eq!(
        conv.topic_id, "alice:bob",
        "DM topic ID should be 'alice:bob'"
    );

    // Wait for topic creation and propagation
    tokio::time::sleep(Duration::from_millis(500)).await;

    // === 2. Alice sends message via WebSocket ===
    struct AckCallback {
        acked: Arc<AtomicBool>,
    }
    impl callback::MessageCallback for AckCallback {
        fn on_ack(&self, _req: ChatRequest) {
            self.acked.store(true, Ordering::Relaxed);
        }
    }

    let acked = Arc::new(AtomicBool::new(false));
    client_alice
        .do_send_text(
            conv.topic_id.clone(),
            "Hello via WS!".to_string(),
            None,
            None,
            Some(Box::new(AckCallback {
                acked: acked.clone(),
            })),
        )
        .await
        .expect("alice sends via ws");

    // Alice should receive ack (server response)
    check_until(WS_TIMEOUT, || acked.load(Ordering::Relaxed))
        .await
        .expect("alice message acked");

    // Bob should receive the message via WebSocket
    check_until(WS_TIMEOUT, || {
        bob_received_topic_id
            .lock()
            .unwrap()
            .contains(&"alice:bob".to_string())
    })
    .await
    .expect("bob received msg via ws");

    // Alice should also get an echo (her own message echoed back)
    check_until(WS_TIMEOUT, || {
        alice_received_topic_id
            .lock()
            .unwrap()
            .contains(&"alice:bob".to_string())
    })
    .await
    .expect("alice received echo via ws");

    log::info!("=== WebSocket delivery confirmed: alice → bob ===");

    // Verify Bob's unread updated (hasRead=false since bob isn't viewing alice's conv)
    let bob_unreads = bob_conv_unreads.lock().unwrap().clone();
    let bob_topic_unreads: Vec<i64> = bob_unreads
        .iter()
        .filter(|(t, _)| t == "alice:bob")
        .map(|(_, u)| *u)
        .collect();
    assert!(
        bob_topic_unreads.iter().any(|u| *u >= 1),
        "Bob's conversation should have unread >= 1, got: {:?}",
        bob_topic_unreads
    );

    // === 3. Send second message to verify unread increments ===
    let req2 = ChatRequest::new_text(&conv.topic_id, "Second message");
    client_alice
        .send_chat_request(conv.topic_id.clone(), req2)
        .await
        .expect("alice sends second msg");

    // Wait for Bob to receive second message
    let bob_second_received = check_until(Duration::from_secs(3), || {
        let count = bob_received_topic_id
            .lock()
            .unwrap()
            .iter()
            .filter(|t| *t == "alice:bob")
            .count();
        count >= 2
    })
    .await;
    assert!(
        bob_second_received.is_ok(),
        "bob should receive second msg via ws"
    );

    // Bob's unread should now be >= 2 (hasRead=false)
    tokio::time::sleep(Duration::from_millis(500)).await;
    let bob_unreads_2 = bob_conv_unreads.lock().unwrap().clone();
    let bob_topic_unreads_2: Vec<i64> = bob_unreads_2
        .iter()
        .filter(|(t, _)| t == "alice:bob")
        .map(|(_, u)| *u)
        .collect();
    let max_unread = bob_topic_unreads_2.iter().max().copied().unwrap_or(0);
    assert!(
        max_unread >= 2,
        "Bob's conversation should have unread >= 2 after 2 messages, got max={} all={:?}",
        max_unread,
        bob_topic_unreads_2
    );
    log::info!(
        "=== Unread count verified: bob's unread = {} ===",
        max_unread
    );

    // === 4. Bob sends reply via WebSocket ===
    let bob_acked = Arc::new(AtomicBool::new(false));
    client_bob
        .do_send_text(
            conv.topic_id.clone(),
            "Reply via WS!".to_string(),
            None,
            None,
            Some(Box::new(AckCallback {
                acked: bob_acked.clone(),
            })),
        )
        .await
        .expect("bob sends via ws");

    check_until(WS_TIMEOUT, || bob_acked.load(Ordering::Relaxed))
        .await
        .expect("bob message acked");

    // Verify total count: alice should have received 2 messages (echo + bob's reply)
    let alice_msg_count = alice_received_topic_id
        .lock()
        .unwrap()
        .iter()
        .filter(|t| *t == "alice:bob")
        .count();
    assert!(
        alice_msg_count >= 2,
        "alice should have received at least 2 messages (echo + bob's reply), got {alice_msg_count}"
    );

    log::info!("=== WebSocket delivery confirmed: bob → alice ===");

    // === 5. Simulate "read" — Bob marks conversation as read (like opening chat view) ===
    // This covers the exact user scenario: open conversation → read → close → new msg
    client_bob
        .set_conversation_read(conv.topic_id.clone(), false)
        .await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    let bob_conv_after_read = client_bob
        .get_conversation(conv.topic_id.clone())
        .await
        .unwrap();
    log::info!(
        "=== After set_conversation_read: unread={}, last_read_seq={}, last_seq={} ===",
        bob_conv_after_read.unread,
        bob_conv_after_read.last_read_seq,
        bob_conv_after_read.last_seq
    );

    // At this point Bob's local unread should be 0 (he read the conversation)
    // The server might have reset it; get_conversation fetches latest from server
    // We check via sync instead for accurate server-side state

    // Now Alice sends a THIRD message while Bob is NOT viewing (hasRead=false)
    let req3 = ChatRequest::new_text(&conv.topic_id, "Third msg after read");
    client_alice
        .send_chat_request(conv.topic_id.clone(), req3)
        .await
        .expect("alice sends third msg");

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Bob syncs conversations to get the latest unread from server
    struct SyncDone {
        done: std::sync::atomic::AtomicBool,
    }
    impl crate::callback::SyncConversationsCallback for SyncDone {
        fn on_success(&self, _: String, _: Option<String>, _: u32, _: u32) {
            self.done.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        fn on_fail(&self, _: crate::Error) {
            self.done.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }
    client_bob
        .sync_conversations(
            None,
            None,
            None,
            None,
            0,
            false,
            None,
            None,
            None,
            Box::new(SyncDone {
                done: std::sync::atomic::AtomicBool::new(false),
            }),
        )
        .await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    let bob_conv_after_third = client_bob
        .get_conversation(conv.topic_id.clone())
        .await
        .expect("bob conversation exists");

    log::info!(
        "=== After third msg: last_seq={}, unread={}, last_sender_id={} ===",
        bob_conv_after_third.last_seq,
        bob_conv_after_third.unread,
        bob_conv_after_third.last_sender_id
    );

    // After read (last_seq=3), new message should be seq=4 with unread=1
    assert_eq!(
        bob_conv_after_third.last_seq, 4,
        "Bob's last_seq should be 4 after third message"
    );
    assert_eq!(
        bob_conv_after_third.unread, 1,
        "Bob should have 1 unread after read+new msg, got: {}",
        bob_conv_after_third.unread
    );
    assert_eq!(
        bob_conv_after_third.last_sender_id, "alice",
        "last_sender_id should be alice"
    );
    log::info!("=== DM WebSocket test passed ===");
}
