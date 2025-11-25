use crate::frb_generated::{RustOpaque, StreamSink};
use restsend_sdk::{
    callback::{ChatRequestStatus, RsCallback, SyncChatLogsCallback, SyncConversationsCallback},
    client::Client,
    models::{
        conversation::{Extra, Tag},
        Attachment, AttachmentStatus, AuthInfo, ChatLog, ChatLogStatus, Content, Conversation,
        GetChatLogsResult,
    },
    request::ChatRequest,
    services::auth,
    storage::QueryResult,
    utils, Error as SdkError,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{atomic::Ordering, Arc, Mutex},
};
use thiserror::Error;
use tokio::sync::oneshot;
type BroadcastSender<T> = tokio::sync::broadcast::Sender<T>;
type BroadcastReceiver<T> = tokio::sync::broadcast::Receiver<T>;

#[derive(Debug, Error)]
pub enum RestsendDartError {
    #[error("{0}")]
    Client(String),
    #[error("stream closed before receiving event")]
    StreamClosed,
}

impl From<restsend_sdk::Error> for RestsendDartError {
    fn from(value: restsend_sdk::Error) -> Self {
        RestsendDartError::Client(value.to_string())
    }
}

pub type ExtraData = HashMap<String, String>;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentData {
    pub url: String,
    pub size: i64,
    pub thumbnail: String,
    pub file_name: String,
    pub file_path: String,
    pub url_or_data: String,
    pub is_private: bool,
    pub status: String,
}

