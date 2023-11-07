use crate::Uploader;
/*
    client 是对外的接口，对外提供了一些接口，比如登录，发送消息，接收消息等
    - client 内部会启动一个线程，用来处理websocket下发的消息，并且对需要发往服务端的数据进行排队
      1. 网络链接是异步的，如果消息发送失败了，会自动重试
      2. 如果网络断开了，会自动重连， 自动重连有次数上线，最后会到1分钟一次，如果app切换到前台(client.app_active), 那么就会立即重连
      3. app退出之前要调用client.shutdown，才会退出run_loop

    - 需要同步的数据：会话列表、联系人和群聊里面的消息

*/
use crate::models::DBStore;
use crate::net::NetStore;
use crate::request::PendingRequest;
use crate::Callback;
use crate::Result;

pub mod connection;
pub mod media;
pub mod message;
pub mod services;

use log::warn;
use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, RwLock};
use tokio::sync::mpsc;

const THREAD_NUM: usize = 2;

const KEY_USER_ID: &str = "user_id";
const KEY_CONVERSATIONS_SYNC_AT: &str = "conversations_sync_at";

const KEY_TOKEN: &str = "token";
const KEY_TOPICS_KNOCK_COUNT: &str = "topic_knock_count";

const LIMIT: u32 = 100;

const MEDIA_TIMEOUT_SECS: u64 = 300; // 5 minutes
const API_TIMEOUT_SECS: u64 = 60;

const CONVERSATIONS_SYNC: i64 = 3600;
const CONVERSATION_CACHE: i64 = 30;
const USER_CACHE: i64 = 30;

// websocket
const CONNECT_TIMEOUT_SECS: u64 = 30;
const REQUEST_TIMEOUT_SECS: u64 = 120;
const REQUEST_RETRY_TIMES: usize = 3;
const KEEPALIVE_INTERVAL: u64 = 30;

#[allow(dead_code)]
pub(crate) enum CtrlMessageType {
    Activate,
    Deactivate,
    Connect,
    ProcessTimeoutRequests,
    Reconnect,
    WebSocketConnected,
    WebSocketMessage(String),
    WebSocketClose(String),
    WebsocketError(String),
    // Media
    MediaUpload(String, String, String, bool),
    MediaDownload(String, String, String),
    MediaCancelDownload(String, String),
    MediaCancelUpload(String, String),
    // Media
    OnMediaDownloadProgress(String, u32, u32, String),
    OnMediaDownloadCancel(String, String, String, String),
    OnMediaDownloadDone(String, String, u32, String),

    OnMediaUploadProgress(String, u32, u32, String),
    OnMediaUploadCancel(String, String, String, String),
    OnMediaUploadDone(String, String, u32, String),

    Shutdown,
}
type CtrlReceiver = mpsc::UnboundedReceiver<CtrlMessageType>;
type CtrlSender = mpsc::UnboundedSender<CtrlMessageType>;

type MediaCancelSender = mpsc::UnboundedSender<bool>;

pub struct Client {
    pub(crate) runtime: tokio::runtime::Runtime,
    pub(crate) net_store: NetStore,
    pub(crate) db: DBStore,
    ws_tx: RwLock<Option<mpsc::UnboundedSender<String>>>,
    callback: RwLock<Option<Box<dyn Callback>>>,
    external_uploader: Mutex<Option<Box<dyn Uploader>>>,
    pending_queue: Mutex<VecDeque<PendingRequest>>,
    ctrl_rx: Mutex<CtrlReceiver>,
    ctrl_tx: CtrlSender,
    pending_medias: Mutex<HashMap<String, MediaCancelSender>>,
}

impl Client {
    pub fn new(db_name: String, endpoint: String) -> Self {
        let (ctrl_tx, ctrl_rx) = mpsc::unbounded_channel::<CtrlMessageType>();
        Client {
            runtime: tokio::runtime::Builder::new_multi_thread()
                .worker_threads(THREAD_NUM)
                .enable_all()
                .build()
                .unwrap(),
            pending_queue: Mutex::new(VecDeque::new()),
            pending_medias: Mutex::new(HashMap::new()),
            net_store: NetStore::new(endpoint),
            db: DBStore::new(&db_name),
            ws_tx: RwLock::new(None),
            callback: RwLock::new(None),
            external_uploader: Mutex::new(None),
            ctrl_tx,
            ctrl_rx: Mutex::new(ctrl_rx),
        }
    }

    pub fn prepare(&self) -> Result<()> {
        warn!("init client");
        self.db.prepare()?;
        Ok(())
    }

    pub fn set_callback(&self, callback: Option<Box<dyn Callback>>) {
        *self.callback.write().unwrap() = callback;
    }
    pub fn set_uploader(&self, uploader: Option<Box<dyn Uploader>>) {
        *self.external_uploader.lock().unwrap() = uploader;
    }
}
