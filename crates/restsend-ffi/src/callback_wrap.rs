use restsend_sdk::{
    callback::{self},
    models::{Conversation, GetChatLogsResult, GetConversationsResult},
    request::ChatRequest,
    Error,
};

#[allow(unused_variables)]
#[uniffi::export(callback_interface)]
pub trait RSCallback: Send + Sync {
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
    fn on_conversations_removed(&self, conversatio_id: String) {}
}

pub(crate) struct CallbackWrap(pub(crate) Box<dyn RSCallback>);
impl callback::Callback for CallbackWrap {
    fn on_connected(&self) {
        self.0.on_connected()
    }
    fn on_connecting(&self) {
        self.0.on_connecting()
    }
    fn on_token_expired(&self, reason: String) {
        self.0.on_token_expired(reason)
    }
    fn on_net_broken(&self, reason: String) {
        self.0.on_net_broken(reason)
    }
    fn on_kickoff_by_other_client(&self, reason: String) {
        self.0.on_kickoff_by_other_client(reason)
    }

    fn on_system_request(&self, req: ChatRequest) -> Option<ChatRequest> {
        self.0.on_system_request(req)
    }
    fn on_unknown_request(&self, req: ChatRequest) -> Option<ChatRequest> {
        self.0.on_unknown_request(req)
    }
    fn on_topic_typing(&self, topic_id: String, message: Option<String>) {
        self.0.on_topic_typing(topic_id, message)
    }

    // if return true, will send `has read` to server
    fn on_new_message(&self, topic_id: String, message: ChatRequest) -> bool {
        self.0.on_new_message(topic_id, message)
    }

    fn on_topic_read(&self, topic_id: String, message: ChatRequest) {
        self.0.on_topic_read(topic_id, message)
    }

    fn on_conversations_updated(&self, conversations: Vec<Conversation>) {
        self.0.on_conversations_updated(conversations)
    }
    fn on_conversations_removed(&self, conversatio_id: String) {
        self.0.on_conversations_removed(conversatio_id)
    }
}

#[allow(unused_variables)]
#[uniffi::export(callback_interface)]
pub trait RSSyncConversationsCallback: Send + Sync {
    fn on_success(&self, r: GetConversationsResult);
    fn on_fail(&self, e: Error);
}

pub(crate) struct RSSyncConversationsCallbackWrap(pub(crate) Box<dyn RSSyncConversationsCallback>);
impl callback::SyncConversationsCallback for RSSyncConversationsCallbackWrap {
    fn on_success(&self, r: restsend_sdk::models::GetConversationsResult) {
        self.0.on_success(r)
    }
    fn on_fail(&self, e: Error) {
        self.0.on_fail(e)
    }
}

#[allow(unused_variables)]
#[uniffi::export(callback_interface)]
pub trait RSMessageCallback: Send + Sync {
    fn on_sent(&self);
    fn on_progress(&self, progress: u64, total: u64);
    fn on_ack(&self, req: ChatRequest);
    fn on_fail(&self, reason: String);
}

pub(crate) struct RSMessageCallbackWrap(pub(crate) Option<Box<dyn RSMessageCallback>>);
impl RSMessageCallbackWrap {}
impl callback::MessageCallback for RSMessageCallbackWrap {
    fn on_sent(&self) {
        self.0.as_ref().map(|t| t.on_sent());
    }
    fn on_progress(&self, progress: u64, total: u64) {
        self.0.as_ref().map(|t| t.on_progress(progress, total));
    }
    fn on_ack(&self, req: ChatRequest) {
        self.0.as_ref().map(|t| t.on_ack(req));
    }
    fn on_fail(&self, reason: String) {
        self.0.as_ref().map(|t| t.on_fail(reason));
    }
}

#[allow(unused_variables)]
#[uniffi::export(callback_interface)]
pub trait RSSyncChatLogsCallback: Send + Sync {
    fn on_success(&self, r: GetChatLogsResult);
    fn on_fail(&self, e: Error);
}

pub(crate) struct RSSyncChatLogsCallbackWrap(pub(crate) Box<dyn RSSyncChatLogsCallback>);
impl RSSyncChatLogsCallbackWrap {}
impl callback::SyncChatLogsCallback for RSSyncChatLogsCallbackWrap {
    fn on_success(&self, r: GetChatLogsResult) {
        self.0.on_success(r);
    }
    fn on_fail(&self, e: Error) {
        self.0.on_fail(e);
    }
}

#[allow(unused_variables)]
#[uniffi::export(callback_interface)]
pub trait RSDownloadCallback: Send + Sync {
    fn on_progress(&self, progress: u64, total: u64) {}
    fn on_success(&self, url: String, file_name: String) {}
    fn on_fail(&self, e: Error) {}
}

pub(crate) struct RSDownloadCallbackWrap(pub(crate) Box<dyn RSDownloadCallback>);
impl RSDownloadCallbackWrap {}
impl callback::DownloadCallback for RSDownloadCallbackWrap {
    fn on_progress(&self, progress: u64, total: u64) {
        self.0.on_progress(progress, total);
    }
    fn on_success(&self, url: String, file_name: String) {
        self.0.on_success(url, file_name);
    }
    fn on_fail(&self, e: Error) {
        self.0.on_fail(e);
    }
}