impl From<Attachment> for AttachmentData {
    fn from(value: Attachment) -> Self {
        AttachmentData {
            url: value.url,
            size: value.size,
            thumbnail: value.thumbnail,
            file_name: value.file_name,
            file_path: value.file_path,
            url_or_data: value.url_or_data,
            is_private: value.is_private,
            status: attachment_status_to_string(value.status),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ContentData {
    #[serde(rename = "type")]
    pub content_type: String,
    pub encrypted: bool,
    pub checksum: u32,
    pub text: String,
    pub placeholder: String,
    pub thumbnail: String,
    pub duration: String,
    pub size: u64,
    pub width: f32,
    pub height: f32,
    pub mentions: Vec<String>,
    pub mention_all: bool,
    pub reply: String,
    pub reply_content: Option<String>,
    pub attachment: Option<AttachmentData>,
    pub extra: Option<ExtraData>,
    pub unreadable: bool,
}

impl From<Content> for ContentData {
    fn from(value: Content) -> Self {
        ContentData {
            content_type: value.content_type,
            encrypted: value.encrypted,
            checksum: value.checksum,
            text: value.text,
            placeholder: value.placeholder,
            thumbnail: value.thumbnail,
            duration: value.duration,
            size: value.size,
            width: value.width,
            height: value.height,
            mentions: value.mentions,
            mention_all: value.mention_all,
            reply: value.reply,
            reply_content: value.reply_content,
            attachment: value.attachment.map(AttachmentData::from),
            extra: extra_to_data(value.extra),
            unreadable: value.unreadable,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatLogData {
    pub topic_id: String,
    pub id: String,
    pub seq: i64,
    pub created_at: String,
    pub sender_id: String,
    pub content: ContentData,
    pub read: bool,
    pub recall: bool,
    pub status: String,
    pub cached_at: i64,
}

impl From<ChatLog> for ChatLogData {
    fn from(value: ChatLog) -> Self {
        ChatLogData {
            topic_id: value.topic_id,
            id: value.id,
            seq: value.seq,
            created_at: value.created_at,
            sender_id: value.sender_id,
            content: ContentData::from(value.content),
            read: value.read,
            recall: value.recall,
            status: chat_log_status_to_string(value.status),
            cached_at: value.cached_at,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TagData {
    pub id: String,
    #[serde(rename = "type")]
    pub tag_type: String,
    pub label: String,
}

impl From<Tag> for TagData {
    fn from(value: Tag) -> Self {
        TagData {
            id: value.id,
            tag_type: value.tag_type,
            label: value.label,
        }
    }
}

impl From<TagData> for Tag {
    fn from(value: TagData) -> Self {
        Tag {
            id: value.id,
            tag_type: value.tag_type,
            label: value.label,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChatRequestStatusData {
    pub has_read: bool,
    pub unread_countable: bool,
}

impl From<ChatRequestStatus> for ChatRequestStatusData {
    fn from(value: ChatRequestStatus) -> Self {
        ChatRequestStatusData {
            has_read: value.has_read,
            unread_countable: value.unread_countable,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationData {
    pub owner_id: String,
    pub topic_id: String,
    pub updated_at: String,
    pub start_seq: i64,
    pub last_seq: i64,
    pub last_read_seq: i64,
    pub last_read_at: Option<String>,
    pub multiple: bool,
    pub attendee: String,
    pub members: i64,
    pub name: String,
    pub icon: String,
    pub sticky: bool,
    pub mute: bool,
    pub source: String,
    pub unread: i64,
    pub last_sender_id: String,
    pub last_message: Option<ContentData>,
    pub last_message_at: String,
    pub last_message_seq: Option<i64>,
    pub remark: Option<String>,
    pub extra: Option<ExtraData>,
    pub topic_extra: Option<ExtraData>,
    pub topic_owner_id: Option<String>,
    pub topic_created_at: Option<String>,
    pub tags: Option<Vec<TagData>>,
    pub cached_at: i64,
    pub is_partial: bool,
}

impl From<Conversation> for ConversationData {
    fn from(value: Conversation) -> Self {
        ConversationData {
            owner_id: value.owner_id,
            topic_id: value.topic_id,
            updated_at: value.updated_at,
            start_seq: value.start_seq,
            last_seq: value.last_seq,
            last_read_seq: value.last_read_seq,
            last_read_at: value.last_read_at,
            multiple: value.multiple,
            attendee: value.attendee,
            members: value.members,
            name: value.name,
            icon: value.icon,
            sticky: value.sticky,
            mute: value.mute,
            source: value.source,
            unread: value.unread,
            last_sender_id: value.last_sender_id,
            last_message: value.last_message.map(ContentData::from),
            last_message_at: value.last_message_at,
            last_message_seq: value.last_message_seq,
            remark: value.remark,
            extra: extra_to_data(value.extra),
            topic_extra: extra_to_data(value.topic_extra),
            topic_owner_id: value.topic_owner_id,
            topic_created_at: value.topic_created_at,
            tags: value
                .tags
                .map(|v| v.into_iter().map(TagData::from).collect()),
            cached_at: value.cached_at,
            is_partial: value.is_partial,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationListPage {
    pub has_more: bool,
    pub start_sort_value: i64,
    pub end_sort_value: i64,
    pub items: Vec<ConversationData>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatLogsPage {
    pub has_more: bool,
    pub start_seq: i64,
    pub end_seq: i64,
    pub items: Vec<ChatLogData>,
}

impl From<QueryResult<Conversation>> for ConversationListPage {
    fn from(value: QueryResult<Conversation>) -> Self {
        ConversationListPage {
            has_more: value.has_more,
            start_sort_value: value.start_sort_value,
            end_sort_value: value.end_sort_value,
            items: value
                .items
                .into_iter()
                .map(ConversationData::from)
                .collect(),
        }
    }
}

impl From<GetChatLogsResult> for ChatLogsPage {
    fn from(value: GetChatLogsResult) -> Self {
        ChatLogsPage {
            has_more: value.has_more,
            start_seq: value.start_seq,
            end_seq: value.end_seq,
            items: value.items.into_iter().map(ChatLogData::from).collect(),
        }
    }
}

fn chat_logs_from_query(result: QueryResult<ChatLog>) -> ChatLogsPage {
    ChatLogsPage {
        has_more: result.has_more,
        start_seq: result.start_sort_value,
        end_seq: result.end_sort_value,
        items: result.items.into_iter().map(ChatLogData::from).collect(),
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SyncConversationsOptions {
    pub updated_at: Option<String>,
    pub before_updated_at: Option<String>,
    pub category: Option<String>,
    pub sync_max_count: Option<u32>,
    pub limit: Option<u32>,
    pub sync_logs: bool,
    pub sync_logs_limit: Option<u32>,
    pub sync_logs_max_count: Option<u32>,
    pub last_removed_at: Option<String>,
}

impl From<AttachmentData> for Attachment {
    fn from(value: AttachmentData) -> Self {
        Attachment {
            url: value.url,
            size: value.size,
            thumbnail: value.thumbnail,
            file_name: value.file_name,
            file_path: value.file_path,
            url_or_data: value.url_or_data,
            is_private: value.is_private,
            status: attachment_status_from_string(&value.status),
        }
    }
}

impl From<ContentData> for Content {
    fn from(value: ContentData) -> Self {
        let extra = extra_from_data(value.extra);
        Content {
            content_type: value.content_type,
            encrypted: value.encrypted,
            checksum: value.checksum,
            text: value.text,
            placeholder: value.placeholder,
            thumbnail: value.thumbnail,
            duration: value.duration,
            size: value.size,
            width: value.width,
            height: value.height,
            mentions: value.mentions,
            mention_all: value.mention_all,
            reply: value.reply,
            reply_content: value.reply_content,
            attachment: value.attachment.map(Attachment::from),
            extra,
            unreadable: value.unreadable,
            ..Content::default()
        }
    }
}

impl From<ChatLogData> for ChatLog {
    fn from(value: ChatLogData) -> Self {
        ChatLog {
            topic_id: value.topic_id,
            id: value.id,
            seq: value.seq,
            created_at: value.created_at,
            sender_id: value.sender_id,
            content: value.content.into(),
            read: value.read,
            recall: value.recall,
            status: chat_log_status_from_string(&value.status),
            cached_at: value.cached_at,
            ..ChatLog::default()
        }
    }
}

fn attachment_status_to_string(status: AttachmentStatus) -> String {
    match status {
        AttachmentStatus::ToUpload => "toUpload",
        AttachmentStatus::ToDownload => "toDownload",
        AttachmentStatus::Uploading => "uploading",
        AttachmentStatus::Downloading => "downloading",
        AttachmentStatus::Paused => "paused",
        AttachmentStatus::Done => "done",
        AttachmentStatus::Failed => "failed",
    }
    .to_string()
}

fn chat_log_status_to_string(status: ChatLogStatus) -> String {
    status.to_string()
}

fn attachment_status_from_string(status: &str) -> AttachmentStatus {
    match status {
        "toUpload" => AttachmentStatus::ToUpload,
        "toDownload" => AttachmentStatus::ToDownload,
        "uploading" => AttachmentStatus::Uploading,
        "downloading" => AttachmentStatus::Downloading,
        "paused" => AttachmentStatus::Paused,
        "done" => AttachmentStatus::Done,
        "failed" => AttachmentStatus::Failed,
        _ => AttachmentStatus::ToUpload,
    }
}

fn chat_log_status_from_string(status: &str) -> ChatLogStatus {
    match status {
        "uploading" => ChatLogStatus::Uploading,
        "sending" => ChatLogStatus::Sending,
        "sent" => ChatLogStatus::Sent,
        "downloading" => ChatLogStatus::Downloading,
        "received" => ChatLogStatus::Received,
        "read" => ChatLogStatus::Read,
        "sendFailed" => ChatLogStatus::SendFailed,
        _ => ChatLogStatus::Sending,
    }
}

fn extra_to_data(extra: Option<Extra>) -> Option<ExtraData> {
    extra
}

fn extra_from_data(extra: Option<ExtraData>) -> Option<Extra> {
    extra
}

#[derive(Clone)]
pub struct ClientHandle {
    inner: Arc<Client>,
    events: BroadcastSender<ClientEvent>,
}

impl ClientHandle {
    fn new(auth: AuthInfo, options: ClientOptions) -> Self {
        let root_path = options.root_path.unwrap_or_default();
        let db_name = options.db_name.unwrap_or_default();
        let client = Arc::new(Client::new_sync(root_path, db_name, &auth));
        let (events, _) = tokio::sync::broadcast::channel(128);
        client.set_callback(Some(Box::new(DartCallback {
            events: events.clone(),
        })));
        Self {
            inner: client,
            events,
        }
    }

    fn client(&self) -> &Client {
        self.inner.as_ref()
    }

    fn connection_status(&self) -> String {
        self.client().connection_status()
    }

    fn last_alive_at(&self) -> i64 {
        self.client().get_last_alive_at()
    }

    fn subscribe(&self) -> BroadcastReceiver<ClientEvent> {
        self.events.subscribe()
    }

    fn event_sender(&self) -> BroadcastSender<ClientEvent> {
        self.events.clone()
    }
}

unsafe impl Send for ClientHandle {}
unsafe impl Sync for ClientHandle {}

struct DartCallback {
    events: BroadcastSender<ClientEvent>,
}

impl RsCallback for DartCallback {
    fn on_connected(&self) {
        let _ = self.events.send(ClientEvent::Connected);
    }

    fn on_connecting(&self) {
        let _ = self.events.send(ClientEvent::Connecting);
    }

    fn on_token_expired(&self, reason: String) {
        let _ = self.events.send(ClientEvent::TokenExpired { reason });
    }

    fn on_net_broken(&self, reason: String) {
        let _ = self.events.send(ClientEvent::NetBroken { reason });
    }

    fn on_kickoff_by_other_client(&self, reason: String) {
        let _ = self
            .events
            .send(ClientEvent::KickedOffByOtherClient { reason });
    }

    fn on_conversations_updated(&self, conversations: Vec<Conversation>, total: Option<i64>) {
        let items = conversations
            .into_iter()
            .map(ConversationData::from)
            .collect();
        let _ = self
            .events
            .send(ClientEvent::ConversationsUpdated { items, total });
    }

    fn on_conversation_removed(&self, conversation_id: String) {
        let _ = self.events.send(ClientEvent::ConversationRemoved {
            topic_id: conversation_id,
        });
    }

    fn on_new_message(&self, topic_id: String, message: ChatRequest) -> ChatRequestStatus {
        let chat_log: ChatLog = ChatLog::from(&message);
        let status = ChatRequestStatus::default();
        let _ = self.events.send(ClientEvent::MessageReceived {
            topic_id,
            message: chat_log.into(),
            status: status.clone().into(),
        });
        status
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ClientEvent {
    Connected,
    Connecting,
    TokenExpired {
        reason: String,
    },
    NetBroken {
        reason: String,
    },
    KickedOffByOtherClient {
        reason: String,
    },
    ConversationsUpdated {
        items: Vec<ConversationData>,
        total: Option<i64>,
    },
    ConversationRemoved {
        topic_id: String,
    },
    MessageReceived {
        topic_id: String,
        message: ChatLogData,
        status: ChatRequestStatusData,
    },
    SyncConversationsProgress {
        updated_at: String,
        last_removed_at: Option<String>,
        count: u32,
        total: u32,
    },
    SyncConversationsFailed {
        message: String,
    },
}

struct EventSyncConversationsCallback {
    events: BroadcastSender<ClientEvent>,
}

impl SyncConversationsCallback for EventSyncConversationsCallback {
    fn on_success(
        &self,
        updated_at: String,
        last_removed_at: Option<String>,
        count: u32,
        total: u32,
    ) {
        let _ = self.events.send(ClientEvent::SyncConversationsProgress {
            updated_at,
            last_removed_at,
            count,
            total,
        });
    }

    fn on_fail(&self, e: SdkError) {
        let _ = self.events.send(ClientEvent::SyncConversationsFailed {
            message: e.to_string(),
        });
    }
}

struct OneShotChatLogsCallback {
    sender: Mutex<Option<oneshot::Sender<Result<GetChatLogsResult, SdkError>>>>,
}

impl OneShotChatLogsCallback {
    fn new(sender: oneshot::Sender<Result<GetChatLogsResult, SdkError>>) -> Self {
        Self {
            sender: Mutex::new(Some(sender)),
        }
    }

    fn complete(&self, value: Result<GetChatLogsResult, SdkError>) {
        if let Some(tx) = self.sender.lock().unwrap().take() {
            let _ = tx.send(value);
        }
    }
}

impl SyncChatLogsCallback for OneShotChatLogsCallback {
    fn on_success(&self, r: GetChatLogsResult) {
        self.complete(Ok(r));
    }

    fn on_fail(&self, e: SdkError) {
        self.complete(Err(e));
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DartAuthInfo {
    pub endpoint: String,
    pub user_id: String,
    pub token: String,
    pub name: Option<String>,
    pub avatar: Option<String>,
    #[serde(default)]
    pub is_staff: bool,
    #[serde(default)]
    pub is_cross_domain: bool,
}

impl From<DartAuthInfo> for AuthInfo {
    fn from(value: DartAuthInfo) -> Self {
        AuthInfo {
            endpoint: value.endpoint,
            user_id: value.user_id,
            token: value.token,
            name: value.name.unwrap_or_default(),
            avatar: value.avatar.unwrap_or_default(),
            is_staff: value.is_staff,
            is_cross_domain: value.is_cross_domain,
            private_extra: None,
        }
    }
}

impl From<AuthInfo> for DartAuthInfo {
    fn from(value: AuthInfo) -> Self {
        DartAuthInfo {
            endpoint: value.endpoint,
            user_id: value.user_id,
            token: value.token,
            name: if value.name.is_empty() {
                None
            } else {
                Some(value.name)
            },
            avatar: if value.avatar.is_empty() {
                None
            } else {
                Some(value.avatar)
            },
            is_staff: value.is_staff,
            is_cross_domain: value.is_cross_domain,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub db_name: Option<String>,
}

pub async fn login_with_password(
    endpoint: String,
    user_id: String,
    password: String,
) -> Result<DartAuthInfo, RestsendDartError> {
    let info = auth::login_with_password(endpoint, user_id, password).await?;
    Ok(info.into())
}

pub fn create_client(
    auth: DartAuthInfo,
    options: Option<ClientOptions>,
) -> Result<RustOpaque<ClientHandle>, RestsendDartError> {
    let handle = ClientHandle::new(auth.into(), options.unwrap_or_default());
    Ok(RustOpaque::new(handle))
}

pub async fn connect_client(client: RustOpaque<ClientHandle>) -> Result<(), RestsendDartError> {
    client.client().connect().await;
    Ok(())
}

pub async fn shutdown_client(client: RustOpaque<ClientHandle>) -> Result<(), RestsendDartError> {
    client.client().shutdown().await;
    Ok(())
}

pub fn get_connection_status(client: RustOpaque<ClientHandle>) -> String {
    client.connection_status()
}

pub fn get_last_alive_at(client: RustOpaque<ClientHandle>) -> i64 {
    client.last_alive_at()
}

pub fn app_active(client: RustOpaque<ClientHandle>) {
    client.client().app_active();
}

pub fn set_keepalive_interval(client: RustOpaque<ClientHandle>, secs: u32) {
    client.client().set_keepalive_interval_secs(secs);
}

pub fn set_ping_interval(client: RustOpaque<ClientHandle>, secs: u32) {
    client
        .client()
        .store
        .option
        .ping_interval_secs
        .store(secs as usize, Ordering::Relaxed);
}

pub fn set_max_retry(client: RustOpaque<ClientHandle>, count: u32) {
    client
        .client()
        .store
        .option
        .max_retry
        .store(count as usize, Ordering::Relaxed);
}

pub async fn get_unread_count(client: RustOpaque<ClientHandle>) -> Result<u32, RestsendDartError> {
    Ok(client.client().get_unread_count().await)
}

pub fn listen_client_events(
    client: RustOpaque<ClientHandle>,
    sink: StreamSink<ClientEvent>,
) -> Result<(), RestsendDartError> {
    let mut rx = client.subscribe();
    utils::spawn_task(async move {
        while let Ok(event) = rx.recv().await {
            if sink.add(event.clone()).is_err() {
                break;
            }
        }
    });
    Ok(())
}

pub async fn list_conversations(
    client: RustOpaque<ClientHandle>,
    updated_at: Option<String>,
    limit: u32,
) -> Result<ConversationListPage, RestsendDartError> {
    let updated_at = updated_at.unwrap_or_default();
    let page = client
        .client()
        .store
        .get_conversations(&updated_at, limit)
        .await?;
    Ok(page.into())
}

pub async fn sync_conversations(
    client: RustOpaque<ClientHandle>,
    options: Option<SyncConversationsOptions>,
) -> Result<(), RestsendDartError> {
    let opts = options.unwrap_or_default();
    let limit = opts.limit.unwrap_or(100);
    let callback = Box::new(EventSyncConversationsCallback {
        events: client.event_sender(),
    });
    client
        .client()
        .sync_conversations(
            opts.updated_at,
            opts.before_updated_at,
            opts.category,
            opts.sync_max_count,
            limit,
            opts.sync_logs,
            opts.sync_logs_limit,
            opts.sync_logs_max_count,
            opts.last_removed_at,
            callback,
        )
        .await;
    Ok(())
}

pub async fn get_chat_logs_local(
    client: RustOpaque<ClientHandle>,
    topic_id: String,
    start_seq: i64,
    end_seq: Option<i64>,
    limit: u32,
) -> Result<ChatLogsPage, RestsendDartError> {
    let (result, _) = client
        .client()
        .store
        .get_chat_logs(&topic_id, start_seq, end_seq, limit)
        .await?;
    Ok(chat_logs_from_query(result))
}

pub async fn sync_chat_logs(
    client: RustOpaque<ClientHandle>,
    topic_id: String,
    last_seq: Option<i64>,
    limit: u32,
    heavy: bool,
    ensure_conversation_last_version: Option<bool>,
) -> Result<ChatLogsPage, RestsendDartError> {
    let (tx, rx) = oneshot::channel();
    let callback = Box::new(OneShotChatLogsCallback::new(tx));

    if heavy {
        client
            .client()
            .sync_chat_logs_heavy(
                topic_id.clone(),
                last_seq,
                limit,
                callback,
                ensure_conversation_last_version,
            )
            .await;
    } else {
        client
            .client()
            .sync_chat_logs_quick(
                topic_id.clone(),
                last_seq,
                limit,
                callback,
                ensure_conversation_last_version,
            )
            .await;
    }

    match rx.await {
        Ok(Ok(result)) => Ok(result.into()),
        Ok(Err(e)) => Err(e.into()),
        Err(_) => Err(RestsendDartError::Client(
            "sync chat logs canceled".to_string(),
        )),
    }
}

pub async fn create_chat(
    client: RustOpaque<ClientHandle>,
    user_id: String,
) -> Result<ConversationData, RestsendDartError> {
    let conversation = client.client().create_chat(user_id).await?;
    Ok(conversation.into())
}

pub async fn clean_messages(
    client: RustOpaque<ClientHandle>,
    topic_id: String,
) -> Result<(), RestsendDartError> {
    client.client().clean_messages(topic_id).await?;
    Ok(())
}

pub async fn remove_messages(
    client: RustOpaque<ClientHandle>,
    topic_id: String,
    chat_ids: Vec<String>,
    sync_to_server: bool,
) -> Result<(), RestsendDartError> {
    client
        .client()
        .remove_messages(topic_id, chat_ids, sync_to_server)
        .await?;
    Ok(())
}

pub async fn get_chat_log(
    client: RustOpaque<ClientHandle>,
    topic_id: String,
    chat_id: String,
) -> Option<ChatLogData> {
    client
        .client()
        .get_chat_log(topic_id, chat_id)
        .await
        .map(ChatLogData::from)
}

pub async fn save_chat_logs(
    client: RustOpaque<ClientHandle>,
    logs: Vec<ChatLogData>,
) -> Result<(), RestsendDartError> {
    let rust_logs: Vec<ChatLog> = logs.into_iter().map(ChatLog::from).collect();
    client.client().save_chat_logs(&rust_logs).await?;
    Ok(())
}

pub async fn get_conversation(
    client: RustOpaque<ClientHandle>,
    topic_id: String,
) -> Option<ConversationData> {
    client
        .client()
        .get_conversation(topic_id)
        .await
        .map(ConversationData::from)
}

pub async fn remove_conversation(client: RustOpaque<ClientHandle>, topic_id: String) {
    client.client().remove_conversation(topic_id).await
}

pub async fn set_conversation_remark(
    client: RustOpaque<ClientHandle>,
    topic_id: String,
    remark: Option<String>,
) -> Result<ConversationData, RestsendDartError> {
    let conversation = client
        .client()
        .set_conversation_remark(topic_id, remark)
        .await?;
    Ok(conversation.into())
}

pub async fn set_conversation_sticky(
    client: RustOpaque<ClientHandle>,
    topic_id: String,
    sticky: bool,
) -> Result<ConversationData, RestsendDartError> {
    let conversation = client
        .client()
        .set_conversation_sticky(topic_id, sticky)
        .await?;
    Ok(conversation.into())
}

pub async fn set_conversation_mute(
    client: RustOpaque<ClientHandle>,
    topic_id: String,
    mute: bool,
) -> Result<ConversationData, RestsendDartError> {
    let conversation = client
        .client()
        .set_conversation_mute(topic_id, mute)
        .await?;
    Ok(conversation.into())
}

pub async fn set_conversation_read(
    client: RustOpaque<ClientHandle>,
    topic_id: String,
    heavy: bool,
) -> Result<(), RestsendDartError> {
    client.client().set_conversation_read(topic_id, heavy).await;
    Ok(())
}

pub async fn set_all_conversations_read(
    client: RustOpaque<ClientHandle>,
) -> Result<(), RestsendDartError> {
    client.client().set_all_conversations_read().await;
    Ok(())
}

pub async fn set_conversation_tags(
    client: RustOpaque<ClientHandle>,
    topic_id: String,
    tags: Option<Vec<TagData>>,
) -> Result<ConversationData, RestsendDartError> {
    let tags = tags.map(|items| items.into_iter().map(Tag::from).collect());
    let conversation = client
        .client()
        .set_conversation_tags(topic_id, tags)
        .await?;
    Ok(conversation.into())
}

pub async fn set_conversation_extra(
    client: RustOpaque<ClientHandle>,
    topic_id: String,
    extra: Option<ExtraData>,
) -> Result<ConversationData, RestsendDartError> {
    let conversation = client
        .client()
        .set_conversation_extra(topic_id, extra_from_data(extra))
        .await?;
    Ok(conversation.into())
}

pub async fn clear_conversation(
    client: RustOpaque<ClientHandle>,
    topic_id: String,
) -> Result<(), RestsendDartError> {
    client.client().clear_conversation(topic_id).await?;
    Ok(())
}

pub async fn send_text_message(
    client: RustOpaque<ClientHandle>,
    topic_id: String,
    text: String,
    mentions: Option<Vec<String>>,
    reply_to: Option<String>,
) -> Result<String, RestsendDartError> {
    let chat_id = client
        .client()
        .do_send_text(topic_id, text, mentions, reply_to, None)
        .await?;
    Ok(chat_id)
}

pub async fn send_custom_message(
    client: RustOpaque<ClientHandle>,
    topic_id: String,
    content: ContentData,
) -> Result<String, RestsendDartError> {
    let chat_id = client
        .client()
        .do_send(topic_id, content.into(), None)
        .await?;
    Ok(chat_id)
}

pub async fn recall_message(
    client: RustOpaque<ClientHandle>,
    topic_id: String,
    chat_id: String,
) -> Result<String, RestsendDartError> {
    let recalled = client.client().do_recall(topic_id, chat_id, None).await?;
    Ok(recalled)
}

pub async fn send_typing(
    client: RustOpaque<ClientHandle>,
    topic_id: String,
) -> Result<(), RestsendDartError> {
    client.client().do_typing(topic_id).await?;
    Ok(())
}

pub async fn send_read_receipt(
    client: RustOpaque<ClientHandle>,
    topic_id: String,
) -> Result<(), RestsendDartError> {
    client.client().do_read(topic_id).await?;
    Ok(())
}

pub async fn update_message_extra(
    client: RustOpaque<ClientHandle>,
    topic_id: String,
    chat_id: String,
    extra: Option<ExtraData>,
) -> Result<String, RestsendDartError> {
    let chat_id = client
        .client()
        .do_update_extra(topic_id, chat_id, extra_from_data(extra), None)
        .await?;
    Ok(chat_id)
}

pub async fn send_ping(
    client: RustOpaque<ClientHandle>,
    content: String,
) -> Result<String, RestsendDartError> {
    let chat_id = client.client().do_ping(content, None).await?;
    Ok(chat_id)
}
