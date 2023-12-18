use super::{
    store::{CallbackRef, ClientStoreRef},
    Client,
};
use crate::{
    callback::Callback,
    request::{ChatRequest, ChatRequestType},
    utils::{sleep, spwan_task},
    websocket::{WebSocket, WebSocketCallback, WebsocketOption},
    KEEPALIVE_INTERVAL_SECS, MAX_CONNECT_INTERVAL_SECS,
};
use log::{debug, info, warn};
use restsend_macros::export_wasm_or_ffi;
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tokio::{
    select,
    sync::{
        broadcast,
        mpsc::{unbounded_channel, UnboundedSender},
    },
};

#[derive(Clone)]
pub enum ConnectionStatus {
    Broken,
    ConnectNow,
    Connected,
    Connecting,
    Shutdown,
}

impl ToString for ConnectionStatus {
    fn to_string(&self) -> String {
        match self {
            ConnectionStatus::Broken => "broken".to_string(),
            ConnectionStatus::ConnectNow => "connect_now".to_string(),
            ConnectionStatus::Connected => "connected".to_string(),
            ConnectionStatus::Connecting => "connecting".to_string(),
            ConnectionStatus::Shutdown => "shutdown".to_string(),
        }
    }
}

pub(super) struct ConnectState {
    must_shutdown: AtomicBool,
    broken_count: AtomicU64,
    last_broken_at: Mutex<Option<i64>>,
    last_alive_at: Mutex<i64>,
    state_tx: broadcast::Sender<ConnectionStatus>,
    last_state: Mutex<ConnectionStatus>,
}

pub(super) type ConnectStateRef = Arc<ConnectState>;

impl ConnectState {
    pub fn new() -> Self {
        let (state_tx, _) = broadcast::channel(1);
        Self {
            must_shutdown: AtomicBool::new(false),
            broken_count: AtomicU64::new(0),
            last_broken_at: Mutex::new(None),
            last_alive_at: Mutex::new(crate::utils::now_millis()),
            state_tx,
            last_state: Mutex::new(ConnectionStatus::Shutdown),
        }
    }

    pub fn did_shutdown(&self) {
        self.must_shutdown.store(true, Ordering::Relaxed);
        *self.last_state.lock().unwrap() = ConnectionStatus::Shutdown;
        self.state_tx.send(ConnectionStatus::Shutdown).ok();
    }

    pub fn is_must_shutdown(&self) -> bool {
        self.must_shutdown.load(Ordering::Relaxed)
    }

    pub fn did_connecting(&self) {
        *self.last_alive_at.lock().unwrap() = crate::utils::now_millis();
        *self.last_state.lock().unwrap() = ConnectionStatus::Connecting;
        self.state_tx.send(ConnectionStatus::Connecting).ok();
    }

    pub fn did_connected(&self) {
        self.broken_count.store(0, Ordering::Relaxed);
        self.last_broken_at.lock().unwrap().take();
        *self.last_alive_at.lock().unwrap() = crate::utils::now_millis();
        *self.last_state.lock().unwrap() = ConnectionStatus::Connected;
        self.state_tx.send(ConnectionStatus::Connected).ok();
    }

    pub fn did_sent_or_recvived(&self) {
        *self.last_alive_at.lock().unwrap() = crate::utils::now_millis();
    }

    pub fn did_broken(&self) {
        self.broken_count.fetch_add(1, Ordering::Relaxed);
        *self.last_broken_at.lock().unwrap() = Some(crate::utils::now_millis());
        *self.last_state.lock().unwrap() = ConnectionStatus::Broken;
        self.state_tx.send(ConnectionStatus::Broken).ok();
    }

    pub fn need_send_keepalive(&self) -> bool {
        let last_alive_at = *self.last_alive_at.lock().unwrap();
        let elapsed = crate::utils::elapsed(last_alive_at).as_secs();
        elapsed >= KEEPALIVE_INTERVAL_SECS
    }

    pub async fn wait_for_next_connect(&self) {
        let broken_count = self.broken_count.load(Ordering::Relaxed);
        if broken_count <= 0 {
            return;
        }

        let remain_secs = broken_count.min(MAX_CONNECT_INTERVAL_SECS);
        let mut rx = self.state_tx.subscribe();

        select! {
            _ = sleep(Duration::from_secs(remain_secs)) => {
            },
            _ = rx.recv() => {
                self.broken_count.store(0, Ordering::Relaxed);
                self.last_broken_at.lock().unwrap().take();
            }
        }
    }
}

struct ConnectionInner {
    connect_state_ref: ConnectStateRef,
    store_ref: ClientStoreRef,
    callback_ref: CallbackRef,
    incoming_tx: UnboundedSender<ChatRequest>,
}

