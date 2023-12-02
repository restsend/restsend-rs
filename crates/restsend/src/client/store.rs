use crate::client::connection::ConnectionStatus;
use log::warn;
use std::{
    collections::{HashMap, VecDeque},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Instant,
};
use tokio::{
    select,
    sync::{
        mpsc::{unbounded_channel, UnboundedSender},
        oneshot, watch,
    },
};

use crate::{
    callback::MessageCallback,
    request::{ChatRequest, ChatRequestType},
    MAX_RETRIES, MAX_SEND_IDLE_SECS,
};

pub struct PendingRequest {
    pub callback: Option<Box<dyn MessageCallback>>,
    pub req: ChatRequest,
    pub retry: AtomicUsize,
    pub updated_at: Instant,
    pub can_retry: bool,
}

impl PendingRequest {
    pub fn new(req: ChatRequest, callback: Option<Box<dyn MessageCallback>>) -> Self {
        let can_retry = match ChatRequestType::from(&req.r#type) {
            ChatRequestType::Typing | ChatRequestType::Read => false,
            _ => true,
        };

        PendingRequest {
            callback,
            req,
            retry: AtomicUsize::new(0),
            can_retry,
            updated_at: Instant::now(),
        }
    }

    pub fn is_expired(&self) -> bool {
        if !self.can_retry {
            return true;
        }
        let retry_count = self.retry.load(Ordering::Relaxed);
        retry_count >= MAX_RETRIES || self.updated_at.elapsed().as_secs() > MAX_SEND_IDLE_SECS
    }

    pub fn did_retry(&self) {
        self.retry.fetch_add(1, Ordering::Relaxed);
    }
}

type PendingRequests = Mutex<HashMap<String, PendingRequest>>;

pub(super) type ClientStoreRef = Arc<ClientStore>;
pub(super) struct ClientStore {
    tmps: Mutex<VecDeque<String>>,
    outgoings: PendingRequests,
    msg_tx: Mutex<Option<UnboundedSender<String>>>,
}

impl ClientStore {
    pub fn new(db_path: &str) -> Self {
        Self {
            tmps: Mutex::new(VecDeque::new()),
            outgoings: Mutex::new(HashMap::new()),
            msg_tx: Mutex::new(None),
        }
    }

    pub async fn process(&self) {}

    pub async fn handle_outgoing(&self, outgoing_tx: UnboundedSender<ChatRequest>) {
        let (msg_tx, mut msg_rx) = unbounded_channel::<String>();
        self.msg_tx.lock().unwrap().replace(msg_tx.clone());

        while let Some(req_id) = msg_rx.recv().await {
            let mut outgoings = self.outgoings.lock().unwrap();
            if let Some(pending) = outgoings.remove(&req_id) {
                if pending.is_expired() {
                    continue;
                }
                outgoing_tx.send(pending.req).ok();
                pending.callback.map(|cb| cb.on_sent());
            }
        }
    }

    pub async fn process_incoming(&self, req: ChatRequest) -> Option<ChatRequest> {
        warn!("process_incoming: {:?}", req);

        None
    }

    pub async fn handle_send_fail(&self, req_id: &str) {}
    pub async fn handle_send_success(&self, req_id: &str) {}

    pub async fn add_pending_request(
        &self,
        req: ChatRequest,
        callback: Option<Box<dyn MessageCallback>>,
    ) {
        let req_id = req.id.clone();
        self.outgoings
            .lock()
            .unwrap()
            .insert(req_id.clone(), PendingRequest::new(req, callback));

        let tx = self.msg_tx.lock().unwrap();
        match tx.as_ref() {
            Some(tx) => {
                tx.send(req_id).unwrap();
            }
            None => {
                let mut tmps = self.tmps.lock().unwrap();
                tmps.push_back(req_id);
            }
        }
    }

    pub async fn flush_offline_requests(&self) {
        let mut tmps = self.tmps.lock().unwrap();
        let tx = self.msg_tx.lock().unwrap();
        match tx.as_ref() {
            Some(tx) => {
                while let Some(req_id) = tmps.pop_front() {
                    tx.send(req_id).unwrap();
                }
            }
            None => {}
        }
    }

    pub async fn shutdown(&self) {}
}
