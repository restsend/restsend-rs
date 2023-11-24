use crate::models::Content;
use crate::request::ChatRequest;
use crate::Result;

impl super::Client {
    pub fn do_send_text(
        &self,
        topic_id: String,
        text: String,
        mentions: Option<Vec<String>>,
        reply_id: Option<String>,
    ) -> Result<String> {
        let req = ChatRequest::new_text(&topic_id, &text)
            .mentions(mentions)
            .reply_id(reply_id);
        self.send_chat_request(&topic_id, &req).map(|_| req.chat_id)
    }

    pub fn do_send_image(
        &self,
        topic_id: String,
        url_or_data: String,
        mentions: Option<Vec<String>>,
        reply_id: Option<String>,
    ) -> Result<String> {
        let req = ChatRequest::new_image(&topic_id, &url_or_data)
            .mentions(mentions)
            .reply_id(reply_id);
        self.send_chat_request(&topic_id, &req).map(|_| req.chat_id)
    }

    pub fn do_send_voice(
        &self,
        topic_id: String,
        url_or_data: String,
        duration: String,
        mentions: Option<Vec<String>>,
        reply_id: Option<String>,
    ) -> Result<String> {
        let req = ChatRequest::new_voice(&topic_id, &url_or_data, &duration)
            .mentions(mentions)
            .reply_id(reply_id);
        self.send_chat_request(&topic_id, &req).map(|_| req.chat_id)
    }

    pub fn do_send_video(
        &self,
        topic_id: String,
        url: String,
        thumbnail: String,
        duration: String,
        mentions: Option<Vec<String>>,
        reply_id: Option<String>,
    ) -> Result<String> {
        let req = ChatRequest::new_video(&topic_id, &url, &thumbnail, &duration)
            .mentions(mentions)
            .reply_id(reply_id);
        self.send_chat_request(&topic_id, &req).map(|_| req.chat_id)
    }

    pub fn do_send_file(
        &self,
        topic_id: String,
        url_or_data: String,
        filename: String,
        size: u64,
        mentions: Option<Vec<String>>,
        reply_id: Option<String>,
    ) -> Result<String> {
        let req = ChatRequest::new_file(&topic_id, &url_or_data, &filename, size)
            .mentions(mentions)
            .reply_id(reply_id);
        self.send_chat_request(&topic_id, &req).map(|_| req.chat_id)
    }

    pub fn do_send_location(
        &self,
        topic_id: String,
        latitude: String,
        longitude: String,
        address: String,
        mentions: Option<Vec<String>>,
        reply_id: Option<String>,
    ) -> Result<String> {
        let req = ChatRequest::new_location(&topic_id, &latitude, &longitude, &address)
            .mentions(mentions)
            .reply_id(reply_id);
        self.send_chat_request(&topic_id, &req).map(|_| req.chat_id)
    }

    pub fn do_send_link(
        &self,
        topic_id: String,
        url: String,
        mentions: Option<Vec<String>>,
        reply_id: Option<String>,
    ) -> Result<String> {
        let req = ChatRequest::new_link(&topic_id, &url)
            .mentions(mentions)
            .reply_id(reply_id);
        self.send_chat_request(&topic_id, &req).map(|_| req.chat_id)
    }

    pub fn do_send_invite(
        &self,
        topic_id: String,
        mentions: Vec<String>,
        message: Option<String>,
    ) -> Result<String> {
        let req = ChatRequest::new_invite(&topic_id, &message.unwrap_or_default())
            .mentions(Some(mentions));
        self.send_chat_request(&topic_id, &req).map(|_| req.chat_id)
    }

    pub fn do_typing(&self, topic_id: String) -> Result<String> {
        let req = ChatRequest::new_typing(&topic_id);
        self.ws_send(serde_json::to_string(&req)?, 0)
            .map(|_| req.chat_id)
    }

    pub fn do_read(&self, topic_id: String) -> Result<String> {
        let req = ChatRequest::new_read(&topic_id);
        self.ws_send(serde_json::to_string(&req)?, 0)
            .map(|_| req.chat_id)
    }

    pub fn do_recall(&self, topic_id: String, chat_id: String) -> Result<String> {
        let req = ChatRequest::new_recall(&topic_id, &chat_id);
        self.send_chat_request(&topic_id, &req).map(|_| req.chat_id)
    }

    pub fn do_send(&self, topic_id: String, content: Content) -> Result<String> {
        let req = ChatRequest::new_chat_with_content(&topic_id, content);
        self.send_chat_request(&topic_id, &req).map(|_| req.chat_id)
    }
}
