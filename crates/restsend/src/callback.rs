use crate::{
    models::{Content, Conversation, GetChatLogsResult},
    request::ChatRequest,
    services::response::Upload,
    Error,
};
use restsend_macros::export_wasm_or_ffi;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[export_wasm_or_ffi(#[derive(uniffi::Record)])]
pub struct ChatRequestStatus {
    /// The message is read by the user
    pub has_read: bool,
    /// The message is not read count
    pub unread_countable: bool,
}

impl Default for ChatRequestStatus {
    fn default() -> Self {
        ChatRequestStatus {
            has_read: false,
            unread_countable: true,
        }
    }
}

#[allow(unused_variables)]
#[export_wasm_or_ffi(#[uniffi::export(callback_interface)])]
pub trait RsCallback: Send + Sync {
    fn on_connected(&self) {}
    fn on_connecting(&self) {}
    fn on_token_expired(&self, reason: String) {}
    fn on_net_broken(&self, reason: String) {}
    fn on_kickoff_by_other_client(&self, reason: String) {}

    fn on_system_request(&self, req: ChatRequest) -> Option<ChatRequest> {
        None
    }
    fn on_unknown_request(&self, req: ChatRequest) -> Option<ChatRequest> {
        None
    }
    fn on_topic_typing(&self, topic_id: String, message: Option<String>) {}

    fn on_new_message(&self, topic_id: String, message: ChatRequest) -> ChatRequestStatus {
        return ChatRequestStatus::default();
    }
    fn on_topic_read(&self, topic_id: String, message: ChatRequest) {}
    fn on_conversations_updated(&self, conversations: Vec<Conversation>, total: Option<i64>) {}
    fn on_conversation_removed(&self, conversation_id: String) {}
}
#[allow(unused_variables)]
#[export_wasm_or_ffi(#[uniffi::export(callback_interface)])]
pub trait CountableCallback: Send + Sync {
    fn is_countable(&self, content: Content) -> bool {
        !content.unreadable
    }
}
#[allow(unused_variables)]
#[export_wasm_or_ffi(#[uniffi::export(callback_interface)])]
pub trait UploadCallback: Send + Sync {
    fn on_progress(&self, progress: u64, total: u64) {}
    fn on_success(&self, result: Upload) {}
    fn on_fail(&self, e: Error) {}
}

#[allow(unused_variables)]
#[export_wasm_or_ffi(#[uniffi::export(callback_interface)])]
pub trait DownloadCallback: Send + Sync {
    fn on_progress(&self, progress: u64, total: u64) {}
    fn on_success(&self, url: String, file_name: String) {}
    fn on_fail(&self, e: Error) {}
}

#[allow(unused_variables)]
#[export_wasm_or_ffi(#[uniffi::export(callback_interface)])]
pub trait MessageCallback: Send + Sync {
    fn on_sent(&self, chat_id: String) {}
    fn on_progress(&self, progress: u64, total: u64) {}
    fn on_attachment_upload(&self, result: Upload) -> Option<Content> {
        None
    }
    fn on_ack(&self, req: ChatRequest) {}
    fn on_fail(&self, reason: String) {}
}

#[allow(unused_variables)]
#[export_wasm_or_ffi(#[uniffi::export(callback_interface)])]
pub trait SyncChatLogsCallback: Send + Sync {
    fn on_success(&self, r: GetChatLogsResult) {}
    fn on_fail(&self, e: Error) {}
}

#[allow(unused_variables)]
#[export_wasm_or_ffi(#[uniffi::export(callback_interface)])]
pub trait SyncConversationsCallback: Send + Sync {
    fn on_success(&self, updated_at: String, last_removed_at: Option<String>, count: u32) {}
    fn on_fail(&self, e: Error) {}
}
