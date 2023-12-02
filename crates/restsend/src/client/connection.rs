use super::{store::ClientStoreRef, Client, WSMessage};
use crate::{
    callback::Callback,
    request::{ChatRequest, ChatRequestType},
    websocket::{WebSocket, WebSocketCallback, WebsocketOption},
    KEEPALIVE_INTERVAL_SECS, MAX_CONNECT_INTERVAL_SECS,
};

use anyhow::Result;
use log::{debug, info, warn};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tokio::{
    select,
    sync::{
        mpsc::{self, unbounded_channel, UnboundedSender},
        oneshot,
    },
    time::sleep,
};

struct ConnectState {
    must_broken: AtomicBool,
    broken_count: AtomicU64,
    last_broken_at: Mutex<Option<Instant>>,
    last_alive_at: Mutex<Instant>,
}

impl ConnectState {
    pub fn new() -> Self {
        Self {
            must_broken: AtomicBool::new(false),
            broken_count: AtomicU64::new(0),
            last_broken_at: Mutex::new(None),
            last_alive_at: Mutex::new(Instant::now()),
        }
    }

    pub fn disconnect(&self) {
        self.must_broken.store(true, Ordering::Relaxed);
    }

    pub fn is_must_broken(&self) -> bool {
        self.must_broken.load(Ordering::Relaxed)
    }

    pub fn did_connected(&self) {
        self.broken_count.store(0, Ordering::Relaxed);
        self.last_broken_at.lock().unwrap().take();
        *self.last_alive_at.lock().unwrap() = Instant::now();
    }

    pub fn did_sent_or_recvived(&self) {
        *self.last_alive_at.lock().unwrap() = Instant::now();
    }

    pub fn did_broken(&self) {
        self.broken_count.fetch_add(1, Ordering::Relaxed);
        *self.last_broken_at.lock().unwrap() = Some(Instant::now());
    }

    pub fn need_send_keepalive(&self) -> bool {
        let last_alive_at = self.last_alive_at.lock().unwrap();
        let elapsed = last_alive_at.elapsed().as_secs();
        elapsed >= KEEPALIVE_INTERVAL_SECS
    }

    pub async fn wait_for_next_connect(&self, connect_now_rx: oneshot::Receiver<()>) {
        let broken_count = self.broken_count.load(Ordering::Relaxed);
        if broken_count <= 0 {
            return;
        }

        let remain_secs = broken_count.max(MAX_CONNECT_INTERVAL_SECS);
        select! {
            _ = tokio::time::sleep(Duration::from_secs(remain_secs)) => {
            },
            _ = connect_now_rx => {
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
    fn on_connected(&self, usage: Duration) {
        self.connect_state_ref.did_connected();
        self.callback_ref.on_connected();
    }
    fn on_connecting(&self) {
        self.callback_ref.on_connecting();
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
        self.incoming_tx.send(req).unwrap();
    }
}

impl Client {
    pub async fn connect(&self, callback: Box<dyn Callback>) {
        let (tx, mut rx) = mpsc::unbounded_channel::<WSMessage>();
        *self.ws_sender.lock().unwrap() = Some(tx.clone());
        let tx_for_requeue = tx.clone();

        let url = WebsocketOption::url_from_endpoint(&self.endpoint);
        let opt = WebsocketOption::new(&url, &self.token);

        let state = Arc::new(ConnectState::new());
        let callback = Arc::new(callback);
        let store_ref = self.store.clone();

        info!("connect websocket url: {}", url);

        let connect_now_ref = self.connect_now.clone();

        tokio::spawn(async move {
            while !state.is_must_broken() {
                let (connect_now_tx, connect_now_rx) = oneshot::channel();
                connect_now_ref.lock().unwrap().replace(connect_now_tx);
                state.wait_for_next_connect(connect_now_rx).await;

                let (incoming_tx, mut incoming_rx) = unbounded_channel();

                let conn_inner = Box::new(ConnectionInner {
                    connect_state_ref: state.clone(),
                    callback_ref: callback.clone(),
                    incoming_tx,
                });

                let conn = WebSocket::new();

                let sender_loop = async {
                    while let Some(message) = rx.recv().await {
                        match message {
                            Some(message) => {
                                state.did_sent_or_recvived();
                                let data: String = (&message.req).into();

                                if let Err(_) = conn.send(data).await {
                                    // requeue the message
                                    if !message.is_expired() && !state.is_must_broken() {
                                        message.did_retry();
                                        tx_for_requeue.send(Some(message)).unwrap();
                                    }
                                    break;
                                }

                                if let Some(callback) = message.callback.as_ref() {
                                    callback.on_sent();
                                }
                            }
                            None => {
                                state.disconnect();
                                break;
                            }
                        }
                    }
                };

                let keepalive_loop = async {
                    while !state.is_must_broken() {
                        sleep(Duration::from_secs(5 as u64)).await;
                        if !state.need_send_keepalive() {
                            continue;
                        }
                        if let Err(e) = conn.send(String::from("{\"type\":\"nop\"}")).await {
                            warn!("keepalive_runner send failed: {:?}", e);
                            break;
                        }
                    }
                };

                let incoming_loop = async {
                    while let Some(req) = incoming_rx.recv().await {
                        match ChatRequestType::from(&req.r#type) {
                            ChatRequestType::Unknown(t) => {
                                warn!("websocket unknown message: {}", t);
                                continue;
                            }
                            ChatRequestType::Nop => {
                                continue;
                            }
                            ChatRequestType::Kickout => {
                                let reason = req.message.unwrap_or_default();
                                warn!("websocket kickout by other client: {}", reason);

                                state.disconnect();
                                callback.on_kickoff_by_other_client(reason);
                                break;
                            }
                            _ => {}
                        }
                        // lookup pending_request
                        store_ref.process_incoming(req).await;
                    }
                };

                select! {
                    _ = conn.serve(&opt, conn_inner) => {
                    },
                    _ = incoming_loop => {
                    },
                    _ = sender_loop => {
                    },
                    _ = keepalive_loop => {
                    }
                }
            }
        });
        info!("websocket disconnected");
    }

    pub async fn app_active(&self) {
        self.connect_now.lock().unwrap().take();
    }

    pub async fn app_deactivate(&self) {
        //TODO:
    }

    pub async fn shutdown(&self) {
        info!("shutdown websocket");
        if let Some(sender) = self.ws_sender.lock().unwrap().take() {
            sender.send(None).unwrap();
        }
    }
}
