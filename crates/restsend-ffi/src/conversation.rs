use super::RsClient;
use crate::callback_wrap::*;
use restsend_sdk::models;
use restsend_sdk::Result;
use std::sync::Arc;

#[uniffi::export]
impl RsClient {
    pub async fn create_topic(
        self: Arc<Self>,
        icon: String,
        name: String,
        members: Vec<String>,
    ) -> Result<models::Conversation> {
        self.0.create_topic(&icon, &name, members).await
    }

    pub async fn create_chat(self: Arc<Self>, user_id: String) -> Option<models::Conversation> {
        self.0.create_chat(&user_id).await
    }

    pub async fn clean_history(self: Arc<Self>, topic_id: String) -> Result<()> {
        self.0.clean_history(&topic_id).await
    }

    pub async fn remove_messages(
        self: Arc<Self>,
        topic_id: String,
        chat_ids: Vec<String>,
        sync_to_server: bool,
    ) -> Result<()> {
        self.0
            .remove_messages(&topic_id, chat_ids, sync_to_server)
            .await
    }

    pub fn sync_chat_logs(
        self: Arc<Self>,
        topic_id: String,
        last_seq: i64,
        limit: u32,
        callback: Box<dyn RSSyncChatLogsCallback>,
    ) {
        self.0.sync_chat_logs(
            &topic_id,
            last_seq,
            limit,
            Box::new(RSSyncChatLogsCallbackWrap { 0: callback }),
        )
    }

    pub fn get_chat_log(
        self: Arc<Self>,
        topic_id: String,
        chat_id: String,
    ) -> Option<models::ChatLog> {
        self.0.get_chat_log(&topic_id, &chat_id)
    }

    pub async fn search_chat_log(
        self: Arc<Self>,
        topic_id: Option<String>,
        sender_id: Option<String>,
        keyword: String,
    ) -> Option<models::GetChatLogsResult> {
        self.0.search_chat_log(topic_id, sender_id, &keyword).await
    }
}
