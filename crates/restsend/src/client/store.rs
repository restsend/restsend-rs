use std::sync::Arc;

use log::warn;

use crate::request::ChatRequest;

pub(super) type ClientStoreRef = Arc<ClientStore>;
pub(super) struct ClientStore {}

impl ClientStore {
    pub fn new(db_path: &str) -> Self {
        Self {}
    }

    pub async fn process_incoming(&self, req: ChatRequest) -> Option<ChatRequest> {
        warn!("process_incoming: {:?}", req);
        None
    }
}
