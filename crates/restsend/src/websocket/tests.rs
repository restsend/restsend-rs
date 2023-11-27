const TEST_ENDPOINT: &str = "ws://chat.ruzhila.cn/";
use std::time::Duration;
struct WebSocketCallbackImpl {}
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
async fn test_websocket() {
    let mut ws = super::WebSocket::new();
    let opt = super::WebsocketOption::new(TEST_ENDPOINT, "");
    let cb = Box::new(WebSocketCallbackImpl {});
    ws.serve(&opt, cb).await.expect("connect fail");
}
