use self::{
    connection::ConnectState,
    store::{ClientStore, ClientStoreRef},
};
use crate::{
    models::{AuthInfo, ChatLogStatus, GetChatLogsResult},
    services::conversation::get_chat_logs_desc,
    utils::now_timestamp,
    DB_SUFFIX,
};
use anyhow::Result;
use log::warn;
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
        let store = ClientStore::new(
            root_path,
            &db_path,
            &info.endpoint,
            &info.token,
            &info.user_id,
        );
        let store_ref = Arc::new(store);

        if let Err(e) = store_ref.migrate() {
            warn!("migrate database fail!! {:?}", e)
        }

        Self {
            root_path: root_path.to_string(),
            user_id: info.user_id.to_string(),
            token: info.token.to_string(),
            endpoint: info.endpoint.to_string(),
            store: store_ref,
            state: Arc::new(ConnectState::new()),
        }
    }

    pub async fn get_chat_logs(
        &self,
        topic_id: &str,
        last_seq: i64,
        limit: u32,
    ) -> Result<GetChatLogsResult> {
        match self.store.get_chat_logs(topic_id, last_seq, limit).await {
            Ok(local_logs) => {
                if local_logs.items.len() == limit as usize {
                    return Ok(GetChatLogsResult {
                        has_more: local_logs.end_sort_value > 1,
                        start_seq: local_logs.start_sort_value,
                        end_seq: local_logs.end_sort_value,
                        items: local_logs.items,
                    });
                }
            }
            Err(e) => {
                warn!("get_chat_logs failed: {:?}", e);
            }
        }

        let endpoint = self.endpoint.clone();
        let token = self.token.clone();

        let start_seq = (last_seq - limit as i64).max(0);
        let current_user_id = self.user_id.clone();

        get_chat_logs_desc(&endpoint, &token, topic_id, start_seq, limit)
            .await
            .map(|(mut lr, _)| {
                let now = now_timestamp();

                lr.items.iter_mut().for_each(|c| {
                    c.cached_at = now;
                    c.status = if c.sender_id == current_user_id {
                        ChatLogStatus::Sent
                    } else {
                        ChatLogStatus::Received
                    };
                    self.store.save_chat_log(c).ok();
                });
                lr
            })
            .map(|lr| {
                let start_seq = lr.items[0].seq;
                let end_seq = if lr.items.len() > 0 {
                    lr.items[lr.items.len() - 1].seq
                } else {
                    0
                };
                let r = GetChatLogsResult {
                    has_more: end_seq > 1,
                    start_seq,
                    end_seq,
                    items: lr.items,
                };
                r
            })
    }
}
