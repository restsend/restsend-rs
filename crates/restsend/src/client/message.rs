use super::Client;
use crate::callback::MessageCallback;
use crate::models::chat_log::Attachment;
use crate::models::conversation::Extra;
use crate::services::conversation::send_request;
use crate::services::response::APISendResponse;
use crate::Result;
use crate::{models::Content, request::ChatRequest};
use restsend_macros::export_wasm_or_ffi;

#[cfg(not(target_family = "wasm"))]
pub fn save_logs_to_file(root_path: &str, file_name: &str, data: String) -> Result<Attachment> {
    use log::warn;
    use std::io::Write;

    let file_path = Client::temp_path(root_path, Some("history_*.json".to_string()));
    let mut file = std::fs::File::create(&file_path)?;
    let file_data = data.as_bytes();
    let file_size = file_data.len();
    file.write_all(file_data)?;
    file.sync_all()?;

    drop(file);

    warn!(
        "save logs file_path:{} size:{} file:{:?}",
        file_path, file_size, file_path
    );
    Ok(Attachment::from_local(file_name, &file_path, false))
}

#[allow(unused)]
pub fn save_logs_to_blob(file_name: &str, data: String) -> Result<Attachment> {
    use wasm_bindgen::JsValue;
    let file_size = data.len() as i64;
    let array: js_sys::Array = js_sys::Array::new();
    array.push(&JsValue::from_str(&data));

    match web_sys::Blob::new_with_str_sequence_and_options(
        &array,
        web_sys::BlobPropertyBag::new().type_("application/json"),
    ) {
        Ok(blob) => Ok(Attachment::from_blob(
            blob,
            Some(file_name.to_string()),
            false,
            file_size,
        )),
        Err(e) => {
            web_sys::console::error_1(&e);
            return Err(e.into());
        }
    }
}

#[export_wasm_or_ffi]
impl Client {
    pub async fn send_chat_request(
        &self,
        topic_id: String,
        req: ChatRequest,
    ) -> Result<APISendResponse> {
        send_request(&self.endpoint, &self.token, &topic_id, req).await
    }

    async fn send_chat_request_via_connection(
        &self,
        req: ChatRequest,
        callback: Option<Box<dyn MessageCallback>>,
    ) -> Result<String> {
        let chat_id = req.chat_id.clone();
        let store_ref = self.store.clone();
        store_ref.add_pending_request(req, callback).await;
        Ok(chat_id)
    }

    pub async fn do_send_text(
        &self,
        topic_id: String,
        text: String,
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
        topic_id: String,
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
        topic_id: String,
        attachment: Attachment,
        duration: String,
        mentions: Option<Vec<String>>,
        mention_all: bool,
        reply_id: Option<String>,
        callback: Option<Box<dyn MessageCallback>>,
    ) -> Result<String> {
        let req = ChatRequest::new_voice(&topic_id, &duration, attachment)
            .mentions(mentions)
            .mention_all(mention_all)
            .reply_id(reply_id);
        self.send_chat_request_via_connection(req, callback).await
    }

    pub async fn do_send_video(
        &self,
        topic_id: String,
        attachment: Attachment,
        duration: String,
        mentions: Option<Vec<String>>,
        mention_all: bool,
        reply_id: Option<String>,
        callback: Option<Box<dyn MessageCallback>>,
    ) -> Result<String> {
        let req = ChatRequest::new_video(&topic_id, &duration, attachment)
            .mentions(mentions)
            .mention_all(mention_all)
            .reply_id(reply_id);
        self.send_chat_request_via_connection(req, callback).await
    }

    pub async fn do_send_file(
        &self,
        topic_id: String,
        attachment: Attachment,
        mentions: Option<Vec<String>>,
        mention_all: bool,
        reply_id: Option<String>,
        callback: Option<Box<dyn MessageCallback>>,
    ) -> Result<String> {
        let req = ChatRequest::new_file(&topic_id, attachment)
            .mentions(mentions)
            .mention_all(mention_all)
            .reply_id(reply_id);
        self.send_chat_request_via_connection(req, callback).await
    }

