use self::{
    connection::ConnectState,
    store::{ClientStore, ClientStoreRef},
};
use crate::{models::AuthInfo, DB_SUFFIX};
use std::sync::Arc;

mod connection;
pub mod message;
mod store;
#[cfg(test)]
mod tests;

pub struct Client {
    pub root_path: String,
    pub user_id: String,
    pub token: String,
    pub endpoint: String,
    store: ClientStoreRef,
    state: Arc<ConnectState>,
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
        let store = ClientStore::new(root_path, &db_path, &info.endpoint, &info.token);
        let store_ref = Arc::new(store);

        Self {
            root_path: root_path.to_string(),
            user_id: info.user_id.to_string(),
            token: info.token.to_string(),
            endpoint: info.endpoint.to_string(),
            store: store_ref,
            state: Arc::new(ConnectState::new()),
        }
    }
}
