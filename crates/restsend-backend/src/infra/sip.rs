//! SIP-over-WebSocket relay (Go `handler_sip_relay.go` parity).
//!
//! The client sends `{type:"sip", message:"<raw SIP text>"}` frames; the
//! backend lazily opens a WebSocket to the PBX (`SIP_RELAY_PBX_WS`, sub
//! protocol `sip`) and pipes raw text in both directions without parsing.
//! When the PBX drops the link the relay is torn down; the next client
//! frame transparently reconnects.

use std::collections::HashMap;
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;

use crate::app::AppState;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SessionKey {
    user_id: Arc<str>,
    device: Arc<str>,
}

struct PbxLink {
    outbound: mpsc::Sender<String>,
    alive: Arc<std::sync::atomic::AtomicBool>,
}

#[derive(Clone, Default)]
pub struct SipRelay {
    links: Arc<Mutex<HashMap<SessionKey, PbxLink>>>,
}

impl SipRelay {
    pub fn new() -> Self {
        Self::default()
    }

    /// Forward a raw SIP text from the client towards the PBX.
    pub async fn relay_from_client(
        &self,
        state: &AppState,
        user_id: &str,
        device: &str,
        text: &str,
    ) {
        if state.config.sip_relay_pbx_ws.is_empty() {
            tracing::warn!("sip relay not configured (SIP_RELAY_PBX_WS empty)");
            return;
        }

        let key = SessionKey {
            user_id: Arc::from(user_id),
            device: Arc::from(device),
        };

        let outbound = {
            let mut links = self.links.lock().await;
            let reuse = match links.get(&key) {
                Some(link) => link.alive.load(std::sync::atomic::Ordering::Relaxed),
                None => false,
            };
            if reuse {
                links.get(&key).unwrap().outbound.clone()
            } else {
                let link = match self.dial(state, key.clone()).await {
                    Ok(link) => link,
                    Err(err) => {
                        tracing::warn!(user_id = %user_id, error = %err, "sip pbx dial failed");
                        self.push_client_error(state, user_id, device, &err).await;
                        return;
                    }
                };
                tracing::info!(user_id = %user_id, device = %device, "sip pbx link established");
                let outbound = link.outbound.clone();
                links.insert(key, link);
                outbound
            }
        };

        if let Err(err) = outbound.send(text.to_string()).await {
            tracing::warn!(user_id = %user_id, error = %err, "sip uplink send failed");
        }
    }

    /// Tear down a session's PBX link (called when the client disconnects).
    pub async fn close_session(&self, user_id: &str, device: &str) {
        let key = SessionKey {
            user_id: Arc::from(user_id),
            device: Arc::from(device),
        };
        if let Some(link) = self.links.lock().await.remove(&key) {
            link.alive.store(false, std::sync::atomic::Ordering::Relaxed);
        }
    }

    async fn push_client_error(&self, state: &AppState, user_id: &str, device: &str, err: &str) {
        let payload = serde_json::json!({
            "type": "sip",
            "message": "",
            "code": 503,
            "error": err,
        })
        .to_string();
        state
            .ws_hub
            .send_to_device(user_id, device, &payload, state.config.ws_drop_on_backpressure)
            .await;
    }

    fn push_client_sip(state: &AppState, key: &SessionKey, text: String) {
        let payload = serde_json::json!({
            "type": "sip",
            "message": text,
        })
        .to_string();
        let state = state.clone();
        let key = key.clone();
        tokio::spawn(async move {
            state
                .ws_hub
                .send_to_device(
                    &key.user_id,
                    &key.device,
                    &payload,
                    state.config.ws_drop_on_backpressure,
                )
                .await;
        });
    }

    async fn dial(&self, state: &AppState, key: SessionKey) -> Result<PbxLink, String> {
        let url = &state.config.sip_relay_pbx_ws;
        let mut request = url
            .into_client_request()
            .map_err(|err| format!("invalid SIP_RELAY_PBX_WS url: {err}"))?;
        request.headers_mut().insert(
            "Sec-WebSocket-Protocol",
            "sip".parse().map_err(|_| "bad protocol")?,
        );

        let (ws, _resp) = tokio_tungstenite::connect_async(request)
            .await
            .map_err(|err| format!("pbx connect failed: {err}"))?;
        let (mut sink, mut stream) = ws.split();

        let (outbound_tx, mut outbound_rx) = mpsc::channel::<String>(256);
        let alive = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let uplink_alive = alive.clone();

        // uplink: client text -> pbx
        tokio::spawn(async move {
            while let Some(text) = outbound_rx.recv().await {
                if sink.send(Message::Text(text)).await.is_err() {
                    break;
                }
            }
            uplink_alive.store(false, std::sync::atomic::Ordering::Relaxed);
        });

        // downlink: pbx text -> client device
        let downlink_state = state.clone();
        let downlink_alive = alive.clone();
        tokio::spawn(async move {
            loop {
                match stream.next().await {
                    Some(Ok(Message::Text(text))) => {
                        Self::push_client_sip(&downlink_state, &key, text);
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(err)) => {
                        tracing::warn!(error = %err, "sip pbx downlink error");
                        break;
                    }
                    _ => {}
                }
            }
            downlink_alive.store(false, std::sync::atomic::Ordering::Relaxed);
            tracing::info!("sip pbx link closed");
        });

        Ok(PbxLink {
            outbound: outbound_tx,
            alive,
        })
    }
}
