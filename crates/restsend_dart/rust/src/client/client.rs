use std::{cell::RefCell, sync::{Arc, Mutex}};

use restsend_sdk::models::AuthInfo;

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}

pub fn set_logging(level: String) {
    restsend_sdk::utils::init_log(level, false);
}

pub struct Client {
    inner: Arc<restsend_sdk::client::Client>,
   pub(super) cb_on_connected: super::callback::CallbackOnConnected,
}

impl Client {
    pub fn new(info:AuthInfo, root_path:Option<String>, db_name: Option<String>,) -> Self {
        let inner = restsend_sdk::client::Client::new(root_path.unwrap_or_default(), db_name.unwrap_or_default(), &info);
        let cb_on_connected = Arc::new(Mutex::new(None));

        let cb_wrap = Box::new(super::callback::CallbackDartWrap {
            cb_on_connected: cb_on_connected.clone(),
        });
        inner.set_callback(Some(cb_wrap));
        let client = Client { inner, cb_on_connected};
        client
    }
    #[flutter_rust_bridge::frb(sync, getter)]
    pub fn connection_status(&self) -> String {
        self.inner.connection_status()
    } 
    /// connect immediately if the connection is broken    
    pub fn app_active(&self) {
        self.inner.app_active();
    }

    #[flutter_rust_bridge::frb(sync, setter)]
    pub fn set_keepalive(&self, secs: u32) {
        self.inner.set_keepalive_interval_secs(secs);
    }

    pub async fn shutdown(&self) {
        self.inner.shutdown().await
    }

    pub async fn connect(&self){
        self.inner.connect().await;
    }
}