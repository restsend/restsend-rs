use crate::{
    models::{Conversation, GetChatLogsResult},
    request::ChatRequest,
    services::response::Upload,
    Error,
};
use restsend_macros::export_wasm_or_ffi;

#[allow(unused_variables)]
#[export_wasm_or_ffi(#[uniffi::export(callback_interface)])]
pub trait Callback: Send + Sync {
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

    // if return true, will send `has read` to server
    fn on_new_message(&self, topic_id: String, message: ChatRequest) -> bool {
        return false;
    }
    fn on_topic_read(&self, topic_id: String, message: ChatRequest) {}
    fn on_conversations_updated(&self, conversations: Vec<Conversation>) {}
    fn on_conversation_removed(&self, conversatio_id: String) {}
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
    fn on_sent(&self) {}
    fn on_progress(&self, progress: u64, total: u64) {}
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
    fn on_success(&self, updated_at: String, count: u32) {}
    fn on_fail(&self, e: Error) {}
}
