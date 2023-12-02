use super::attachment::Attachment;
use super::Client;
use crate::callback::{MessageCallback, UploadCallback};
use crate::request::ChatRequestType;
use crate::{models::Content, request::ChatRequest};
use crate::{MAX_RETRIES, MAX_SEND_IDLE_SECS};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

pub struct PendingRequest {
    pub callback: Option<Box<dyn MessageCallback>>,
    pub req: ChatRequest,
    pub retry: AtomicUsize,
    pub updated_at: Instant,
    pub can_retry: bool,
}

impl PendingRequest {
    pub fn new(req: ChatRequest, callback: Option<Box<dyn MessageCallback>>) -> Self {
        let can_retry = match ChatRequestType::from(&req.r#type) {
            ChatRequestType::Typing | ChatRequestType::Read => false,
            _ => true,
        };

        PendingRequest {
            callback,
            req,
            retry: AtomicUsize::new(0),
            can_retry,
            updated_at: Instant::now(),
        }
    }

    pub fn is_expired(&self) -> bool {
        if !self.can_retry {
            return true;
        }
        let retry_count = self.retry.load(Ordering::Relaxed);
        retry_count >= MAX_RETRIES || self.updated_at.elapsed().as_secs() > MAX_SEND_IDLE_SECS
    }

    pub fn did_retry(&self) {
        self.retry.fetch_add(1, Ordering::Relaxed);
    }
}

type PendingRequests = Mutex<HashMap<String, PendingRequest>>;
pub(super) struct MessageInner {
    pub(super) pendings: PendingRequests,
}

impl MessageInner {
    pub(super) fn new() -> Self {
        Self {
            pendings: Mutex::new(HashMap::new()),
        }
    }

    // pub(super) fn push_pending(&self, req_id: &str, req: PendingRequest) {
    //     self.pendings
    //         .lock()
    //         .unwrap()
    //         .insert(req_id.to_string(), req);
    // }

    // pub(super) fn pop_pending(&self, req_id: &str) -> Option<PendingRequest> {
    //     self.pendings.lock().unwrap().remove(req_id)
    // }

    // pub(super) fn update_retry(&self, req_id: &str) {
    //     let mut pendings = self.pendings.lock().unwrap();
    //     if let Some(p) = pendings.get_mut(req_id) {
    //         p.retry = p.retry + 1;
    //         p.updated_at = Instant::now();
    //     }
    // }
}

impl Client {
    pub(super) async fn send_chat_request(
        &self,
        req: ChatRequest,
        callback: Option<Box<dyn MessageCallback>>,
    ) -> Result<String> {
        let chat_id = req.chat_id.clone();

        // TODO: check if the request is already in pending
        if let Some(sender) = self.ws_sender.lock().unwrap().as_ref() {
            let pending = PendingRequest::new(req, callback);
            sender.send(Some(pending))?;
        }

        Ok(chat_id)
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
        self.send_chat_request(req, callback).await
    }

    pub async fn do_send_image(
        &self,
        topic_id: &str,
        attachment: Attachment,
        attachment_callback: Option<Box<dyn UploadCallback>>,
        mentions: Option<Vec<String>>,
        reply_id: Option<String>,
        callback: Option<Box<dyn MessageCallback>>,
    ) -> Result<String> {
        let r = self
            .attachment_inner
            .upload_attachment(&self.endpoint, &self.token, attachment, attachment_callback)
            .await?;
        let req = ChatRequest::new_image(&topic_id, &r.path, r.size)
            .mentions(mentions)
            .reply_id(reply_id);
        self.send_chat_request(req, callback).await
    }

    pub async fn do_send_voice(
        &self,
        topic_id: &str,
        attachment: Attachment,
        attachment_callback: Option<Box<dyn UploadCallback>>,
        duration: &str,
        mentions: Option<Vec<String>>,
        reply_id: Option<String>,
        callback: Option<Box<dyn MessageCallback>>,
    ) -> Result<String> {
        let r = self
            .attachment_inner
            .upload_attachment(&self.endpoint, &self.token, attachment, attachment_callback)
            .await?;
        let req = ChatRequest::new_voice(&topic_id, &r.path, &duration, r.size)
            .mentions(mentions)
            .reply_id(reply_id);
        self.send_chat_request(req, callback).await
    }

