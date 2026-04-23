#![cfg(not(target_arch = "wasm32"))]

use crate::{
    callback::{self, ChatRequestStatus},
    client::Client,
    models::GetChatLogsResult,
    request::ChatRequest,
    services::conversation::get_chat_logs_desc,
    services::auth::{login_with_password, signup},
    utils::{check_until, init_log},
};
use restsend_backend::app::{build_router, AppConfig};
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, AtomicI64, AtomicU32, Ordering},
    Arc, Mutex,
};
use std::time::Duration;
use tokio::net::TcpListener;

fn unique_name(prefix: &str) -> String {
    format!("{}-{}", prefix, crate::utils::random_text(8))
}

struct LocalCallback {
    connected: Arc<AtomicBool>,
    received: Arc<AtomicBool>,
    typing_received: Arc<AtomicBool>,
    read_received: Arc<AtomicBool>,
    last_read_seq: Arc<AtomicI64>,
}

impl callback::RsCallback for LocalCallback {
    fn on_connected(&self) {
        self.connected.store(true, Ordering::Relaxed);
    }

    fn on_new_message(&self, _topic_id: String, _message: ChatRequest) -> ChatRequestStatus {
        self.received.store(true, Ordering::Relaxed);
        ChatRequestStatus::default()
    }

    fn on_topic_typing(&self, _topic_id: String, _message: Option<String>) {
        self.typing_received.store(true, Ordering::Relaxed);
    }

    fn on_topic_read(&self, _topic_id: String, message: ChatRequest) {
        self.read_received.store(true, Ordering::Relaxed);
        self.last_read_seq.store(message.seq, Ordering::Relaxed);
    }
}

struct LocalMessageCallback {
    acked: Arc<AtomicBool>,
}

impl callback::MessageCallback for LocalMessageCallback {
    fn on_ack(&self, _req: ChatRequest) {
        self.acked.store(true, Ordering::Relaxed);
    }
}

struct LocalSyncLogsCallback {
    result: Arc<Mutex<Option<GetChatLogsResult>>>,
}

impl callback::SyncChatLogsCallback for LocalSyncLogsCallback {
    fn on_success(&self, r: GetChatLogsResult) {
        self.result.lock().unwrap().replace(r);
    }

    fn on_fail(&self, reason: crate::Error) {
        panic!("sync logs failed: {:?}", reason);
    }
}

struct LocalSyncConversationsCallback {
    done: Arc<AtomicBool>,
    count: Arc<AtomicU32>,
}

impl callback::SyncConversationsCallback for LocalSyncConversationsCallback {
    fn on_success(&self, _updated_at: String, _last_removed_at: Option<String>, count: u32, _total: u32) {
        self.count.store(count, Ordering::Relaxed);
        self.done.store(true, Ordering::Relaxed);
    }

    fn on_fail(&self, reason: crate::Error) {
        panic!("sync conversations failed: {:?}", reason);
    }
}

struct CountingCallback {
    connected: Arc<AtomicBool>,
    received_count: Arc<AtomicU32>,
}

impl callback::RsCallback for CountingCallback {
    fn on_connected(&self) {
        self.connected.store(true, Ordering::Relaxed);
    }

    fn on_new_message(&self, _topic_id: String, _message: ChatRequest) -> ChatRequestStatus {
        self.received_count.fetch_add(1, Ordering::Relaxed);
        ChatRequestStatus::default()
    }
}

struct LocalTestServer {
    endpoint: String,
    server: tokio::task::JoinHandle<()>,
}

