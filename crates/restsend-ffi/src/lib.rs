uniffi::setup_scaffolding!();

use restsend_sdk::{
    callback, client,
    models::{Attachment, AuthInfo, Content, User},
};
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
        callback: Box<dyn callback_wrap::RSSyncConversationsCallback>,
    ) {
        self.0.sync_conversations(
            updated_at,
            limit,
            Box::new(callback_wrap::RSSyncConversationsCallbackWrap { 0: callback }),
        )
    }

    // User
    pub fn get_user(self: Arc<Self>, user_id: String) -> Option<User> {
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

    // User API
    pub async fn do_typing(self: Arc<Self>, topic_id: String) {
        self.0.do_typing(&topic_id).await.ok();
    }

    pub async fn do_read(self: Arc<Self>, topic_id: String) {
        self.0.do_read(&topic_id).await.ok();
    }

    pub async fn do_send(
        self: Arc<Self>,
        topic_id: String,
        content: Content,
        callback: Option<Box<dyn callback_wrap::RSMessageCallback>>,
    ) {
        self.0
            .do_send(
                &topic_id,
                content,
                Some(Box::new(callback_wrap::RSMessageCallbackWrap {
                    0: callback,
                })),
            )
            .await
            .ok();
    }

    pub async fn do_recall(
        self: Arc<Self>,
        topic_id: String,
        chat_id: String,
        callback: Option<Box<dyn callback_wrap::RSMessageCallback>>,
    ) -> Option<String> {
        self.0
            .do_recall(
                &topic_id,
                &chat_id,
                Some(Box::new(callback_wrap::RSMessageCallbackWrap {
                    0: callback,
                })),
            )
            .await
            .ok()
    }

    pub async fn do_send_text(
        self: Arc<Self>,
        topic_id: String,
        text: String,
        mentions: Option<Vec<String>>,
        reply_id: Option<String>,
        callback: Option<Box<dyn callback_wrap::RSMessageCallback>>,
    ) -> Option<String> {
        self.0
            .do_send_text(
                &topic_id,
                &text,
                mentions,
                reply_id,
                Some(Box::new(callback_wrap::RSMessageCallbackWrap {
                    0: callback,
                })),
            )
            .await
            .ok()
    }

    pub async fn do_send_image(
        self: Arc<Self>,
        topic_id: String,
        attachment: Attachment,
        mentions: Option<Vec<String>>,
        reply_id: Option<String>,
        callback: Option<Box<dyn callback_wrap::RSMessageCallback>>,
    ) -> Option<String> {
        self.0
            .do_send_image(
                &topic_id,
                attachment,
                mentions,
                reply_id,
                Some(Box::new(callback_wrap::RSMessageCallbackWrap {
                    0: callback,
                })),
            )
            .await
            .ok()
    }

    pub async fn do_send_voice(
        self: Arc<Self>,
        topic_id: String,
        attachment: Attachment,
        duration: String,
        mentions: Option<Vec<String>>,
        reply_id: Option<String>,
        callback: Option<Box<dyn callback_wrap::RSMessageCallback>>,
    ) -> Option<String> {
        self.0
            .do_send_voice(
                &topic_id,
                attachment,
                &duration,
                mentions,
                reply_id,
                Some(Box::new(callback_wrap::RSMessageCallbackWrap {
                    0: callback,
                })),
            )
            .await
            .ok()
    }

    pub async fn do_send_video(
        self: Arc<Self>,
        topic_id: String,
        attachment: Attachment,
        duration: String,
        mentions: Option<Vec<String>>,
        reply_id: Option<String>,
        callback: Option<Box<dyn callback_wrap::RSMessageCallback>>,
    ) -> Option<String> {
        self.0
            .do_send_video(
                &topic_id,
                attachment,
                &duration,
                mentions,
                reply_id,
                Some(Box::new(callback_wrap::RSMessageCallbackWrap {
                    0: callback,
                })),
            )
            .await
            .ok()
    }

    pub async fn do_send_file(
        self: Arc<Self>,
        topic_id: String,
        attachment: Attachment,
        mentions: Option<Vec<String>>,
        reply_id: Option<String>,
        callback: Option<Box<dyn callback_wrap::RSMessageCallback>>,
    ) -> Option<String> {
        self.0
            .do_send_file(
                &topic_id,
                attachment,
                mentions,
                reply_id,
                Some(Box::new(callback_wrap::RSMessageCallbackWrap {
                    0: callback,
                })),
            )
            .await
            .ok()
    }

    pub async fn do_send_location(
        self: Arc<Self>,
        topic_id: String,
        latitude: String,
        longitude: String,
        address: String,
        mentions: Option<Vec<String>>,
        reply_id: Option<String>,
        callback: Option<Box<dyn callback_wrap::RSMessageCallback>>,
    ) -> Option<String> {
        self.0
            .do_send_location(
                &topic_id,
                &latitude,
                &longitude,
                &address,
                mentions,
                reply_id,
                Some(Box::new(callback_wrap::RSMessageCallbackWrap {
                    0: callback,
                })),
            )
            .await
            .ok()
    }

    pub async fn do_send_link(
        self: Arc<Self>,
        topic_id: String,
        url: String,
        placeholder: String,
        mentions: Option<Vec<String>>,
        reply_id: Option<String>,
        callback: Option<Box<dyn callback_wrap::RSMessageCallback>>,
    ) -> Option<String> {
        self.0
            .do_send_link(
                &topic_id,
                &url,
                &placeholder,
                mentions,
                reply_id,
                Some(Box::new(callback_wrap::RSMessageCallbackWrap {
                    0: callback,
                })),
            )
            .await
            .ok()
    }
}
