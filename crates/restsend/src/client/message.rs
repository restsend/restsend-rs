use super::Client;
use crate::callback::MessageCallback;
use crate::models::chat_log::Attachment;
use crate::services::conversation::send_request;
use crate::services::response::APISendResponse;
use crate::Result;
use crate::{models::Content, request::ChatRequest};
use log::warn;
use std::io::Write;

impl Client {
    pub async fn send_chat_request(
        &self,
        topic_id: &str,
        req: ChatRequest,
    ) -> Result<APISendResponse> {
        send_request(&self.endpoint, &self.token, topic_id, req).await
    }

    async fn send_chat_request_via_connection(
        &self,
        req: ChatRequest,
        callback: Option<Box<dyn MessageCallback>>,
    ) -> Result<String> {
        let req_id = req.id.clone();
        self.store.add_pending_request(req, callback).await;
        Ok(req_id)
    }

    pub async fn do_send_text(
        &self,
        topic_id: &str,
        text: &str,
        mentions: Option<Vec<String>>,
        reply_id: Option<String>,
        callback: Option<Box<dyn MessageCallback>>,
    ) -> Result<String> {
        let req = ChatRequest::new_text(&topic_id, &text)
            .mentions(mentions)
            .reply_id(reply_id);
        self.send_chat_request_via_connection(req, callback).await
    }

    pub async fn do_send_image(
        &self,
        topic_id: &str,
        attachment: Attachment,
        mentions: Option<Vec<String>>,
        reply_id: Option<String>,
        callback: Option<Box<dyn MessageCallback>>,
    ) -> Result<String> {
        let req = ChatRequest::new_image(&topic_id, attachment)
            .mentions(mentions)
            .reply_id(reply_id);
        self.send_chat_request_via_connection(req, callback).await
    }

    pub async fn do_send_voice(
        &self,
        topic_id: &str,
        attachment: Attachment,
        duration: &str,
        mentions: Option<Vec<String>>,
        reply_id: Option<String>,
        callback: Option<Box<dyn MessageCallback>>,
    ) -> Result<String> {
        let req = ChatRequest::new_voice(topic_id, duration, attachment)
            .mentions(mentions)
            .reply_id(reply_id);
        self.send_chat_request_via_connection(req, callback).await
    }

    pub async fn do_send_video(
        &self,
        topic_id: &str,
        attachment: Attachment,
        duration: &str,
        mentions: Option<Vec<String>>,
        reply_id: Option<String>,
        callback: Option<Box<dyn MessageCallback>>,
    ) -> Result<String> {
        let req = ChatRequest::new_video(&topic_id, &duration, attachment)
            .mentions(mentions)
            .reply_id(reply_id);
        self.send_chat_request_via_connection(req, callback).await
    }

    pub async fn do_send_file(
        &self,
        topic_id: &str,
        attachment: Attachment,
        mentions: Option<Vec<String>>,
        reply_id: Option<String>,
        callback: Option<Box<dyn MessageCallback>>,
    ) -> Result<String> {
        let req = ChatRequest::new_file(&topic_id, attachment)
            .mentions(mentions)
            .reply_id(reply_id);
        self.send_chat_request_via_connection(req, callback).await
    }

    pub async fn do_send_location(
        &self,
        topic_id: &str,
        latitude: &str,
        longitude: &str,
        address: &str,
        mentions: Option<Vec<String>>,
        reply_id: Option<String>,
        callback: Option<Box<dyn MessageCallback>>,
    ) -> Result<String> {
        let req = ChatRequest::new_location(&topic_id, &latitude, &longitude, &address)
            .mentions(mentions)
            .reply_id(reply_id);
        self.send_chat_request_via_connection(req, callback).await
    }

    pub async fn do_send_link(
        &self,
        topic_id: &str,
        url: &str,
        placeholder: &str,
        mentions: Option<Vec<String>>,
        reply_id: Option<String>,
        callback: Option<Box<dyn MessageCallback>>,
    ) -> Result<String> {
        let req = ChatRequest::new_link(&topic_id, &url, &placeholder)
            .mentions(mentions)
            .reply_id(reply_id);
        self.send_chat_request_via_connection(req, callback).await
    }

    pub async fn do_send_invite(
        &self,
        topic_id: &str,
        messsage: Option<String>,
        callback: Option<Box<dyn MessageCallback>>,
    ) -> Result<String> {
        let req = ChatRequest::new_invite(&topic_id, &messsage.unwrap_or_default());
        self.send_chat_request_via_connection(req, callback).await
    }

    pub async fn do_send_logs(
        &self,
        topic_id: &str,
        log_ids: Vec<String>,
        mentions: Option<Vec<String>>,
        callback: Option<Box<dyn MessageCallback>>,
    ) -> Result<String> {
        let file_name = "Chat history";
        let file_path = Self::temp_path(&self.root_path, Some("history_*.json".to_string()));

        let mut items = Vec::new();
        for log_id in log_ids.iter() {
            if let Some(log) = self.store.get_chat_log(topic_id, log_id) {
                items.push(log.to_string());
            }
        }

        let data = serde_json::json!({
            "topicId": topic_id,
            "ownerId": self.user_id,
            "createdAt": chrono::Local::now().to_rfc3339(),
            "logIds": log_ids,
            "items": items,
        })
        .to_string();

        let mut file = std::fs::File::create(&file_path)?;
        let file_data = data.as_bytes();
        let file_size = file_data.len();
        file.write_all(file_data)?;
        file.sync_all()?;

        drop(file);

        warn!(
            "save logs, log_ids:{:?} size:{} file:{:?}",
            log_ids, file_size, file_path
        );

        let attachment = Attachment::local(file_name, &file_path, false);
        let req = ChatRequest::new_logs(&topic_id, attachment).mentions(mentions);
        self.send_chat_request_via_connection(req, callback).await
    }

    pub fn cancel_send(&self, req_id: &str) {
        self.store.cancel_send(req_id)
    }

    pub async fn do_typing(&self, topic_id: &str) -> Result<()> {
        let req = ChatRequest::new_typing(&topic_id);
        self.send_chat_request_via_connection(req, None)
            .await
            .map(|_| ())
    }

    pub async fn do_read(&self, topic_id: &str) -> Result<()> {
        let req = ChatRequest::new_read(&topic_id);
        self.send_chat_request_via_connection(req, None)
            .await
            .map(|_| ())
    }

    pub async fn do_recall(
        &self,
        topic_id: &str,
        chat_id: &str,
        callback: Option<Box<dyn MessageCallback>>,
    ) -> Result<String> {
        let req = ChatRequest::new_recall(&topic_id, &chat_id);
        self.send_chat_request_via_connection(req, callback).await
    }

    pub async fn do_send(
        &self,
        topic_id: &str,
        content: Content,
        callback: Option<Box<dyn MessageCallback>>,
    ) -> Result<String> {
        let req = ChatRequest::new_chat_with_content(&topic_id, content);
        self.send_chat_request_via_connection(req, callback).await
    }
}