#[cfg(target_family = "wasm")]
unsafe impl Send for ConnectionInner {}
#[cfg(target_family = "wasm")]
unsafe impl Sync for ConnectionInner {}

impl WebSocketCallback for ConnectionInner {
    fn on_connected(&self, _usage: Duration) {
        self.connect_state_ref.did_connected();
        self.callback_ref
            .lock()
            .unwrap()
            .as_ref()
            .map(|cb| cb.on_connected());
        self.store_ref.flush_offline_requests();
    }

    fn on_connecting(&self) {
        self.callback_ref
            .lock()
            .unwrap()
            .as_ref()
            .map(|cb| cb.on_connecting());
        self.connect_state_ref.did_connecting();
    }

    fn on_unauthorized(&self) {
        self.callback_ref
            .lock()
            .unwrap()
            .as_ref()
            .map(|cb| cb.on_token_expired("unauthorized".to_string()));
    }

    fn on_net_broken(&self, reason: String) {
        self.connect_state_ref.did_broken();
        self.callback_ref
            .lock()
            .unwrap()
            .as_ref()
            .map(|cb| cb.on_net_broken(reason));
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
            ChatRequestType::Nop => {
                return;
            }
            _ => {
                self.incoming_tx.send(req).unwrap();
            }
        }
    }
}

#[export_wasm_or_ffi]
impl Client {
    pub fn connection_status(&self) -> String {
        self.state.last_state.lock().unwrap().to_string()
    }

    pub async fn connect(&self, callback: Box<dyn Callback>) {
        serve_connection(
            &self.endpoint,
            &self.token,
            self.store.clone(),
            self.state.clone(),
            callback,
        )
        .await;
    }

    pub fn app_active(&self) {
        self.state.state_tx.send(ConnectionStatus::ConnectNow).ok();
    }

    pub fn app_deactivate(&self) {
        warn!("app_deactivate not implemented")
    }

    pub fn shutdown(&self) {
        info!("shutdown websocket");
        self.state.did_shutdown();
        self.store.shutdown();
    }
}

async fn serve_connection(
    endpoint: &str,
    token: &str,
    store: ClientStoreRef,
    state: ConnectStateRef,
    callback: Box<dyn Callback>,
) {
    let url = WebsocketOption::url_from_endpoint(endpoint);
    let opt = WebsocketOption::new(&url, token);

    store.callback.lock().unwrap().replace(callback);

    let state_ref = state.clone();
    let store_ref = store.clone();
    let callback_ref = store_ref.callback.clone();

    info!("connect websocket url: {}", url);
    let conn_state_ref = state_ref.state_tx.clone();

    spwan_task(async move {
        let conn_loop = async {
            while !state_ref.is_must_shutdown() {
                state_ref.wait_for_next_connect().await;

                let (incoming_tx, mut incoming_rx) = unbounded_channel();

                let conn_inner = ConnectionInner {
                    connect_state_ref: state_ref.clone(),
                    callback_ref: callback_ref.clone(),
                    store_ref: store_ref.clone(),
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
                        state_ref.did_sent_or_recvived();
                        store_ref.handle_send_success(&message.id).await;
                    }
                };

                let keepalive_loop = async {
                    while !state_ref.is_must_shutdown() {
                        sleep(Duration::from_secs(5 as u64)).await;
                        if !state_ref.need_send_keepalive() {
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
                                let r = callback_ref
                                    .lock()
                                    .unwrap()
                                    .as_ref()
                                    .map(|cb| cb.on_unknown_request(req).unwrap_or_default());
                                vec![r]
                            }
                            ChatRequestType::System => vec![callback_ref
                                .lock()
                                .unwrap()
                                .as_ref()
                                .map(|cb| cb.on_system_request(req).unwrap_or_default())],
                            ChatRequestType::Typing => {
                                callback_ref.lock().unwrap().as_ref().map(|cb| {
                                    cb.on_topic_typing(req.topic_id.clone(), req.message.clone())
                                });
                                vec![]
                            }
                            ChatRequestType::Kickout => {
                                let reason = req.message.unwrap_or_default();
                                warn!("websocket kickout by other client: {}", reason);

                                state_ref.did_shutdown();
                                callback_ref
                                    .lock()
                                    .unwrap()
                                    .as_ref()
                                    .map(|cb| cb.on_kickoff_by_other_client(reason));
                                break;
                            }
                            _ => store_ref.process_incoming(req, callback_ref.clone()).await,
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
            _ = async {
                sleep(Duration::from_secs(1)).await;
                store_ref.process_timeout_requests();
            } =>{}
            _ = conn_loop => {
                warn!("connect shutdown");
            },
            _ = async {
                loop {
                    let mut conn_state_rx = conn_state_ref.subscribe();
                    let st = conn_state_rx.recv().await;
                    match st {
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