    pub async fn do_send_location(
        &self,
        topic_id: String,
        latitude: String,
        longitude: String,
        address: String,
        mentions: Option<Vec<String>>,
        mention_all: bool,
        reply_id: Option<String>,
        callback: Option<Box<dyn MessageCallback>>,
    ) -> Result<String> {
        let req = ChatRequest::new_location(&topic_id, &latitude, &longitude, &address)
            .mentions(mentions)
            .mention_all(mention_all)
            .reply_id(reply_id);
        self.send_chat_request_via_connection(req, callback).await
    }

    pub async fn do_send_link(
        &self,
        topic_id: String,
        url: String,
        placeholder: String,
        mentions: Option<Vec<String>>,
        mention_all: bool,
        reply_id: Option<String>,
        callback: Option<Box<dyn MessageCallback>>,
    ) -> Result<String> {
        let req = ChatRequest::new_link(&topic_id, &url, &placeholder)
            .mentions(mentions)
            .mention_all(mention_all)
            .reply_id(reply_id);
        self.send_chat_request_via_connection(req, callback).await
    }

    pub async fn do_send_invite(
        &self,
        topic_id: String,
        messsage: Option<String>,
        callback: Option<Box<dyn MessageCallback>>,
    ) -> Result<String> {
        let req = ChatRequest::new_invite(&topic_id, &messsage.unwrap_or_default());
        self.send_chat_request_via_connection(req, callback).await
    }

    pub async fn do_send_logs(
        &self,
        topic_id: String,
        source_topic_id: String,
        log_ids: Vec<String>,
        mentions: Option<Vec<String>>,
        mention_all: bool,
        callback: Option<Box<dyn MessageCallback>>,
    ) -> Result<String> {
        let file_name = "Chat history";
        let mut items = Vec::new();
        for log_id in log_ids.iter() {
            if let Some(log) = self.store.get_chat_log(&source_topic_id, log_id).await {
                items.push(log.to_string());
            }
        }

        let data = serde_json::json!({
            "topicId": source_topic_id,
            "ownerId": self.user_id,
            "createdAt": chrono::Local::now().to_rfc3339(),
            "logIds": log_ids,
            "items": items,
        })
        .to_string();

        #[cfg(not(target_family = "wasm"))]
        let attachment = save_logs_to_file(&self.root_path, &file_name, data)?;
        #[cfg(target_family = "wasm")]
        let attachment = save_logs_to_blob(&file_name, data)?;

        let req = ChatRequest::new_logs(&topic_id, attachment)
            .mentions(mentions)
            .mention_all(mention_all);
        self.send_chat_request_via_connection(req, callback).await
    }

    pub fn cancel_send(&self, chat_id: String) {
        self.store.cancel_send(&chat_id)
    }

    pub async fn do_typing(&self, topic_id: String) -> Result<()> {
        let req = ChatRequest::new_typing(&topic_id);
        self.send_chat_request_via_connection(req, None)
            .await
            .map(|_| ())
    }

    pub async fn do_read(&self, topic_id: String) -> Result<()> {
        let req = ChatRequest::new_read(&topic_id);
        self.send_chat_request_via_connection(req, None)
            .await
            .map(|_| ())
    }

    pub async fn do_recall(
        &self,
        topic_id: String,
        chat_id: String,
        callback: Option<Box<dyn MessageCallback>>,
    ) -> Result<String> {
        let req = ChatRequest::new_recall(&topic_id, &chat_id);
        self.send_chat_request_via_connection(req, callback).await
    }

    pub async fn do_send(
        &self,
        topic_id: String,
        content: Content,
        callback: Option<Box<dyn MessageCallback>>,
    ) -> Result<String> {
        let req = ChatRequest::new_chat_with_content(&topic_id, content);
        self.send_chat_request_via_connection(req, callback).await
    }

    pub async fn do_update_extra(
        &self,
        topic_id: String,
        chat_id: String,
        extra: Option<Extra>,
        callback: Option<Box<dyn MessageCallback>>,
    ) -> Result<String> {
        let req = ChatRequest::new_chat(&topic_id, crate::models::ContentType::UpdateExtra)
            .text(&chat_id)
            .extra(extra);
        self.send_chat_request_via_connection(req, callback).await
    }
}
