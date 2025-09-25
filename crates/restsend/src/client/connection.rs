use super::{
    store::{CallbackRef, ClientStoreRef},
    Client,
};
use crate::{
    request::{ChatRequest, ChatRequestType},
    utils::{sleep, spawn_task},
    websocket::{WebSocket, WebSocketCallback, WebsocketOption},
    KEEPALIVE_INTERVAL_SECS, MAX_CONNECT_INTERVAL_SECS, PING_INTERVAL_SECS,
};
use log::{info, warn};
use restsend_macros::export_wasm_or_ffi;
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, AtomicI64, AtomicU32, AtomicU64, Ordering},
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
    Shutdowned,
}

impl ToString for ConnectionStatus {
    fn to_string(&self) -> String {
        match self {
            ConnectionStatus::Broken => "broken".to_string(),
            ConnectionStatus::ConnectNow => "connectNow".to_string(),
            ConnectionStatus::Connected => "connected".to_string(),
            ConnectionStatus::Connecting => "connecting".to_string(),
            ConnectionStatus::Shutdown => "shutdown".to_string(),
            ConnectionStatus::Shutdowned => "shutdowned".to_string(),
        }
    }
}

pub(super) struct ConnectState {
    must_shutdown: AtomicBool,
    broken_count: AtomicU64,
    keepalive_inerval_secs: AtomicU32,
    ping_interval_secs: AtomicU32,
    last_broken_at: Mutex<Option<i64>>,
    last_alive_at: AtomicI64,
    state_tx: broadcast::Sender<ConnectionStatus>,
    last_state: Mutex<ConnectionStatus>,
    error_collector: ErrorLogCollector,
}

pub(super) type ConnectStateRef = Arc<ConnectState>;

impl ConnectState {
    pub fn new() -> Self {
        let (state_tx, _) = broadcast::channel(32);
        Self {
            must_shutdown: AtomicBool::new(false),
            broken_count: AtomicU64::new(0),
            keepalive_inerval_secs: AtomicU32::new(KEEPALIVE_INTERVAL_SECS as u32),
            ping_interval_secs: AtomicU32::new(PING_INTERVAL_SECS as u32),
            last_broken_at: Mutex::new(None),
            last_alive_at: AtomicI64::new(crate::utils::now_millis()),
            state_tx,
            last_state: Mutex::new(ConnectionStatus::Shutdown),
            error_collector: ErrorLogCollector::new(100),
        }
    }

    pub fn did_shutdown(&self) {
        self.must_shutdown.store(true, Ordering::Relaxed);
        *self.last_state.lock().unwrap() = ConnectionStatus::Shutdown;
        self.state_tx.send(ConnectionStatus::Shutdown).ok();
    }

    pub fn did_connect_now(&self) {
        self.state_tx.send(ConnectionStatus::ConnectNow).ok();
    }

    pub fn is_must_shutdown(&self) -> bool {
        self.must_shutdown.load(Ordering::Relaxed)
    }

    pub fn did_connecting(&self) {
        self.last_alive_at
            .store(crate::utils::now_millis(), Ordering::Relaxed);
        *self.last_state.lock().unwrap() = ConnectionStatus::Connecting;
        //self.state_tx.send(ConnectionStatus::Connecting).ok();
    }

    pub fn did_connected(&self) {
        self.broken_count.store(0, Ordering::Relaxed);
        self.last_broken_at.lock().unwrap().take();
        self.last_alive_at
            .store(crate::utils::now_millis(), Ordering::Relaxed);
        *self.last_state.lock().unwrap() = ConnectionStatus::Connected;
    }

    pub fn did_sent_or_recvived(&self) {
        self.last_alive_at
            .store(crate::utils::now_millis(), Ordering::Relaxed);
    }

    pub fn did_broken(&self) {
        self.broken_count.fetch_add(1, Ordering::Relaxed);
        *self.last_broken_at.lock().unwrap() = Some(crate::utils::now_millis());
        *self.last_state.lock().unwrap() = ConnectionStatus::Broken;
        self.state_tx.send(ConnectionStatus::Broken).ok();
    }

    pub fn set_keepalive_interval_secs(&self, secs: u32) {
        self.keepalive_inerval_secs
            .store(secs as u32, Ordering::Relaxed);
        self.ping_interval_secs
            .store(secs as u32, Ordering::Relaxed);
    }

