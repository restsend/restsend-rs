uniffi::setup_scaffolding!();

use restsend_sdk::{client, models::AuthInfo};
use std::sync::Arc;
pub mod account;
pub mod callback_wrap;

#[derive(uniffi::Object)]
pub struct RsClient(client::Client);

#[uniffi::export]
impl RsClient {
    #[uniffi::constructor]
    pub fn new(root_path: String, db_name: String, info: AuthInfo) -> Arc<Self> {
        Arc::new(Self {
            0: client::Client::new(&root_path, &db_name, &info),
        })
    }

    pub async fn connect(self: Arc<Self>, callback: Box<dyn callback_wrap::RSCallback>) {
        self.0
            .connect(Box::new(callback_wrap::CallbackWrap { 0: callback }))
            .await
    }

    pub fn app_active(self: Arc<Self>) {
        self.0.app_active()
    }

    pub fn app_deactivate(self: Arc<Self>) {
        self.0.app_deactivate()
    }

    pub fn shutdown(self: Arc<Self>) {
        self.0.shutdown()
    }

    pub fn get_conversation(
        self: Arc<Self>,
        topic_id: String,
    ) -> Option<restsend_sdk::models::Conversation> {
        self.0.get_conversation(&topic_id)
    }

    // remove conversation with local  and server, not clean local chat logs
    pub fn remove_conversation(self: Arc<Self>, topic_id: String) {
        self.0.remove_conversation(&topic_id)
    }

    pub fn set_conversation_sticky(self: Arc<Self>, topic_id: String, sticky: bool) {
        self.0.set_conversation_sticky(&topic_id, sticky)
    }

    pub fn set_conversation_mute(self: Arc<Self>, topic_id: String, mute: bool) {
        self.0.set_conversation_mute(&topic_id, mute)
    }

    pub fn set_conversation_read(self: Arc<Self>, topic_id: String) {
        self.0.set_conversation_read(&topic_id)
    }

    pub fn sync_conversations(
        self: Arc<Self>,
        updated_at: Option<String>,
        limit: u32,
        callback: Box<dyn callback_wrap::RSSyncConversationsCallback>,
    ) {
        self.0.sync_conversations(
            updated_at,
            limit,
            Box::new(callback_wrap::RSSyncConversationsCallbackWrap { 0: callback }),
        )
    }
}
