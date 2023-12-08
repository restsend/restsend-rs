use super::RsClient;
use crate::callback_wrap::*;
use restsend_sdk::models::{Attachment, Content};
use std::sync::Arc;

#[uniffi::export]
impl RsClient {
    // User API
    pub fn cancel_send(self: Arc<Self>, req_id: String) {
        self.0.cancel_send(&req_id);
    }

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
        callback: Option<Box<dyn RSMessageCallback>>,
    ) {
        self.0
            .do_send(
                &topic_id,
                content,
                Some(Box::new(RSMessageCallbackWrap { 0: callback })),
            )
            .await
            .ok();
    }

    pub async fn do_recall(
        self: Arc<Self>,
        topic_id: String,
        chat_id: String,
        callback: Option<Box<dyn RSMessageCallback>>,
    ) -> Option<String> {
        self.0
            .do_recall(
                &topic_id,
                &chat_id,
                Some(Box::new(RSMessageCallbackWrap { 0: callback })),
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
        callback: Option<Box<dyn RSMessageCallback>>,
    ) -> Option<String> {
        self.0
            .do_send_text(
                &topic_id,
                &text,
                mentions,
                reply_id,
                Some(Box::new(RSMessageCallbackWrap { 0: callback })),
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
        callback: Option<Box<dyn RSMessageCallback>>,
    ) -> Option<String> {
        self.0
            .do_send_image(
                &topic_id,
                attachment,
                mentions,
                reply_id,
                Some(Box::new(RSMessageCallbackWrap { 0: callback })),
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
        callback: Option<Box<dyn RSMessageCallback>>,
    ) -> Option<String> {
        self.0
            .do_send_voice(
                &topic_id,
                attachment,
                &duration,
                mentions,
                reply_id,
                Some(Box::new(RSMessageCallbackWrap { 0: callback })),
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
        callback: Option<Box<dyn RSMessageCallback>>,
    ) -> Option<String> {
        self.0
            .do_send_video(
                &topic_id,
                attachment,
                &duration,
                mentions,
                reply_id,
                Some(Box::new(RSMessageCallbackWrap { 0: callback })),
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
        callback: Option<Box<dyn RSMessageCallback>>,
    ) -> Option<String> {
        self.0
            .do_send_file(
                &topic_id,
                attachment,
                mentions,
                reply_id,
                Some(Box::new(RSMessageCallbackWrap { 0: callback })),
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
        callback: Option<Box<dyn RSMessageCallback>>,
    ) -> Option<String> {
        self.0
            .do_send_location(
                &topic_id,
                &latitude,
                &longitude,
                &address,
                mentions,
                reply_id,
                Some(Box::new(RSMessageCallbackWrap { 0: callback })),
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
        callback: Option<Box<dyn RSMessageCallback>>,
    ) -> Option<String> {
        self.0
            .do_send_link(
                &topic_id,
                &url,
                &placeholder,
                mentions,
                reply_id,
                Some(Box::new(RSMessageCallbackWrap { 0: callback })),
            )
            .await
            .ok()
    }

    pub async fn do_send_invite(
        &self,
        topic_id: String,
        messsage: Option<String>,
        callback: Option<Box<dyn RSMessageCallback>>,
    ) -> Option<String> {
        self.0
            .do_send_invite(
                &topic_id,
                messsage,
                Some(Box::new(RSMessageCallbackWrap { 0: callback })),
            )
            .await
            .ok()
    }

    pub async fn do_send_logs(
        &self,
        topic_id: String,
        log_ids: Vec<String>,
        mentions: Option<Vec<String>>,
        callback: Option<Box<dyn RSMessageCallback>>,
    ) -> Option<String> {
        self.0
            .do_send_logs(
                &topic_id,
                log_ids,
                mentions,
                Some(Box::new(RSMessageCallbackWrap { 0: callback })),
            )
            .await
            .ok()
    }
}
