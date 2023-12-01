use std::sync::{Arc, Mutex};

use self::{
    attachment::AttachmentInner,
    store::{ClientStore, ClientStoreRef},
};
use crate::{models::AuthInfo, DB_SUFFIX};
use tokio::sync::{mpsc, oneshot};
pub mod attachment;
mod connection;
pub mod message;
mod store;
#[cfg(test)]
mod tests;

type WSMessage = Option<String>;
type WSSender = Option<mpsc::UnboundedSender<WSMessage>>;

pub struct Client {
    pub root_path: String,
    pub user_id: String,
    pub token: String,
    pub endpoint: String,
    ws_connect_now: Arc<Mutex<Option<oneshot::Sender<()>>>>,
    ws_sender: Mutex<WSSender>,
    attachment_inner: AttachmentInner,
    store: ClientStoreRef,
}

impl Client {
    pub fn db_path(root_path: &str, db_name: &str) -> String {
        if root_path.is_empty() && db_name.is_empty() {
            // for unit test
            "".to_string()
        } else {
            format!("{}/{}{}", root_path, db_name, DB_SUFFIX)
        }
    }

    pub fn new(root_path: &str, db_name: &str, info: &AuthInfo) -> Self {
        let db_path = Self::db_path(root_path, db_name);
        let store = ClientStore::new(&db_path);
        let store_ref = Arc::new(store);

        Self {
            root_path: root_path.to_string(),
            user_id: info.user_id.to_string(),
            token: info.token.to_string(),
            endpoint: info.endpoint.to_string(),
            ws_sender: Mutex::new(None),
            attachment_inner: AttachmentInner::new(),
            store: store_ref,
            ws_connect_now: Arc::new(Mutex::new(None)),
        }
    }
}
