use self::{
    connection::ConnectState,
    store::{ClientStore, ClientStoreRef},
};
use crate::{
    callback::DownloadCallback,
    error::ClientError,
    models::{AuthInfo, User},
    services::{
        media::{build_download_url, download_file},
        user::set_allow_guest_chat,
    },
    DB_SUFFIX, TEMP_FILENAME_LEN,
};
use crate::{utils, Result};
use log::warn;
use std::{path::Path, sync::Arc};
use tokio::sync::oneshot;
mod connection;
mod conversation;
mod message;
mod store;
#[cfg(test)]
mod tests;
mod topic;

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

    pub fn temp_path(root_path: &str, file_name: Option<String>) -> String {
        let mut file_name = file_name.unwrap_or_else(|| utils::random_text(TEMP_FILENAME_LEN));
        if file_name.contains("*") {
            file_name = file_name.replace("*", &utils::random_text(TEMP_FILENAME_LEN));
        }
        format!("{}/tmp/{}", root_path, file_name)
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

    pub fn get_user(&self, user_id: &str) -> Option<User> {
        self.store.get_user(user_id)
    }

    pub async fn set_user_remark(&self, user_id: &str, remark: &str) -> Result<()> {
        self.store.set_user_remark(user_id, remark).await
    }
    pub async fn set_user_star(&self, user_id: &str, star: bool) -> Result<()> {
        self.store.set_user_star(user_id, star).await
    }
    pub async fn set_user_block(&self, user_id: &str, block: bool) -> Result<()> {
        self.store.set_user_block(user_id, block).await
    }
    pub async fn set_allow_guest_chat(&self, allow: bool) -> Result<()> {
        set_allow_guest_chat(&self.endpoint, &self.token, allow).await
    }

    pub async fn download_file(
        &self,
        file_url: &str,
        callback: Box<dyn DownloadCallback>,
    ) -> Result<String> {
        let download_url = build_download_url(&self.endpoint, file_url);
        let file_ext = url::Url::parse(&download_url)
            .map_err(|e| {
                ClientError::HTTP(format!("download_file: url parse fail: {}", download_url))
            })
            .map(|u| {
                u.path_segments()
                    .unwrap()
                    .last()
                    .unwrap_or_default()
                    .split('.')
                    .last()
                    .unwrap_or_default()
                    .to_string()
            })?;

        let digest = md5::compute(&download_url);
        let file_name = format!("{:x}.{}", digest, file_ext);
        let save_file_name = Self::temp_path(&self.root_path, Some(file_name.to_string()));

        match std::fs::metadata(Path::new(&save_file_name)) {
            Ok(m) => {
                if m.is_file() && m.len() > 0 {
                    callback.on_success(download_url, save_file_name.clone());
                    return Ok(save_file_name);
                }
            }
            Err(_) => {}
        }

        #[allow(unused_variables)]
        let (cancel_tx, cancel_rx) = oneshot::channel();

        download_file(
            download_url,
            Some(self.token.clone()),
            save_file_name,
            callback,
            cancel_rx,
        )
        .await
    }
}
