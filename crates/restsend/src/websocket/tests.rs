use crate::client::tests::test_server::LocalTestServer;
use std::time::Duration;

struct WebSocketCallbackImpl {}

impl Default for WebSocketCallbackImpl {
    fn default() -> Self {
        Self {}
    }
}

impl super::WebSocketCallback for WebSocketCallbackImpl {
    fn on_connected(&self, usage: Duration) {
        println!("on_connected, usage:{:?}", usage);
    }
    fn on_connecting(&self) {
        println!("on_connecting");
    }
    fn on_net_broken(&self, reason: String) {
        println!("on_net_broken: {}", reason);
    }
    fn on_message(&self, message: String) {
        println!("on_message: {}", message);
    }
}

#[tokio::test]
async fn test_websocket_bad_handshake() {
    let server = LocalTestServer::start().await;
    let ws = super::WebSocket::new();
    let opt = super::WebsocketOption::new(&server.endpoint, "", false);
    let cb = Box::new(WebSocketCallbackImpl::default());
    let r = ws.serve(&opt, cb).await;
    assert!(r
        .unwrap_err()
        .to_string()
        .contains("expected HTTP 101 Switching Protocols"));
}

#[tokio::test]
async fn test_websocket_handshake() {
    let server = LocalTestServer::start().await;
    let ws = super::WebSocket::new();
    let url = super::WebsocketOption::url_from_endpoint(&server.endpoint);
    let opt = super::WebsocketOption::new(&url, "", false);
    let cb = Box::new(WebSocketCallbackImpl::default());
    let r = ws.serve(&opt, cb).await;
    assert!(r
        .unwrap_err()
        .to_string()
        .contains("expected HTTP 101 Switching Protocols"));
}

/// The native transport backstop must give up on a peer that upgrades the
/// socket and then never sends anything (half-open style), and report it via
/// `on_net_broken` so the connection layer reconnects.
#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn test_websocket_read_timeout_backstop() {
    use axum::{
        extract::ws::{WebSocket, WebSocketUpgrade},
        response::Response,
        routing::get,
        Router,
    };
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    };

    struct Recorder {
        reason: Arc<Mutex<String>>,
        broken: Arc<AtomicBool>,
    }
    impl super::WebSocketCallback for Recorder {
        fn on_net_broken(&self, reason: String) {
            *self.reason.lock().unwrap() = reason;
            self.broken.store(true, Ordering::Relaxed);
        }
    }

    async fn silent(ws: WebSocketUpgrade) -> Response {
        ws.on_upgrade(|mut socket: WebSocket| async move {
            while let Some(Ok(_)) = socket.recv().await {}
        })
    }

    let app = Router::new().route("/api/connect", get(silent));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let mut opt = super::WebsocketOption::new(&format!("ws://{addr}/api/connect"), "", false);
    opt.read_timeout = Duration::from_millis(300);

    let reason = Arc::new(Mutex::new(String::new()));
    let broken = Arc::new(AtomicBool::new(false));
    let cb = Box::new(Recorder {
        reason: reason.clone(),
        broken: broken.clone(),
    });

    super::WebSocket::new().serve(&opt, cb).await.unwrap();

    assert!(
        broken.load(Ordering::Relaxed),
        "read timeout should report a broken connection"
    );
    assert!(
        reason.lock().unwrap().contains("read timeout"),
        "unexpected reason: {}",
        reason.lock().unwrap()
    );
}
