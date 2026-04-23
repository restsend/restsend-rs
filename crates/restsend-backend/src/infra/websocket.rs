use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use tokio::sync::{mpsc, RwLock};

#[derive(Clone, Debug)]
pub enum SessionSender {
    Unbounded(mpsc::UnboundedSender<String>),
    Bounded(mpsc::Sender<String>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SendOutcome {
    Sent,
    Closed,
    Backpressure,
}

impl SessionSender {
    pub fn try_send(&self, payload: String) -> SendOutcome {
        match self {
            SessionSender::Unbounded(sender) => sender
                .send(payload)
                .map(|_| SendOutcome::Sent)
                .unwrap_or(SendOutcome::Closed),
            SessionSender::Bounded(sender) => match sender.try_send(payload) {
                Ok(_) => SendOutcome::Sent,
                Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => SendOutcome::Backpressure,
                Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => SendOutcome::Closed,
            },
        }
    }
}

#[derive(Clone, Debug)]
pub struct WsSession {
    pub device: String,
    pub sender: SessionSender,
}

#[derive(Clone, Default)]
pub struct WsHub {
    peers: Arc<RwLock<HashMap<String, Vec<WsSession>>>>,
}

impl WsHub {
    pub async fn register(&self, user_id: &str, device: &str, sender: SessionSender) {
        let mut peers = self.peers.write().await;
        let sessions = peers.entry(user_id.to_string()).or_default();
        sessions.push(WsSession {
            device: device.to_string(),
            sender,
        });
    }

    pub async fn unregister(&self, user_id: &str, device: &str) {
        let mut peers = self.peers.write().await;
        if let Some(clients) = peers.get_mut(user_id) {
            clients.retain(|session| session.device != device);
            if clients.is_empty() {
                peers.remove(user_id);
            }
        }
    }

    pub async fn broadcast_to_user(
        &self,
        user_id: &str,
        payload: &str,
        drop_on_backpressure: bool,
    ) {
        let mut to_remove = Vec::new();
        let peers = self.peers.read().await;
        if let Some(clients) = peers.get(user_id) {
            for session in clients {
                let outcome = session.sender.try_send(payload.to_string());
                match outcome {
                    SendOutcome::Sent => {}
                    SendOutcome::Closed => {
                        tracing::warn!(
                            user_id = %user_id,
                            device = %session.device,
                            "ws session removed: channel closed"
                        );
                        to_remove.push(session.device.clone());
                    }
                    SendOutcome::Backpressure if drop_on_backpressure => {
                        tracing::warn!(
                            user_id = %user_id,
                            device = %session.device,
                            payload_len = payload.len(),
                            "ws session removed: backpressure"
                        );
                        to_remove.push(session.device.clone());
                    }
                    SendOutcome::Backpressure => {}
                }
            }
        }
        drop(peers);
        for device in to_remove {
            self.unregister(user_id, &device).await;
        }
    }

    pub async fn send_to_device(
        &self,
        user_id: &str,
        device: &str,
        payload: &str,
        drop_on_backpressure: bool,
    ) {
        let mut remove_target = false;
        let peers = self.peers.read().await;
        if let Some(clients) = peers.get(user_id) {
            for session in clients {
                if session.device == device {
                    let outcome = session.sender.try_send(payload.to_string());
                    remove_target = matches!(outcome, SendOutcome::Closed)
                        || (drop_on_backpressure && matches!(outcome, SendOutcome::Backpressure));
                    if remove_target {
                        let reason = match outcome {
                            SendOutcome::Closed => "channel closed",
                            SendOutcome::Backpressure => "backpressure",
                            SendOutcome::Sent => "sent",
                        };
                        tracing::warn!(
                            user_id = %user_id,
                            device = %device,
                            payload_len = payload.len(),
                            reason,
                            "ws session removed"
                        );
                    }
                    break;
                }
            }
        }
        drop(peers);
        if remove_target {
            self.unregister(user_id, device).await;
        }
    }

    pub async fn is_online(&self, user_id: &str) -> bool {
        let peers = self.peers.read().await;
        peers.get(user_id).is_some_and(|items| !items.is_empty())
    }

    pub async fn total_sessions(&self) -> usize {
        let peers = self.peers.read().await;
        peers.values().map(Vec::len).sum()
    }

    pub async fn total_users(&self) -> usize {
        let peers = self.peers.read().await;
        peers
            .iter()
            .filter(|(_, sessions)| !sessions.is_empty())
            .count()
    }

    pub async fn all_user_ids(&self) -> Vec<String> {
        let peers = self.peers.read().await;
        let mut out = HashSet::new();
        for (user_id, sessions) in peers.iter() {
            if !sessions.is_empty() {
                out.insert(user_id.clone());
            }
        }
        out.into_iter().collect()
    }

    pub async fn list_devices(&self, user_id: &str) -> Vec<String> {
        let peers = self.peers.read().await;
        peers
            .get(user_id)
            .map(|sessions| sessions.iter().map(|v| v.device.clone()).collect())
            .unwrap_or_default()
    }
}
