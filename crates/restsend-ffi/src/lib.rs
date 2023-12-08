uniffi::setup_scaffolding!();
use callback_wrap::*;
use restsend_sdk::{client, models};
use std::sync::Arc;

mod account;
mod attachment;
mod callback_wrap;
mod conversation;
mod message;
mod topic;

#[derive(uniffi::Object)]
pub struct RsClient(client::Client);

#[uniffi::export]
impl RsClient {
    #[uniffi::constructor]
    pub fn new(root_path: String, db_name: String, info: models::AuthInfo) -> Arc<Self> {
        Arc::new(Self {
            0: client::Client::new(&root_path, &db_name, &info),
        })
    }

    pub async fn connect(self: Arc<Self>, callback: Box<dyn RSCallback>) {
        self.0.connect(Box::new(CallbackWrap { 0: callback })).await
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
    pub async fn remove_conversation(self: Arc<Self>, topic_id: String) {
        self.0.remove_conversation(&topic_id).await
    }

    pub async fn set_conversation_sticky(self: Arc<Self>, topic_id: String, sticky: bool) {
        self.0.set_conversation_sticky(&topic_id, sticky).await
    }

    pub async fn set_conversation_mute(self: Arc<Self>, topic_id: String, mute: bool) {
        self.0.set_conversation_mute(&topic_id, mute).await
    }

    pub async fn set_conversation_read(self: Arc<Self>, topic_id: String) {
        self.0.set_conversation_read(&topic_id).await
    }

    pub fn sync_conversations(
        self: Arc<Self>,
        updated_at: Option<String>,
        limit: u32,
        callback: Box<dyn RSSyncConversationsCallback>,
    ) {
        self.0.sync_conversations(
            updated_at,
            limit,
            Box::new(RSSyncConversationsCallbackWrap { 0: callback }),
        )
    }

    // User
    pub fn get_user(self: Arc<Self>, user_id: String) -> Option<models::User> {
        self.0.get_user(&user_id)
    }

    pub async fn set_user_remark(
        self: Arc<Self>,
        user_id: String,
        remark: String,
    ) -> restsend_sdk::Result<()> {
        self.0.set_user_remark(&user_id, &remark).await
    }

    pub async fn set_user_star(
        self: Arc<Self>,
        user_id: String,
        star: bool,
    ) -> restsend_sdk::Result<()> {
        self.0.set_user_star(&user_id, star).await
    }

    pub async fn set_user_block(
        self: Arc<Self>,
        user_id: String,
        block: bool,
    ) -> restsend_sdk::Result<()> {
        self.0.set_user_block(&user_id, block).await
    }

    pub async fn set_allow_guest_chat(self: Arc<Self>, allow: bool) -> restsend_sdk::Result<()> {
        self.0.set_allow_guest_chat(allow).await
    }
}
