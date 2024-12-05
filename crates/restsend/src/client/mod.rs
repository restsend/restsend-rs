use self::{
    connection::{ConnectState, ConnectStateRef},
    store::{ClientStore, ClientStoreRef},
};
use crate::{
    callback::{Callback, DownloadCallback},
    error::ClientError,
    media::{build_download_url, download_file},
    models::{AuthInfo, User},
    services::user::set_allow_guest_chat,
    DB_SUFFIX, TEMP_FILENAME_LEN,
};
use crate::{utils, Result};
use restsend_macros::export_wasm_or_ffi;
use std::{path::Path, sync::Arc};
use tokio::sync::oneshot;
mod connection;
mod conversation;
mod message;
mod store;
#[cfg(test)]
mod tests;
mod topic;
#[export_wasm_or_ffi(#[derive(uniffi::Object)])]
pub struct Client {
    pub root_path: String,
    pub user_id: String,
    pub token: String,
    pub endpoint: String,
    pub is_cross_domain: bool,

    store: ClientStoreRef,
    state: ConnectStateRef,
}

impl Client {
    pub fn db_path(root_path: &str, db_name: &str) -> String {
        if root_path.is_empty() && db_name.is_empty() {
            // for unit test
            "".to_string()
        } else {
            let root_path = if root_path.ends_with('/') {
                root_path.to_string()
            } else if !root_path.is_empty() {
                format!("{}/", root_path)
            } else {
                "".to_string()
            };
            format!("{}{}{}", root_path, db_name, DB_SUFFIX)
        }
    }

    pub fn temp_path(root_path: &str, file_name: Option<String>) -> String {
        let mut file_name = file_name.unwrap_or_else(|| utils::random_text(TEMP_FILENAME_LEN));
        if file_name.contains("*") {
            file_name = file_name.replace("*", &utils::random_text(TEMP_FILENAME_LEN));
        }
        format!("{}/tmp/{}", root_path, file_name)
    }
}

impl Client {
    pub fn new_sync(root_path: String, db_name: String, info: &AuthInfo) -> Self {
        let db_path = Self::db_path(&root_path, &db_name);
        let store = ClientStore::new(
            &root_path,
            &db_path,
            &info.endpoint,
            &info.token,
            &info.user_id,
        );

        Self {
            root_path: root_path.to_string(),
            user_id: info.user_id.to_string(),
            token: info.token.to_string(),
            endpoint: info.endpoint.to_string().trim_end_matches("/").to_string(),
            is_cross_domain: info.is_cross_domain,
            store: Arc::new(store),
            state: Arc::new(ConnectState::new()),
        }
    }
}

#[export_wasm_or_ffi]
impl Client {
    #[uniffi::constructor]
    pub fn new(root_path: String, db_name: String, info: &AuthInfo) -> Arc<Self> {
        Arc::new(Self::new_sync(root_path, db_name, info))
    }

    pub fn set_callback(&self, callback: Option<Box<dyn Callback>>) {
        *self.store.callback.lock().unwrap() = callback;
    }

    pub async fn get_user(&self, user_id: String, blocking: bool) -> Option<User> {
        self.store.get_user(&user_id, blocking).await
    }

    pub async fn get_users(&self, user_ids: Vec<String>) -> Vec<User> {
        self.store.get_users(user_ids).await
    }

    pub async fn set_user_remark(&self, user_id: String, remark: String) -> Result<()> {
        self.store.set_user_remark(&user_id, &remark).await
    }
    pub async fn set_user_star(&self, user_id: String, star: bool) -> Result<()> {
        self.store.set_user_star(&user_id, star).await
    }
    pub async fn set_user_block(&self, user_id: String, block: bool) -> Result<()> {
        self.store.set_user_block(&user_id, block).await
    }
    pub async fn set_allow_guest_chat(&self, allow: bool) -> Result<()> {
        set_allow_guest_chat(&self.endpoint, &self.token, allow).await
    }

    pub async fn download_file(
        &self,
        file_url: String,
        callback: Box<dyn DownloadCallback>,
    ) -> Result<String> {
        let download_url = build_download_url(&self.endpoint, &file_url);
        let file_ext = url::Url::parse(&download_url)
            .map_err(|_| {
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
