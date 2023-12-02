use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Instant,
};

use log::warn;

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
    pub(super) pendings: PendingRequests,
}

impl ClientStore {
    pub fn new(db_path: &str) -> Self {
        Self {
            pendings: Mutex::new(HashMap::new()),
        }
    }

    pub async fn process_incoming(&self, req: ChatRequest) -> Option<ChatRequest> {
        warn!("process_incoming: {:?}", req);
        None
    }
}
