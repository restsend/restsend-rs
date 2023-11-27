const TEST_ENDPOINT: &str = "https://chat.ruzhila.cn";
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
    let mut ws = super::WebSocket::new();
    let opt = super::WebsocketOption::new(TEST_ENDPOINT, "");
    let cb = Box::new(WebSocketCallbackImpl::default());
    let r = ws.serve(&opt, cb).await;
    assert!(r
        .unwrap_err()
        .to_string()
        .contains("expected HTTP 101 Switching Protocols"));
}

#[tokio::test]
async fn test_websocket_handshake() {
    let mut ws = super::WebSocket::new();
    let url = format!("{}/api/connect", TEST_ENDPOINT);
    let opt = super::WebsocketOption::new(&url, "");
    let cb = Box::new(WebSocketCallbackImpl::default());
    let r = ws.serve(&opt, cb).await;
    assert!(r
        .unwrap_err()
        .to_string()
        .contains("expected HTTP 101 Switching Protocols"));
}