impl LocalTestServer {
    async fn start() -> Self {
        let config = AppConfig {
            addr: "127.0.0.1:0".to_string(),
            endpoint: "127.0.0.1:0".to_string(),
            database_url: format!(
                "sqlite:file:restsend-sdk-e2e-{}?mode=memory&cache=shared",
                crate::utils::random_text(8)
            ),
            openapi_schema: "http".to_string(),
            openapi_prefix: "/open".to_string(),
            api_prefix: "/api".to_string(),
            log_file: format!("logs/sdk-e2e-{}.log", crate::utils::random_text(8)),
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
            presence_node_id: "sdk-e2e-node".to_string(),
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

        Self { endpoint, server }
    }
}

impl Drop for LocalTestServer {
    fn drop(&mut self) {
        self.server.abort();
    }
}

#[tokio::test]
async fn test_sdk_local_backend_e2e_minimal_flow() {
    init_log("INFO".to_string(), true);
    let server = LocalTestServer::start().await;
    let endpoint = server.endpoint.clone();

    let user_a = unique_name("sdk-e2e-a");
    let user_b = unique_name("sdk-e2e-b");

    signup(endpoint.clone(), user_a.clone(), "pass-a".to_string())
        .await
        .expect("signup a");
    signup(endpoint.clone(), user_b.clone(), "pass-b".to_string())
        .await
        .expect("signup b");

    let info_a = login_with_password(endpoint.clone(), user_a.clone(), "pass-a".to_string())
        .await
        .expect("login a");
    let info_b = login_with_password(endpoint.clone(), user_b.clone(), "pass-b".to_string())
        .await
        .expect("login b");

    let client_a = Client::new("".to_string(), "".to_string(), &info_a);
    let client_b = Client::new("".to_string(), "".to_string(), &info_b);

    let connected_a = Arc::new(AtomicBool::new(false));
    let connected_b = Arc::new(AtomicBool::new(false));
    let received_b = Arc::new(AtomicBool::new(false));
    let acked_a = Arc::new(AtomicBool::new(false));

    client_a.set_callback(Some(Box::new(LocalCallback {
        connected: connected_a.clone(),
        received: Arc::new(AtomicBool::new(false)),
        typing_received: Arc::new(AtomicBool::new(false)),
        read_received: Arc::new(AtomicBool::new(false)),
        last_read_seq: Arc::new(AtomicI64::new(0)),
    })));
    client_b.set_callback(Some(Box::new(LocalCallback {
        connected: connected_b.clone(),
        received: received_b.clone(),
        typing_received: Arc::new(AtomicBool::new(false)),
        read_received: Arc::new(AtomicBool::new(false)),
        last_read_seq: Arc::new(AtomicI64::new(0)),
    })));

    client_a.connect().await;
    client_b.connect().await;

    check_until(Duration::from_secs(3), || connected_a.load(Ordering::Relaxed)).await.unwrap();
    check_until(Duration::from_secs(3), || connected_b.load(Ordering::Relaxed)).await.unwrap();

    let conversation = client_a
        .create_chat(user_b)
        .await
        .expect("create chat");

    client_a
        .do_send_text(
            conversation.topic_id,
            "hello local e2e".to_string(),
            None,
            None,
            Some(Box::new(LocalMessageCallback {
                acked: acked_a.clone(),
            })),
        )
        .await
        .expect("send text");

    check_until(Duration::from_secs(5), || acked_a.load(Ordering::Relaxed)).await.unwrap();
    check_until(Duration::from_secs(5), || received_b.load(Ordering::Relaxed)).await.unwrap();
}

#[tokio::test]
async fn test_sdk_local_backend_e2e_sync_logs() {
    init_log("INFO".to_string(), true);
    let server = LocalTestServer::start().await;
    let endpoint = server.endpoint.clone();

    let user_a = unique_name("sdk-sync-a");
    let user_b = unique_name("sdk-sync-b");

    signup(endpoint.clone(), user_a.clone(), "pass-a".to_string())
        .await
        .expect("signup a");
    signup(endpoint.clone(), user_b.clone(), "pass-b".to_string())
        .await
        .expect("signup b");

    let info_a = login_with_password(endpoint.clone(), user_a.clone(), "pass-a".to_string())
        .await
        .expect("login a");
    let client_a = Client::new("".to_string(), "".to_string(), &info_a);

    let conversation = client_a
        .create_chat(user_b)
        .await
        .expect("create chat");
    let topic_id = conversation.topic_id.clone();

    for i in 0..2 {
        let req = ChatRequest::new_text(&topic_id, &format!("sync msg {}", i));
        client_a
            .send_chat_request(topic_id.clone(), req)
            .await
            .expect("send chat request");
    }

    let result = Arc::new(Mutex::new(None));
    client_a
        .sync_chat_logs_quick(
            topic_id.clone(),
            None,
            2,
            Box::new(LocalSyncLogsCallback {
                result: result.clone(),
            }),
            Some(true),
        )
        .await;

    check_until(Duration::from_secs(5), || result.lock().unwrap().is_some())
        .await
        .unwrap();
    let logs = result.lock().unwrap().take().unwrap();
    assert_eq!(logs.items.len(), 2);
    assert_eq!(logs.start_seq, 2);
    assert_eq!(logs.end_seq, 1);
}

#[tokio::test]
async fn test_sdk_local_backend_e2e_recall_flow() {
    init_log("INFO".to_string(), true);
    let server = LocalTestServer::start().await;
    let endpoint = server.endpoint.clone();

    let user_a = unique_name("sdk-recall-a");
    let user_b = unique_name("sdk-recall-b");

    signup(endpoint.clone(), user_a.clone(), "pass-a".to_string())
        .await
        .expect("signup a");
    signup(endpoint.clone(), user_b.clone(), "pass-b".to_string())
        .await
        .expect("signup b");

    let info_a = login_with_password(endpoint.clone(), user_a.clone(), "pass-a".to_string())
        .await
        .expect("login a");
    let info_b = login_with_password(endpoint.clone(), user_b.clone(), "pass-b".to_string())
        .await
        .expect("login b");

    let client_a = Client::new("".to_string(), "".to_string(), &info_a);
    let client_b = Client::new("".to_string(), "".to_string(), &info_b);

    let connected_a = Arc::new(AtomicBool::new(false));
    let connected_b = Arc::new(AtomicBool::new(false));
    let acked_send = Arc::new(AtomicBool::new(false));
    let acked_recall = Arc::new(AtomicBool::new(false));

    client_a.set_callback(Some(Box::new(LocalCallback {
        connected: connected_a.clone(),
        received: Arc::new(AtomicBool::new(false)),
        typing_received: Arc::new(AtomicBool::new(false)),
        read_received: Arc::new(AtomicBool::new(false)),
        last_read_seq: Arc::new(AtomicI64::new(0)),
    })));
    client_b.set_callback(Some(Box::new(LocalCallback {
        connected: connected_b.clone(),
        received: Arc::new(AtomicBool::new(false)),
        typing_received: Arc::new(AtomicBool::new(false)),
        read_received: Arc::new(AtomicBool::new(false)),
        last_read_seq: Arc::new(AtomicI64::new(0)),
    })));

    client_a.connect().await;
    client_b.connect().await;

    check_until(Duration::from_secs(3), || connected_a.load(Ordering::Relaxed)).await.unwrap();
    check_until(Duration::from_secs(3), || connected_b.load(Ordering::Relaxed)).await.unwrap();

    let conversation = client_a
        .create_chat(user_b)
        .await
        .expect("create chat");
    let topic_id = conversation.topic_id.clone();

    let send_chat_id = client_a
        .do_send_text(
            topic_id.clone(),
            "hello recall".to_string(),
            None,
            None,
            Some(Box::new(LocalMessageCallback {
                acked: acked_send.clone(),
            })),
        )
        .await
        .expect("send text");

    check_until(Duration::from_secs(5), || acked_send.load(Ordering::Relaxed)).await.unwrap();

    client_a
        .do_recall(
            topic_id.clone(),
            send_chat_id.clone(),
            Some(Box::new(LocalMessageCallback {
                acked: acked_recall.clone(),
            })),
        )
        .await
        .expect("recall message");

    check_until(Duration::from_secs(5), || acked_recall.load(Ordering::Relaxed)).await.unwrap();

    let logs = client_a
        .store
        .get_chat_logs(&topic_id, 0, None, 10)
        .await
        .expect("get local logs")
        .0;
    assert_eq!(logs.items.len(), 2);
    assert_eq!(logs.items[0].content.content_type, "recall");
    assert_eq!(logs.items[0].content.text, send_chat_id);

    let synced = get_chat_logs_desc(&endpoint, &info_a.token, &topic_id, None, 10)
        .await
        .expect("sync recalled logs");
    assert_eq!(synced.items.len(), 2);
    assert_eq!(synced.items[0].content.content_type, "recall");
    assert_eq!(synced.items[0].content.text, send_chat_id);
    assert!(synced.items[1].recall);
    assert_eq!(synced.items[1].content.content_type, "recalled");
}

#[tokio::test]
async fn test_sdk_local_backend_e2e_typing_and_read_flow() {
    init_log("INFO".to_string(), true);
    let server = LocalTestServer::start().await;
    let endpoint = server.endpoint.clone();

    let user_a = unique_name("sdk-tr-a");
    let user_b = unique_name("sdk-tr-b");

    signup(endpoint.clone(), user_a.clone(), "pass-a".to_string())
        .await
        .expect("signup a");
    signup(endpoint.clone(), user_b.clone(), "pass-b".to_string())
        .await
        .expect("signup b");

    let info_a = login_with_password(endpoint.clone(), user_a.clone(), "pass-a".to_string())
        .await
        .expect("login a");
    let info_b = login_with_password(endpoint.clone(), user_b.clone(), "pass-b".to_string())
        .await
        .expect("login b");

    let client_a = Client::new("".to_string(), "".to_string(), &info_a);
    let client_b = Client::new("".to_string(), "".to_string(), &info_b);

    let connected_a = Arc::new(AtomicBool::new(false));
    let connected_b = Arc::new(AtomicBool::new(false));
    let typing_a = Arc::new(AtomicBool::new(false));
    let read_a = Arc::new(AtomicBool::new(false));
    let last_read_seq_a = Arc::new(AtomicI64::new(0));
    let acked_send = Arc::new(AtomicBool::new(false));

    client_a.set_callback(Some(Box::new(LocalCallback {
        connected: connected_a.clone(),
        received: Arc::new(AtomicBool::new(false)),
        typing_received: typing_a.clone(),
        read_received: read_a.clone(),
        last_read_seq: last_read_seq_a.clone(),
    })));
    client_b.set_callback(Some(Box::new(LocalCallback {
        connected: connected_b.clone(),
        received: Arc::new(AtomicBool::new(false)),
        typing_received: Arc::new(AtomicBool::new(false)),
        read_received: Arc::new(AtomicBool::new(false)),
        last_read_seq: Arc::new(AtomicI64::new(0)),
    })));

    client_a.connect().await;
    client_b.connect().await;
    check_until(Duration::from_secs(3), || connected_a.load(Ordering::Relaxed)).await.unwrap();
    check_until(Duration::from_secs(3), || connected_b.load(Ordering::Relaxed)).await.unwrap();

    let conversation = client_a
        .create_chat(user_b)
        .await
        .expect("create chat");
    let topic_id = conversation.topic_id;

    client_b.do_typing(topic_id.clone()).await.expect("typing");
    check_until(Duration::from_secs(5), || typing_a.load(Ordering::Relaxed))
        .await
        .unwrap();

    client_b
        .do_send_text(
            topic_id.clone(),
            "need read".to_string(),
            None,
            None,
            Some(Box::new(LocalMessageCallback {
                acked: acked_send.clone(),
            })),
        )
        .await
        .expect("send");
    check_until(Duration::from_secs(5), || acked_send.load(Ordering::Relaxed))
        .await
        .unwrap();

    client_b.do_read(topic_id).await.expect("read");
    check_until(Duration::from_secs(5), || read_a.load(Ordering::Relaxed))
        .await
        .unwrap();
    assert!(last_read_seq_a.load(Ordering::Relaxed) > 0);
}

#[tokio::test]
async fn test_sdk_local_backend_e2e_remove_and_clear_flow() {
    init_log("INFO".to_string(), true);
    let server = LocalTestServer::start().await;
    let endpoint = server.endpoint.clone();

    let user_a = unique_name("sdk-rm-a");
    let user_b = unique_name("sdk-rm-b");

    signup(endpoint.clone(), user_a.clone(), "pass-a".to_string())
        .await
        .expect("signup a");
    signup(endpoint.clone(), user_b.clone(), "pass-b".to_string())
        .await
        .expect("signup b");

    let info_a = login_with_password(endpoint.clone(), user_a.clone(), "pass-a".to_string())
        .await
        .expect("login a");
    let client_a = Client::new("".to_string(), "".to_string(), &info_a);

    let conversation = client_a
        .create_chat(user_b)
        .await
        .expect("create chat");
    let topic_id = conversation.topic_id.clone();

    let first = ChatRequest::new_text(&topic_id, "to remove");
    let first_id = first.chat_id.clone();
    client_a
        .send_chat_request(topic_id.clone(), first)
        .await
        .expect("send first");
    client_a
        .send_chat_request(topic_id.clone(), ChatRequest::new_text(&topic_id, "to keep"))
        .await
        .expect("send second");

    client_a
        .remove_messages(topic_id.clone(), vec![first_id.clone()], true)
        .await
        .expect("remove messages");

    let synced = get_chat_logs_desc(&endpoint, &info_a.token, &topic_id, None, 10)
        .await
        .expect("sync after remove");
    assert_eq!(synced.items.len(), 2);
    let removed_item = synced
        .items
        .iter()
        .find(|item| item.id == first_id)
        .expect("removed item exists");
    assert_eq!(removed_item.content.content_type, "");

    client_a
        .clean_messages(topic_id.clone())
        .await
        .expect("clear messages");
    let synced_after_clear = get_chat_logs_desc(&endpoint, &info_a.token, &topic_id, None, 10)
        .await
        .expect("sync after clear");
    assert!(synced_after_clear.items.is_empty());
}

#[tokio::test]
async fn test_sdk_local_backend_e2e_update_extra_flow() {
    init_log("INFO".to_string(), true);
    let server = LocalTestServer::start().await;
    let endpoint = server.endpoint.clone();

    let user_a = unique_name("sdk-ux-a");
    let user_b = unique_name("sdk-ux-b");

    signup(endpoint.clone(), user_a.clone(), "pass-a".to_string())
        .await
        .expect("signup a");
    signup(endpoint.clone(), user_b.clone(), "pass-b".to_string())
        .await
        .expect("signup b");

    let info_a = login_with_password(endpoint.clone(), user_a.clone(), "pass-a".to_string())
        .await
        .expect("login a");
    let client_a = Client::new("".to_string(), "".to_string(), &info_a);

    let conversation = client_a
        .create_chat(user_b)
        .await
        .expect("create chat");
    let topic_id = conversation.topic_id.clone();

    let base = ChatRequest::new_text(&topic_id, "with extra");
    let base_id = base.chat_id.clone();
    client_a
        .send_chat_request(topic_id.clone(), base)
        .await
        .expect("send base");

    let mut extra = std::collections::HashMap::new();
    extra.insert("k".to_string(), "v".to_string());
    let update_req = ChatRequest::new_chat(&topic_id, crate::models::ContentType::UpdateExtra)
        .text(&base_id)
        .extra(Some(extra));
    client_a
        .send_chat_request(topic_id.clone(), update_req)
        .await
        .expect("update extra");

    let synced = get_chat_logs_desc(&endpoint, &info_a.token, &topic_id, None, 10)
        .await
        .expect("sync after update extra");
    let updated = synced
        .items
        .iter()
        .find(|item| item.id == base_id)
        .expect("updated base exists");
    assert_eq!(
        updated
            .content
            .extra
            .as_ref()
            .and_then(|m| m.get("k"))
            .map(String::as_str),
        Some("v")
    );
}

#[tokio::test]
async fn test_sdk_local_backend_e2e_reconnect_after_client_restart() {
    init_log("INFO".to_string(), true);
    let server = LocalTestServer::start().await;
    let endpoint = server.endpoint.clone();

    let user_a = unique_name("sdk-rs-a");
    let user_b = unique_name("sdk-rs-b");

    signup(endpoint.clone(), user_a.clone(), "pass-a".to_string())
        .await
        .expect("signup a");
    signup(endpoint.clone(), user_b.clone(), "pass-b".to_string())
        .await
        .expect("signup b");

    let info_a = login_with_password(endpoint.clone(), user_a.clone(), "pass-a".to_string())
        .await
        .expect("login a");
    let info_b = login_with_password(endpoint.clone(), user_b.clone(), "pass-b".to_string())
        .await
        .expect("login b");

    let client_a_v1 = Client::new("".to_string(), "".to_string(), &info_a);
    let client_b = Client::new("".to_string(), "".to_string(), &info_b);

    let connected_a_v1 = Arc::new(AtomicBool::new(false));
    let connected_b = Arc::new(AtomicBool::new(false));
    let recv_a_v1 = Arc::new(AtomicU32::new(0));
    client_a_v1.set_callback(Some(Box::new(CountingCallback {
        connected: connected_a_v1.clone(),
        received_count: recv_a_v1.clone(),
    })));
    client_b.set_callback(Some(Box::new(CountingCallback {
        connected: connected_b.clone(),
        received_count: Arc::new(AtomicU32::new(0)),
    })));

    client_a_v1.connect().await;
    client_b.connect().await;
    check_until(Duration::from_secs(3), || connected_a_v1.load(Ordering::Relaxed))
        .await
        .unwrap();
    check_until(Duration::from_secs(3), || connected_b.load(Ordering::Relaxed))
        .await
        .unwrap();

    let topic_id = client_a_v1
        .create_chat(user_b)
        .await
        .expect("create chat")
        .topic_id;

    let ack_first = Arc::new(AtomicBool::new(false));
    client_b
        .do_send_text(
            topic_id.clone(),
            "before-reconnect".to_string(),
            None,
            None,
            Some(Box::new(LocalMessageCallback {
                acked: ack_first.clone(),
            })),
        )
        .await
        .expect("send first");
    check_until(Duration::from_secs(5), || ack_first.load(Ordering::Relaxed))
        .await
        .unwrap();
    check_until(Duration::from_secs(5), || recv_a_v1.load(Ordering::Relaxed) >= 1)
        .await
        .unwrap();

    client_a_v1.shutdown().await;
    tokio::time::sleep(Duration::from_millis(200)).await;

    let client_a_v2 = Client::new("".to_string(), "".to_string(), &info_a);
    let connected_a_v2 = Arc::new(AtomicBool::new(false));
    let recv_a_v2 = Arc::new(AtomicU32::new(0));
    client_a_v2.set_callback(Some(Box::new(CountingCallback {
        connected: connected_a_v2.clone(),
        received_count: recv_a_v2.clone(),
    })));
    client_a_v2.connect().await;
    check_until(Duration::from_secs(3), || connected_a_v2.load(Ordering::Relaxed))
        .await
        .unwrap();

    let ack_second = Arc::new(AtomicBool::new(false));
    client_b
        .do_send_text(
            topic_id.clone(),
            "after-reconnect".to_string(),
            None,
            None,
            Some(Box::new(LocalMessageCallback {
                acked: ack_second.clone(),
            })),
        )
        .await
        .expect("send second");
    check_until(Duration::from_secs(5), || ack_second.load(Ordering::Relaxed))
        .await
        .unwrap();
    check_until(Duration::from_secs(5), || recv_a_v2.load(Ordering::Relaxed) >= 1)
        .await
        .unwrap();

    let synced = get_chat_logs_desc(&endpoint, &info_a.token, &topic_id, None, 20)
        .await
        .expect("sync logs after reconnect");
    assert!(synced
        .items
        .iter()
        .any(|item| item.content.text == "before-reconnect"));
    assert!(synced
        .items
        .iter()
        .any(|item| item.content.text == "after-reconnect"));
}

#[tokio::test]
async fn test_sdk_local_backend_e2e_batch_sync_chatlogs_stress() {
    init_log("INFO".to_string(), true);
    let server = LocalTestServer::start().await;
    let endpoint = server.endpoint.clone();

    let user_a = unique_name("sdk-batch-a");
    let user_b = unique_name("sdk-batch-b");
    let user_c = unique_name("sdk-batch-c");
    let user_d = unique_name("sdk-batch-d");

    for user in [&user_a, &user_b, &user_c, &user_d] {
        signup(endpoint.clone(), user.to_string(), "pass".to_string())
            .await
            .expect("signup user");
    }

    let info_a = login_with_password(endpoint.clone(), user_a.clone(), "pass".to_string())
        .await
        .expect("login a");
    let client_sender = Client::new("".to_string(), "".to_string(), &info_a);

    let mut topics = Vec::new();
    for peer in [user_b, user_c, user_d] {
        let conversation = client_sender.create_chat(peer).await.expect("create chat");
        topics.push(conversation.topic_id);
    }

    for topic_id in &topics {
        for i in 0..3 {
            let text = format!("batch-{topic_id}-{i}");
            let req = ChatRequest::new_text(topic_id, &text);
            client_sender
                .send_chat_request(topic_id.clone(), req)
                .await
                .expect("send request");
        }
    }

    let client_sync = Client::new("".to_string(), "".to_string(), &info_a);
    let sync_done = Arc::new(AtomicBool::new(false));
    let sync_count = Arc::new(AtomicU32::new(0));

    client_sync
        .sync_conversations(
            None,
            None,
            None,
            None,
            100,
            false,
            None,
            None,
            None,
            Box::new(LocalSyncConversationsCallback {
                done: sync_done.clone(),
                count: sync_count.clone(),
            }),
        )
        .await;

    assert!(sync_done.load(Ordering::Relaxed));
    assert!(sync_count.load(Ordering::Relaxed) >= topics.len() as u32);

    let mut conversation_map = HashMap::new();
    for topic_id in &topics {
        let conversation = client_sync
            .get_conversation(topic_id.clone())
            .await
            .expect("conversation should exist after sync");
        assert!(conversation.last_seq >= 3, "conversation last_seq should be synced");
        conversation_map.insert(topic_id.clone(), conversation);
    }

    client_sync
        .batch_sync_chatlogs(conversation_map.clone(), Some(20))
        .await
        .expect("first batch sync chat logs");
    client_sync
        .batch_sync_chatlogs(conversation_map, Some(20))
        .await
        .expect("second batch sync chat logs");

    for topic_id in &topics {
        let (logs, _need_fetch) = client_sync
            .store
            .get_chat_logs(topic_id, 0, None, 20)
            .await
            .expect("get local chat logs");
        assert!(
            logs.items.len() >= 3,
            "topic {} should have at least 3 logs after batch sync",
            topic_id
        );
        assert!(logs.items.iter().any(|item| item.content.text.contains(topic_id)));
    }
}

#[tokio::test]
async fn test_sdk_local_backend_e2e_reconnect_batch_sync_churn() {
    init_log("INFO".to_string(), true);
    let server = LocalTestServer::start().await;
    let endpoint = server.endpoint.clone();

    let user_a = unique_name("sdk-churn-a");
    let user_b = unique_name("sdk-churn-b");
    let user_c = unique_name("sdk-churn-c");

    for user in [&user_a, &user_b, &user_c] {
        signup(endpoint.clone(), user.to_string(), "pass".to_string())
            .await
            .expect("signup user");
    }

    let info_a = login_with_password(endpoint.clone(), user_a.clone(), "pass".to_string())
        .await
        .expect("login a");
    let info_b = login_with_password(endpoint.clone(), user_b.clone(), "pass".to_string())
        .await
        .expect("login b");
    let info_c = login_with_password(endpoint.clone(), user_c.clone(), "pass".to_string())
        .await
        .expect("login c");

    let sender_b = Client::new("".to_string(), "".to_string(), &info_b);
    let sender_c = Client::new("".to_string(), "".to_string(), &info_c);
    let topic_ab = sender_b
        .create_chat(user_a.clone())
        .await
        .expect("create chat ab")
        .topic_id;
    let topic_ac = sender_c
        .create_chat(user_a.clone())
        .await
        .expect("create chat ac")
        .topic_id;

    let sender_b_connected = Arc::new(AtomicBool::new(false));
    let sender_c_connected = Arc::new(AtomicBool::new(false));
    sender_b.set_callback(Some(Box::new(CountingCallback {
        connected: sender_b_connected.clone(),
        received_count: Arc::new(AtomicU32::new(0)),
    })));
    sender_c.set_callback(Some(Box::new(CountingCallback {
        connected: sender_c_connected.clone(),
        received_count: Arc::new(AtomicU32::new(0)),
    })));
    sender_b.connect().await;
    sender_c.connect().await;
    check_until(Duration::from_secs(3), || sender_b_connected.load(Ordering::Relaxed))
        .await
        .unwrap();
    check_until(Duration::from_secs(3), || sender_c_connected.load(Ordering::Relaxed))
        .await
        .unwrap();

    let receiver = Client::new("".to_string(), "".to_string(), &info_a);
    let receiver_connected = Arc::new(AtomicBool::new(false));
    let receiver_received = Arc::new(AtomicU32::new(0));
    receiver.set_callback(Some(Box::new(CountingCallback {
        connected: receiver_connected.clone(),
        received_count: receiver_received.clone(),
    })));
    receiver.connect().await;
    check_until(Duration::from_secs(3), || receiver_connected.load(Ordering::Relaxed))
        .await
        .unwrap();

    for i in 0..2 {
        let ack_b = Arc::new(AtomicBool::new(false));
        sender_b
            .do_send_text(
                topic_ab.clone(),
                format!("churn-live-ab-{i}"),
                None,
                None,
                Some(Box::new(LocalMessageCallback {
                    acked: ack_b.clone(),
                })),
            )
            .await
            .expect("send live message on ab");
        check_until(Duration::from_secs(5), || ack_b.load(Ordering::Relaxed))
            .await
            .unwrap();

        let ack_c = Arc::new(AtomicBool::new(false));
        sender_c
            .do_send_text(
                topic_ac.clone(),
                format!("churn-live-ac-{i}"),
                None,
                None,
                Some(Box::new(LocalMessageCallback {
                    acked: ack_c.clone(),
                })),
            )
            .await
            .expect("send live message on ac");
        check_until(Duration::from_secs(5), || ack_c.load(Ordering::Relaxed))
            .await
            .unwrap();
    }

    check_until(Duration::from_secs(5), || receiver_received.load(Ordering::Relaxed) >= 4)
        .await
        .unwrap();

    receiver.shutdown().await;
    tokio::time::sleep(Duration::from_millis(200)).await;

    for i in 0..3 {
        sender_b
            .send_chat_request(
                topic_ab.clone(),
                ChatRequest::new_text(&topic_ab, &format!("churn-offline-ab-{i}")),
            )
            .await
            .expect("send offline message on ab");
        sender_c
            .send_chat_request(
                topic_ac.clone(),
                ChatRequest::new_text(&topic_ac, &format!("churn-offline-ac-{i}")),
            )
            .await
            .expect("send offline message on ac");
    }

    let receiver = Client::new("".to_string(), "".to_string(), &info_a);
    let receiver_reconnected = Arc::new(AtomicBool::new(false));
    receiver.set_callback(Some(Box::new(CountingCallback {
        connected: receiver_reconnected.clone(),
        received_count: Arc::new(AtomicU32::new(0)),
    })));
    receiver.connect().await;
    check_until(Duration::from_secs(3), || receiver_reconnected.load(Ordering::Relaxed))
        .await
        .unwrap();

    let sync_done = Arc::new(AtomicBool::new(false));
    let sync_count = Arc::new(AtomicU32::new(0));
    receiver
        .sync_conversations(
            None,
            None,
            None,
            None,
            100,
            false,
            None,
            None,
            None,
            Box::new(LocalSyncConversationsCallback {
                done: sync_done.clone(),
                count: sync_count.clone(),
            }),
        )
        .await;
    assert!(sync_done.load(Ordering::Relaxed));

    let mut conversation_map = HashMap::new();
    for topic_id in [&topic_ab, &topic_ac] {
        let conversation = receiver
            .get_conversation(topic_id.to_string())
            .await
            .expect("conversation exists after reconnect sync");
        conversation_map.insert(topic_id.to_string(), conversation);
    }

    receiver
        .batch_sync_chatlogs(conversation_map.clone(), Some(50))
        .await
        .expect("batch sync after reconnect");
    receiver
        .batch_sync_chatlogs(conversation_map, Some(50))
        .await
        .expect("batch sync second pass after reconnect");

    let (logs_ab, _) = receiver
        .store
        .get_chat_logs(&topic_ab, 0, None, 50)
        .await
        .expect("get chat logs ab");
    let (logs_ac, _) = receiver
        .store
        .get_chat_logs(&topic_ac, 0, None, 50)
        .await
        .expect("get chat logs ac");

    assert!(logs_ab.items.len() >= 5);
    assert!(logs_ab
        .items
        .iter()
        .any(|item| item.content.text == "churn-live-ab-0"));
    assert!(logs_ab
        .items
        .iter()
        .any(|item| item.content.text == "churn-offline-ab-0"));
    assert!(logs_ac.items.len() >= 5);
    assert!(logs_ac
        .items
        .iter()
        .any(|item| item.content.text == "churn-live-ac-0"));
    assert!(logs_ac
        .items
        .iter()
        .any(|item| item.content.text == "churn-offline-ac-0"));
}
