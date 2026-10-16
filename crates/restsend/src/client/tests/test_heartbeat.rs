#![cfg(not(target_arch = "wasm32"))]

use crate::{
    callback,
    client::Client,
    models::AuthInfo,
    utils::{check_until, init_log},
};
use axum::{
    extract::{ws::Message, ws::WebSocket, ws::WebSocketUpgrade, State},
    response::Response,
    routing::get,
    Router,
};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tokio::net::TcpListener;

struct HeartbeatCallback {
    connected: Arc<AtomicUsize>,
    broken: Arc<AtomicUsize>,
    reason: Arc<Mutex<String>>,
}

impl callback::RsCallback for HeartbeatCallback {
    fn on_connected(&self) {
        self.connected.fetch_add(1, Ordering::Relaxed);
    }

    fn on_net_broken(&self, reason: String) {
        *self.reason.lock().unwrap() = reason;
        self.broken.fetch_add(1, Ordering::Relaxed);
    }
}

#[derive(Clone)]
struct FakeState {
    /// Whether the fake peer answers heartbeat pings at all.
    respond: Arc<AtomicBool>,
    /// Artificial delay before answering a heartbeat ping.
    delay_ms: Arc<AtomicU64>,
}

async fn fake_ws(State(state): State<FakeState>, ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(move |socket| fake_conn(socket, state))
}

/// A configurable fake server: it answers application-level `ping` requests
/// with a matching `resp` (optionally delayed, optionally not at all), which is
/// enough to exercise the client heartbeat state machine.
async fn fake_conn(mut socket: WebSocket, state: FakeState) {
    while let Some(Ok(msg)) = socket.recv().await {
        let Message::Text(text) = msg else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
            continue;
        };
        if value.get("type").and_then(|v| v.as_str()) != Some("ping") {
            continue;
        }

        let delay = state.delay_ms.load(Ordering::Relaxed);
        if delay > 0 {
            tokio::time::sleep(Duration::from_millis(delay)).await;
        }
        if !state.respond.load(Ordering::Relaxed) {
            continue;
        }

        let resp = serde_json::json!({
            "type": "resp",
            "chatId": value.get("chatId").cloned().unwrap_or_default(),
            "code": 200,
            "content": value.get("content").cloned(),
        });
        if socket.send(Message::Text(resp.to_string())).await.is_err() {
            break;
        }
    }
}

async fn start_fake_server(respond: Arc<AtomicBool>, delay_ms: Arc<AtomicU64>) -> String {
    let app = Router::new()
        .route("/api/connect", get(fake_ws))
        .with_state(FakeState { respond, delay_ms });
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{}", addr)
}

fn new_client(endpoint: &str, interval_secs: u64, timeout_secs: u64) -> Arc<Client> {
    let info = AuthInfo::new(endpoint, "u1", "token-1");
    let c = Client::new("".to_string(), "".to_string(), &info);
    c.store
        .option
        .ping_interval_secs
        .store(interval_secs as usize, Ordering::Relaxed);
    c.store
        .option
        .ping_timeout_secs
        .store(timeout_secs as usize, Ordering::Relaxed);
    c
}

fn attach_callback(c: &Client) -> (Arc<AtomicUsize>, Arc<AtomicUsize>, Arc<Mutex<String>>) {
    let connected = Arc::new(AtomicUsize::new(0));
    let broken = Arc::new(AtomicUsize::new(0));
    let reason = Arc::new(Mutex::new(String::new()));
    c.set_callback(Some(Box::new(HeartbeatCallback {
        connected: connected.clone(),
        broken: broken.clone(),
        reason: reason.clone(),
    })));
    (connected, broken, reason)
}

/// A healthy peer that answers every heartbeat promptly must never be declared
/// broken, across several probe cycles.
#[tokio::test]
async fn test_healthy_connection_stays_connected() {
    init_log("INFO".to_string(), true);
    let endpoint =
        start_fake_server(Arc::new(AtomicBool::new(true)), Arc::new(AtomicU64::new(0))).await;

    let c = new_client(&endpoint, 1, 1);
    let (connected, broken, _reason) = attach_callback(&c);
    c.connect().await;

    check_until(Duration::from_secs(5), || {
        connected.load(Ordering::Relaxed) > 0
    })
    .await
    .expect("client should connect");

    // Run through ~4 heartbeat cycles.
    tokio::time::sleep(Duration::from_secs(4)).await;

    assert_eq!(
        broken.load(Ordering::Relaxed),
        0,
        "a healthy connection must never be reported broken"
    );
    assert_eq!(c.connection_status(), "connected");
    assert_eq!(
        connected.load(Ordering::Relaxed),
        1,
        "a healthy connection must not reconnect"
    );
}