    pub fn get_ping_interval_secs(&self) -> u32 {
        self.ping_interval_secs.load(Ordering::Relaxed)
    }
    pub fn get_last_alive_at(&self) -> i64 {
        self.last_alive_at.load(Ordering::Relaxed)
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

    pub fn add_error(&self, error: String) {
        self.error_collector.add_error(error);
    }

    pub fn get_and_clear_errors(&self) -> Vec<String> {
        self.error_collector.get_and_clear()
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
            .read()
            .unwrap()
            .as_ref()
            .map(|cb| cb.on_connected());
        self.store_ref.flush_offline_requests();
    }

    fn on_connecting(&self) {
        self.callback_ref
            .read()
            .unwrap()
            .as_ref()
            .map(|cb| cb.on_connecting());
        self.connect_state_ref.did_connecting();
    }

    fn on_unauthorized(&self) {
        self.callback_ref
            .read()
            .unwrap()
            .as_ref()
            .map(|cb| cb.on_token_expired("unauthorized".to_string()));
    }

    fn on_net_broken(&self, reason: String) {
        self.connect_state_ref
            .add_error(format!("net_broken: {}", reason));
        self.connect_state_ref.did_broken();
        self.callback_ref
            .read()
            .unwrap()
            .as_ref()
            .map(|cb| cb.on_net_broken(reason));
    }

    fn on_message(&self, message: String) {
        self.connect_state_ref.did_sent_or_recvived();

        let req = match ChatRequest::try_from(message) {
            Ok(req) => req,
            Err(e) => {
                let error_msg = format!("websocket parse message error: {}", e);
                warn!("{}", error_msg);
                self.connect_state_ref.add_error(error_msg);
                return;
            }
        };

        match ChatRequestType::from(&req.req_type) {
            ChatRequestType::Nop => {
                return;
            }
            _ => match self.incoming_tx.send(req) {
                Ok(_) => {}
                Err(e) => {
                    let error_msg = format!("websocket send to incoming_tx failed: {}", e);
                    warn!("{}", error_msg);
                    self.connect_state_ref.add_error(error_msg);
                }
            },
        }
    }
}

#[derive(Clone)]
pub struct ErrorLogCollector {
    logs: Arc<Mutex<VecDeque<String>>>,
    max_logs: usize,
}

impl ErrorLogCollector {
    pub fn new(max_logs: usize) -> Self {
        Self {
            logs: Arc::new(Mutex::new(VecDeque::new())),
            max_logs,
        }
    }

    pub fn add_error(&self, error: String) {
        let mut logs = self.logs.lock().unwrap();
        logs.push_back(format!("{}: {}", crate::utils::now_millis(), error));
        while logs.len() > self.max_logs {
            logs.pop_front();
        }
    }

    pub fn get_and_clear(&self) -> Vec<String> {
        let mut logs = self.logs.lock().unwrap();
        let result = logs.iter().cloned().collect();
        logs.clear();
        result
    }
}

#[export_wasm_or_ffi]
impl Client {
    pub fn connection_status(&self) -> String {
        self.state.last_state.lock().unwrap().to_string()
    }

    pub async fn connect(&self) {
        let state_ref = self.state.clone();
        let store_ref = self.store.clone();
        let endpoint = self.endpoint.clone();
        let token = self.token.clone();
        let is_cross_domain = self.is_cross_domain;

        spawn_task(async move {
            serve_connection(&endpoint, &token, is_cross_domain, store_ref, state_ref).await;
            warn!("connection serve_connection done");
        });
    }

    pub fn app_active(&self) {
        self.state.did_connect_now();
    }

    pub fn set_keepalive_interval_secs(&self, secs: u32) {
        self.state.set_keepalive_interval_secs(secs);
    }

    pub fn get_last_alive_at(&self) -> i64 {
        self.state.get_last_alive_at()
    }

