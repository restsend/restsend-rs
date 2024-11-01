use std::time::Duration;

use crate::DEVICE;
#[cfg(test)]
mod tests;

#[cfg(not(target_family = "wasm"))]
mod tokio_impl;

#[allow(dead_code)]
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
    #[cfg(not(target_family = "wasm"))]
    pub handshake_timeout: Duration,
    pub is_cross_domain: bool,
}

impl WebsocketOption {
    pub fn url_from_endpoint(endpoint: &str) -> String {
        let nonce = crate::utils::random_text(4);
        let url = format!("{}/api/connect?device={}&nonce={}", endpoint, DEVICE, nonce);
        url.replace("http", "ws")
    }

    pub fn new(url: &str, token: &str, is_cross_domain: bool) -> Self {
        Self {
            url: url.to_string(),
            token: token.to_string(),
            #[cfg(not(target_family = "wasm"))]
            handshake_timeout: Duration::from_secs(30), // default 30s
            is_cross_domain,
        }
    }
}

#[cfg(not(target_family = "wasm"))]
pub(crate) type WebSocket = tokio_impl::WebSocketImpl;

#[cfg(target_family = "wasm")]
pub(crate) type WebSocket = web_sys_impl::WebSocketImpl;