/// A pong that arrives later than the per-probe timeout but within the two-miss
/// tolerance must be treated as alive (no false positive on slow networks).
#[tokio::test]
async fn test_late_pong_within_tolerance_survives() {
    init_log("INFO".to_string(), true);
    // Reply ~1.2s after each ping while the per-probe timeout is 1s: the first
    // attempt "misses" but the retry is cleared by the late pong.
    let endpoint = start_fake_server(
        Arc::new(AtomicBool::new(true)),
        Arc::new(AtomicU64::new(1200)),
    )
    .await;

    let c = new_client(&endpoint, 2, 1);
    let (connected, broken, _reason) = attach_callback(&c);
    c.connect().await;

    check_until(Duration::from_secs(5), || {
        connected.load(Ordering::Relaxed) > 0
    })
    .await
    .expect("client should connect");

    tokio::time::sleep(Duration::from_secs(7)).await;

    assert_eq!(
        broken.load(Ordering::Relaxed),
        0,
        "a slow-but-alive peer must not be reported broken"
    );
    assert_eq!(c.connection_status(), "connected");
}

/// A peer that stops answering heartbeats is detected and the client reconnects.
#[tokio::test]
async fn test_dead_connection_detected_and_reconnected() {
    init_log("INFO".to_string(), true);
    let endpoint = start_fake_server(
        Arc::new(AtomicBool::new(false)),
        Arc::new(AtomicU64::new(0)),
    )
    .await;

    let c = new_client(&endpoint, 1, 1);
    let (connected, broken, reason) = attach_callback(&c);
    c.connect().await;

    check_until(Duration::from_secs(5), || {
        connected.load(Ordering::Relaxed) > 0
    })
    .await
    .expect("client should connect");

    check_until(Duration::from_secs(12), || {
        broken.load(Ordering::Relaxed) > 0
    })
    .await
    .expect("client should detect the dead connection via ping timeout");

    assert!(
        reason.lock().unwrap().contains("ping timeout"),
        "unexpected reason: {}",
        reason.lock().unwrap()
    );

    check_until(Duration::from_secs(12), || {
        connected.load(Ordering::Relaxed) >= 2
    })
    .await
    .expect("client should reconnect after a detected death");
}

/// End-to-end against the real backend: a healthy client must stay connected
/// across several fast heartbeat cycles (no false positive from inline ping
/// responses, and the native read-timeout backstop must not trip).
#[tokio::test]
async fn test_real_backend_heartbeat_keeps_connection() {
    use crate::client::tests::test_server::LocalTestServer;
    use crate::client::tests::unique_test_user;
    use crate::services::auth::{login_with_password, signup};

    init_log("INFO".to_string(), true);
    let server = LocalTestServer::start().await;
    let user = unique_test_user("hb-real");
    let password = "pass-1".to_string();
    signup(server.endpoint.clone(), user.clone(), password.clone())
        .await
        .expect("signup");
    let info = login_with_password(server.endpoint.clone(), user, password)
        .await
        .expect("login");

    // Use the real token/user from login (the fake-server helper hardcodes one).
    let c = Arc::new(Client::new("".to_string(), "".to_string(), &info));
    c.store
        .option
        .ping_interval_secs
        .store(1, Ordering::Relaxed);
    c.store.option.ping_timeout_secs.store(1, Ordering::Relaxed);
    let (connected, broken, _reason) = attach_callback(&c);
    c.connect().await;

    check_until(Duration::from_secs(5), || {
        connected.load(Ordering::Relaxed) > 0
    })
    .await
    .expect("client should connect");

    // Run through several heartbeat cycles against the real backend.
    tokio::time::sleep(Duration::from_secs(4)).await;

    assert_eq!(
        broken.load(Ordering::Relaxed),
        0,
        "real backend heartbeat must keep the connection alive"
    );
    assert_eq!(c.connection_status(), "connected");
}

/// After the peer recovers, the auto-reconnect loop brings the client back to a
/// healthy connected state and it stays there (no thrashing).
#[tokio::test]
async fn test_reconnects_and_recovers_when_peer_returns() {
    init_log("INFO".to_string(), true);
    let respond = Arc::new(AtomicBool::new(false));
    let endpoint = start_fake_server(respond.clone(), Arc::new(AtomicU64::new(0))).await;

    let c = new_client(&endpoint, 1, 1);
    let (connected, broken, _reason) = attach_callback(&c);
    c.connect().await;

    // Silent peer: the client detects death and keeps reconnecting.
    check_until(Duration::from_secs(5), || {
        connected.load(Ordering::Relaxed) > 0
    })
    .await
    .expect("first connect");
    check_until(Duration::from_secs(12), || {
        broken.load(Ordering::Relaxed) > 0
    })
    .await
    .expect("detect death");
    check_until(Duration::from_secs(12), || {
        connected.load(Ordering::Relaxed) >= 2
    })
    .await
    .expect("reconnect after death");

    // The peer comes back to life.
    respond.store(true, Ordering::Relaxed);

    check_until(Duration::from_secs(12), || {
        c.connection_status() == "connected"
    })
    .await
    .expect("client should reconnect to the recovered peer");

    // Settle: the recovered connection must remain healthy across several
    // heartbeat cycles.
    tokio::time::sleep(Duration::from_secs(4)).await;
    assert_eq!(
        c.connection_status(),
        "connected",
        "recovered connection must stay connected"
    );
}
