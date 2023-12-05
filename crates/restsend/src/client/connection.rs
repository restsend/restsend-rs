use super::Client;
use crate::{
    callback::Callback,
    request::{ChatRequest, ChatRequestType},
    websocket::{WebSocket, WebSocketCallback, WebsocketOption},
    KEEPALIVE_INTERVAL_SECS, MAX_CONNECT_INTERVAL_SECS,
};

use log::{debug, info, warn};
use serde_json::de;
use std::{
    pin::Pin,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tokio::{
    select,
    sync::{
        broadcast,
        mpsc::{unbounded_channel, UnboundedSender},
    },
    time::sleep,
};

#[derive(Clone)]
pub(super) enum ConnectionStatus {
    Broken,
    ConnectNow,
    Connected,
    Connecting,
    Shutdown,
}

pub(super) struct ConnectState {
    must_shutdown: AtomicBool,
    broken_count: AtomicU64,
    last_broken_at: Mutex<Option<Instant>>,
    last_alive_at: Mutex<Instant>,
    state_tx: broadcast::Sender<ConnectionStatus>,
}

impl ConnectState {
    pub fn new() -> Self {
        let (state_tx, _) = broadcast::channel(1);
        Self {
            must_shutdown: AtomicBool::new(false),
            broken_count: AtomicU64::new(0),
            last_broken_at: Mutex::new(None),
            last_alive_at: Mutex::new(Instant::now()),
            state_tx,
        }
    }

    pub fn did_shutdown(&self) {
        self.must_shutdown.store(true, Ordering::Relaxed);
        self.state_tx.send(ConnectionStatus::Shutdown).ok();
    }

    pub fn is_must_shutdown(&self) -> bool {
        self.must_shutdown.load(Ordering::Relaxed)
    }

    pub fn did_connecting(&self) {
        *self.last_alive_at.lock().unwrap() = Instant::now();
        self.state_tx.send(ConnectionStatus::Connecting).ok();
    }

    pub fn did_connected(&self) {
        self.broken_count.store(0, Ordering::Relaxed);
        self.last_broken_at.lock().unwrap().take();
        *self.last_alive_at.lock().unwrap() = Instant::now();
        self.state_tx.send(ConnectionStatus::Connected).ok();
    }

    pub fn did_sent_or_recvived(&self) {
        *self.last_alive_at.lock().unwrap() = Instant::now();
    }

    pub fn did_broken(&self) {
        self.broken_count.fetch_add(1, Ordering::Relaxed);
        *self.last_broken_at.lock().unwrap() = Some(Instant::now());
        self.state_tx.send(ConnectionStatus::Broken).ok();
    }

    pub fn need_send_keepalive(&self) -> bool {
        let last_alive_at = self.last_alive_at.lock().unwrap();
        let elapsed = last_alive_at.elapsed().as_secs();
        elapsed >= KEEPALIVE_INTERVAL_SECS
    }

    pub async fn wait_for_next_connect(&self) {
        let broken_count = self.broken_count.load(Ordering::Relaxed);
        if broken_count <= 0 {
            return;
        }

        let remain_secs = broken_count.max(MAX_CONNECT_INTERVAL_SECS);
        let mut rx = self.state_tx.subscribe();

        select! {
            _ = tokio::time::sleep(Duration::from_secs(remain_secs)) => {
            },
            _ = rx.recv() => {
                self.broken_count.store(0, Ordering::Relaxed);
                self.last_broken_at.lock().unwrap().take();
            }
        }
    }
}

struct ConnectionInner {
    connect_state_ref: Arc<ConnectState>,
    callback_ref: Arc<Box<dyn Callback>>,
    incoming_tx: UnboundedSender<ChatRequest>,
}

impl WebSocketCallback for ConnectionInner {
    fn on_connected(&self, _usage: Duration) {
        self.connect_state_ref.did_connected();
        self.callback_ref.on_connected();
    }

    fn on_connecting(&self) {
        self.callback_ref.on_connecting();
        self.connect_state_ref.did_connecting();
    }

    fn on_unauthorized(&self) {
        self.callback_ref
            .on_token_expired("unauthorized".to_string());
    }

    fn on_net_broken(&self, reason: String) {
        self.connect_state_ref.did_broken();
        self.callback_ref.on_net_broken(reason);
    }

    fn on_message(&self, message: String) {
        debug!("websocket message: {}", message);
        self.connect_state_ref.did_sent_or_recvived();

        let req = match ChatRequest::try_from(message) {
            Ok(req) => req,
            Err(e) => {
                warn!("websocket parse message error: {}", e);
                return;
            }
        };

        match ChatRequestType::from(&req.r#type) {
            ChatRequestType::Unknown(t) => {
                warn!("websocket unknown message: {}", t);
                return;
            }
            ChatRequestType::Nop => {
                return;
            }
            _ => {
                self.incoming_tx.send(req).unwrap();
            }
        }
    }
}

impl Client {
    pub async fn connect(&self, callback: Box<dyn Callback>) {
        self.serve_connection(callback).await
    }

    async fn serve_connection(&self, callback: Box<dyn Callback>) {
        let url = WebsocketOption::url_from_endpoint(&self.endpoint);
        let opt = WebsocketOption::new(&url, &self.token);

        let state = self.state.clone();
        let callback = Arc::new(callback);
        let callback_clone = callback.clone();
        let store_ref = self.store.clone();

        info!("connect websocket url: {}", url);
        let conn_state_ref = state.state_tx.clone();

        tokio::spawn(async move {
            let conn_loop = async {
                while !state.is_must_shutdown() {
                    state.wait_for_next_connect().await;

                    let (incoming_tx, mut incoming_rx) = unbounded_channel();

                    let conn_inner = ConnectionInner {
                        connect_state_ref: state.clone(),
                        callback_ref: callback.clone(),
                        incoming_tx,
                    };

                    let conn = WebSocket::new();
                    let (outgoing_tx, mut outgoing_rx) = unbounded_channel::<ChatRequest>();

                    let sender_loop = async {
                        while let Some(message) = outgoing_rx.recv().await {
                            if let Err(_) = conn.send((&message).into()).await {
                                store_ref.handle_send_fail(&message.id).await;
                                break;
                            }
                            state.did_sent_or_recvived();
                            store_ref.handle_send_success(&message.id).await;
                        }
                    };

                    let keepalive_loop = async {
                        while !state.is_must_shutdown() {
                            sleep(Duration::from_secs(5 as u64)).await;
                            if !state.need_send_keepalive() {
                                continue;
                            }
                            if let Err(e) = conn.send(String::from(r#"{"type":"nop"}"#)).await {
                                warn!("keepalive_runner send failed: {:?}", e);
                                break;
                            }
                        }
                    };

                    let incoming_loop = async {
                        while let Some(req) = incoming_rx.recv().await {
                            let resps = match ChatRequestType::from(&req.r#type) {
                                ChatRequestType::Nop => vec![],
                                ChatRequestType::Unknown(_) => {
                                    vec![callback.on_unknown_request(req)]
                                }
                                ChatRequestType::System => vec![callback.on_system_request(req)],
                                ChatRequestType::Typing => {
                                    callback
                                        .on_topic_typing(req.topic_id.clone(), req.message.clone());
                                    vec![]
                                }
                                ChatRequestType::Kickout => {
                                    let reason = req.message.unwrap_or_default();
                                    warn!("websocket kickout by other client: {}", reason);

                                    state.did_shutdown();
                                    callback.on_kickoff_by_other_client(reason);
                                    break;
                                }
                                _ => store_ref.process_incoming(req, callback.clone()).await,
                            };

                            for resp in resps {
                                if let Some(resp) = resp {
                                    if let Err(e) = conn.send((&resp).into()).await {
                                        warn!("websocket send failed: {}", e);
                                        break;
                                    }
                                }
                            }
                        }
                    };

                    select! {
                        _ = conn.serve(&opt, Box::new(conn_inner)) => {},
                        _ = store_ref.handle_outgoing(outgoing_tx) => {
                        },
                        _ = sender_loop => {},
                        _ = incoming_loop => {},
                        _ = keepalive_loop => {}
                    }
                }
            };

            select! {
                _ = store_ref.process(callback_clone) =>{}
                _ = conn_loop => {
                    info!("connect shutdown");
                },
                _ = async {
                    loop {
                        let mut conn_state_rx = conn_state_ref.subscribe();
                        let st = conn_state_rx.recv().await;
                        match st {
                            Ok(ConnectionStatus::Connected) => {
                                store_ref.flush_offline_requests().await;
                            }
                            Ok(ConnectionStatus::Shutdown) | Err(_) => {
                                break
                            }
                            _ => {}
                        }
                    }
                } => {}
            };
        });
    }

    pub async fn app_active(&self) {
        self.state.state_tx.send(ConnectionStatus::ConnectNow).ok();
    }

    pub async fn app_deactivate(&self) {
        todo!("app_deactivate")
    }

    pub async fn shutdown(&self) {
        info!("shutdown websocket");
        self.state.did_shutdown();
        self.store.shutdown().await;
    }
}
