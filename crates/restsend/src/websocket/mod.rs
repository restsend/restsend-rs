use std::time::Duration;
#[cfg(test)]
mod tests;

#[cfg(not(target_family = "wasm"))]
mod tokio_impl;

#[cfg(target_family = "wasm")]
mod web_sys_impl;

#[allow(unused)]
pub trait WebSocketCallback: Send + Sync {
    fn on_connected(&self, usage: Duration) {}
    fn on_connecting(&self) {}
    fn on_unauthorized(&self) {}
    fn on_net_broken(&self, reason: String) {}
    fn on_message(&self, message: String) {}
}

#[derive(Debug, Clone)]
pub struct WebsocketOption {
    pub url: String,
    pub token: String,
    pub handshake_timeout: Duration,
}

impl WebsocketOption {
    pub fn url_from_endpoint(endpoint: &str) -> String {
        format!("{}/api/connect", endpoint)
    }
    pub fn new(url: &str, token: &str) -> Self {
        Self {
            url: url.to_string(),
            token: token.to_string(),
            handshake_timeout: Duration::from_secs(30), // default 30s
        }
    }
}

#[cfg(not(target_family = "wasm"))]
pub(crate) type WebSocket = tokio_impl::WebSocketImpl;

#[cfg(target_family = "wasm")]
pub(crate) type WebSocket = web_sys_impl::WebSocketImpl;