    pub async fn do_send_video(
        &self,
        topic_id: &str,
        attachment: Attachment,
        attachment_callback: Option<Box<dyn UploadCallback>>,
        duration: String,
        mentions: Option<Vec<String>>,
        reply_id: Option<String>,
        callback: Option<Box<dyn MessageCallback>>,
    ) -> Result<String> {
        let r = self
            .attachment_inner
            .upload_attachment(&self.endpoint, &self.token, attachment, attachment_callback)
            .await?;
        let req = ChatRequest::new_video(&topic_id, &r.path, &r.thumbnail, &duration, r.size)
            .mentions(mentions)
            .reply_id(reply_id);
        self.send_chat_request(req, callback).await
    }

    pub async fn do_send_file(
        &self,
        topic_id: &str,
        attachment: Attachment,
        attachment_callback: Option<Box<dyn UploadCallback>>,
        filename: String,
        mentions: Option<Vec<String>>,
        reply_id: Option<String>,
        callback: Option<Box<dyn MessageCallback>>,
    ) -> Result<String> {
        let r = self
            .attachment_inner
            .upload_attachment(&self.endpoint, &self.token, attachment, attachment_callback)
            .await?;
        let req = ChatRequest::new_file(&topic_id, &r.path, &filename, r.size)
            .mentions(mentions)
            .reply_id(reply_id);
        self.send_chat_request(req, callback).await
    }

    pub async fn do_send_location(
        &self,
        topic_id: &str,
        latitude: String,
        longitude: String,
        address: String,
        mentions: Option<Vec<String>>,
        reply_id: Option<String>,
        callback: Option<Box<dyn MessageCallback>>,
    ) -> Result<String> {
        let req = ChatRequest::new_location(&topic_id, &latitude, &longitude, &address)
            .mentions(mentions)
            .reply_id(reply_id);
        self.send_chat_request(req, callback).await
    }

    pub async fn do_send_link(
        &self,
        topic_id: &str,
        url: String,
        mentions: Option<Vec<String>>,
        reply_id: Option<String>,
        callback: Option<Box<dyn MessageCallback>>,
    ) -> Result<String> {
        let req = ChatRequest::new_link(&topic_id, &url)
            .mentions(mentions)
            .reply_id(reply_id);
        self.send_chat_request(req, callback).await
    }

    pub async fn do_send_logs(
        &self,
        topic_id: &str,
        log_ids: Vec<String>,
        mentions: Option<Vec<String>>,
        callback: Option<Box<dyn MessageCallback>>,
    ) -> Result<String> {
        // TODO: combine log_ids into one string
        /*
        let mut f = NamedTempFile::new()?;

        let attachment = Attachment {
            key: "".to_string(),
            file_path: f.path().to_str().unwrap().to_string(),
            is_private: false,
        };
        */

        let attachment = Attachment {
            key: "".to_string(),
            file_path: "".to_string(),
            is_private: false,
        };

        let r = self
            .attachment_inner
            .upload_attachment(&self.endpoint, &self.token, attachment, None)
            .await?;
        let req = ChatRequest::new_logs(&topic_id, &r.path, r.size).mentions(mentions);
        self.send_chat_request(req, callback).await
    }

    pub async fn do_send_invite(
        &self,
        topic_id: String,
        mentions: Vec<String>,
        message: Option<String>,
        callback: Option<Box<dyn MessageCallback>>,
    ) -> Result<String> {
        let req = ChatRequest::new_invite(&topic_id, &message.unwrap_or_default())
            .mentions(Some(mentions));
        self.send_chat_request(req, callback).await
    }

    pub async fn do_typing(&self, topic_id: &str) -> Result<()> {
        let req = ChatRequest::new_typing(&topic_id);
        self.send_chat_request(req, None).await.map(|_| ())
    }

    pub async fn do_read(&self, topic_id: &str) -> Result<()> {
        let req = ChatRequest::new_read(&topic_id);
        self.send_chat_request(req, None).await.map(|_| ())
    }

    pub async fn do_recall(
        &self,
        topic_id: &str,
        chat_id: String,
        callback: Option<Box<dyn MessageCallback>>,
    ) -> Result<String> {
        let req = ChatRequest::new_recall(&topic_id, &chat_id);
        self.send_chat_request(req, callback).await
    }

    pub async fn do_send(
        &self,
        topic_id: &str,
        content: Content,
        callback: Option<Box<dyn MessageCallback>>,
    ) -> Result<String> {
        let req = ChatRequest::new_chat_with_content(&topic_id, content);
        self.send_chat_request(req, callback).await
    }
}