    pub async fn shutdown(&self) {
        info!("shutdown websocket");
        self.state.did_shutdown();
        self.store.shutdown();

        select! {
            _ = async {
                sleep(Duration::from_secs(1)).await;
            } => {}
            _ = async {
                loop {
                    let mut state_ref = self.state.state_tx.subscribe();
                    let st = state_ref.recv().await;
                    match st {
                        Ok(ConnectionStatus::Shutdowned)  => {
                            break;
                        }
                        _ => {}
                    }
                }
            } => {}
        };
    }
}

async fn serve_connection(
    endpoint: &str,
    token: &str,
    is_cross_domain: bool,
    store_ref: ClientStoreRef,
    state_ref: ConnectStateRef,
) {
    let callback_ref = store_ref.callback.clone();

    let conn_state_ref = state_ref.state_tx.clone();

    let conn_loop = async {
        while !state_ref.is_must_shutdown() {
            state_ref.wait_for_next_connect().await;

            let url = WebsocketOption::url_from_endpoint(endpoint);
            let opt = WebsocketOption::new(&url, token, is_cross_domain);
            info!("connect websocket url: {}", url);

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
                    if let Err(e) = conn.send((&message).into()).await {
                        warn!("send fail {:?}", e);
                        store_ref.handle_send_fail(&message.chat_id).await;
                        break;
                    }
                    state_ref.did_sent_or_recvived();
                }
            };

            let keepalive_loop = async {
                while !state_ref.is_must_shutdown() {
                    let ping_interval = state_ref.get_ping_interval_secs();
                    sleep(Duration::from_secs(ping_interval as u64)).await;

                    let timestamp = crate::utils::now_millis();
                    let error_logs = state_ref.get_and_clear_errors();
                    let ping_req = ChatRequest::new_ping(timestamp, error_logs);

                    if let Err(e) = conn.send((&ping_req).into()).await {
                        let error_msg = format!("ping send failed: {:?}", e);
                        warn!("{}", error_msg);
                        state_ref.add_error(error_msg);
                        break;
                    }
                    state_ref.did_sent_or_recvived();
                    info!(
                        "ping sent at {} with interval {}s",
                        timestamp, ping_interval
                    );
                }
            };

            let incoming_loop = async {
                while let Some(req) = incoming_rx.recv().await {
                    let resps = match ChatRequestType::from(&req.req_type) {
                        ChatRequestType::Nop => vec![],
                        ChatRequestType::Ping => {
                            vec![Some(ChatRequest::new_ping_response(
                                req.chat_id,
                                req.content,
                            ))]
                        }
                        ChatRequestType::Unknown(_) => {
                            let r = callback_ref
                                .read()
                                .unwrap()
                                .as_ref()
                                .map(|cb| cb.on_unknown_request(req).unwrap_or_default());
                            vec![r]
                        }
                        ChatRequestType::System => {
                            let r = callback_ref
                                .read()
                                .unwrap()
                                .as_ref()
                                .map(|cb| cb.on_system_request(req).unwrap_or_default());
                            vec![r]
                        }
                        ChatRequestType::Typing => {
                            callback_ref.read().unwrap().as_ref().map(|cb| {
                                cb.on_topic_typing(req.topic_id.clone(), req.message.clone())
                            });
                            vec![]
                        }
                        ChatRequestType::Kickout => {
                            let reason = req.message.unwrap_or_default();
                            warn!("websocket kickout by other client: {}", reason);

                            state_ref.did_shutdown();
                            callback_ref
                                .read()
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
                                warn!("websocket send failed: {:?}", e);
                                break;
                            }
                        }
                    }
                }
            };

            select! {
                _ = conn.serve(&opt, Box::new(conn_inner)) => {
                    warn!("connection serve done");
                },
                _ = store_ref.handle_outgoing(outgoing_tx) => {
                    warn!("connection handle_outgoing done");
                },
                _ = sender_loop => {
                    warn!("connection sender_loop done");
                },
                _ = incoming_loop => {
                    warn!("connection incoming_loop done");
                },
                _ = keepalive_loop => {
                    warn!("connection keepalive_loop done");
                }
            }
            state_ref.did_broken();
        }
    };

    select! {
        _ = async {
            loop {
                sleep(Duration::from_secs(1)).await;
                store_ref.process_timeout_requests();
                store_ref.process_removed_conversations();
            };
        } =>{
            warn!("connection shutdown timeout");
        }
        _ = conn_loop => {
            warn!("connection shutdown conn_loop");
        },
        _ = async {
            loop {
                let mut conn_state_rx = conn_state_ref.subscribe();
                let st = conn_state_rx.recv().await;
                match st {
                    Err(e) => {
                        warn!("connection shutdown conn_state_ref err {:?}", e);
                        break
                    }
                    Ok(ConnectionStatus::Shutdown)  => {
                        warn!("connection shutdown conn_state_ref");
                        break;
                    }
                    _ => {}
                }
            }
        } => {
            warn!("connection shutdown conn_state_ref");
        }
    };

    conn_state_ref.send(ConnectionStatus::Shutdowned).ok();
}
