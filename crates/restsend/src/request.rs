use crate::models::Attachment;
use crate::models::{omit_empty, Content, ContentType, User};
use crate::utils::random_text;
use restsend_macros::export_wasm_or_ffi;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq)]
pub enum ChatRequestType {
    Chat,
    Typing,
    Read,
    Response,
    Kickout,
    System,
    Nop, // KeepAlive
    Unknown(String),
}

impl From<&String> for ChatRequestType {
    fn from(value: &String) -> Self {
        match value.as_str() {
            "chat" => ChatRequestType::Chat,
            "typing" => ChatRequestType::Typing,
            "read" => ChatRequestType::Read,
            "resp" => ChatRequestType::Response,
            "kickout" => ChatRequestType::Kickout,
            "system" => ChatRequestType::System,
            "nop" => ChatRequestType::Nop,
            _ => ChatRequestType::Unknown(value.clone()),
        }
    }
}

impl From<ChatRequestType> for String {
    fn from(value: ChatRequestType) -> Self {
        match value {
            ChatRequestType::Chat => "chat",
            ChatRequestType::Typing => "typing",
            ChatRequestType::Read => "read",
            ChatRequestType::Response => "resp",
            ChatRequestType::Kickout => "kickout",
            ChatRequestType::System => "system",
            ChatRequestType::Nop => "nop",
            ChatRequestType::Unknown(v) => return v.clone(),
        }
        .to_string()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
#[export_wasm_or_ffi(#[derive(uniffi::Record)])]
pub struct ChatRequest {
    pub r#type: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub id: String,

    #[serde(skip_serializing_if = "omit_empty")]
    #[serde(default)]
    pub code: u32,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub topic_id: String,

    #[serde(skip_serializing_if = "omit_empty")]
    #[serde(default)]
    pub seq: i64,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub attendee: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub attendee_profile: Option<User>,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub chat_id: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub created_at: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Content>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub e2e_content: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

impl ChatRequest {
    pub fn new_response(req: &ChatRequest, code: u32) -> Option<Self> {
        if req.id == "" {
            return None;
        }
        Some(ChatRequest {
            r#type: String::from(ChatRequestType::Response),
            topic_id: req.topic_id.clone(),
            code,
            ..Default::default()
        })
    }
    pub fn new_typing(topic_id: &str) -> Self {
        ChatRequest {
            r#type: String::from(ChatRequestType::Typing),
            topic_id: String::from(topic_id),
            ..Default::default()
        }
    }
    pub fn new_read(topic_id: &str) -> Self {
        ChatRequest {
            r#type: String::from(ChatRequestType::Read),
            topic_id: String::from(topic_id),
            ..Default::default()
        }
    }

    pub fn new_chat_with_content(topic_id: &str, content: Content) -> Self {
        ChatRequest {
            r#type: String::from(ChatRequestType::Chat),
            id: random_text(crate::REQ_ID_LEN),
            topic_id: String::from(topic_id),
            chat_id: random_text(crate::CHAT_ID_LEN),
            content: Some(content),
            ..Default::default()
        }
    }
    pub fn new_chat(topic_id: &str, r#type: ContentType) -> Self {
        ChatRequest {
            r#type: String::from(ChatRequestType::Chat),
            id: random_text(crate::REQ_ID_LEN),
            topic_id: String::from(topic_id),
            chat_id: random_text(crate::CHAT_ID_LEN),
            content: Some(Content {
                r#type: String::from(r#type),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    pub fn new_text(topic_id: &str, text: &str) -> Self {
        Self::new_chat(topic_id, ContentType::Text).text(text)
    }

    pub fn new_image(topic_id: &str, attachment: Attachment) -> Self {
        Self::new_chat(topic_id, ContentType::Image).attachment(attachment)
    }

    pub fn new_voice(topic_id: &str, duration: &str, attachment: Attachment) -> Self {
        Self::new_chat(topic_id, ContentType::Voice)
            .duration(duration)
            .attachment(attachment)
    }

    pub fn new_video(topic_id: &str, duration: &str, attachment: Attachment) -> Self {
        Self::new_chat(topic_id, ContentType::Video)
            .duration(duration)
            .attachment(attachment)
    }

    pub fn new_file(topic_id: &str, attachment: Attachment) -> Self {
        Self::new_chat(topic_id, ContentType::File).attachment(attachment)
    }

    pub fn new_logs(topic_id: &str, attachment: Attachment) -> Self {
        Self::new_chat(topic_id, ContentType::Logs).attachment(attachment)
    }

    pub fn new_location(topic_id: &str, latitude: &str, longitude: &str, address: &str) -> Self {
        let lat_lng = format!("{},{}", latitude, longitude);
        Self::new_chat(topic_id, ContentType::Location)
            .text(&lat_lng)
            .placeholder(address)
    }

    pub fn new_link(topic_id: &str, url: &str, placeholder: &str) -> Self {
        Self::new_chat(topic_id, ContentType::Link)
            .text(url)
            .placeholder(placeholder)
    }

    pub fn new_invite(topic_id: &str, message: &str) -> Self {
        Self::new_chat(topic_id, ContentType::Invite).text(&message)
    }

    pub fn new_recall(topic_id: &str, chat_id: &str) -> Self {
        ChatRequest {
            r#type: String::from(ChatRequestType::Chat),
            topic_id: String::from(topic_id),
            chat_id: String::from(chat_id),
            content: Some(Content {
                r#type: String::from(ContentType::Recall),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    pub fn thumbnail(&self, thumbnail: &str) -> Self {
        ChatRequest {
            content: Some(Content {
                thumbnail: String::from(thumbnail),
                ..self.content.clone().unwrap_or(Content::default())
            }),
            ..self.clone()
        }
    }

    pub fn size(&self, size: u64) -> Self {
        ChatRequest {
            content: Some(Content {
                size,
                ..self.content.clone().unwrap_or(Content::default())
            }),
            ..self.clone()
        }
    }

    pub fn placeholder(&self, placeholder: &str) -> Self {
        ChatRequest {
            content: Some(Content {
                placeholder: String::from(placeholder),
                ..self.content.clone().unwrap_or(Content::default())
            }),
            ..self.clone()
        }
    }

    pub fn duration(&self, duration: &str) -> Self {
        ChatRequest {
            content: Some(Content {
                duration: String::from(duration),
                ..self.content.clone().unwrap_or(Content::default())
            }),
            ..self.clone()
        }
    }

    pub fn text(&self, text: &str) -> Self {
        ChatRequest {
            content: Some(Content {
                text: String::from(text),
                ..self.content.clone().unwrap_or(Content::default())
            }),
            ..self.clone()
        }
    }

    pub fn reply_id(&self, reply_id: Option<String>) -> Self {
        ChatRequest {
            content: Some(Content {
                reply: reply_id.unwrap_or_default(),
                ..self.content.clone().unwrap_or(Default::default())
            }),
            ..self.clone()
        }
    }

    pub fn mentions(&self, user_ids: Option<Vec<String>>) -> Self {
        ChatRequest {
            content: Some(Content {
                mentions: user_ids.unwrap_or_default(),
                ..self.content.clone().unwrap_or(Content::default())
            }),
            ..self.clone()
        }
    }
    pub fn attachment(&self, attachment: Attachment) -> Self {
        ChatRequest {
            content: Some(Content {
                attachment: Some(attachment),
                ..self.content.clone().unwrap_or(Content::default())
            }),
            ..self.clone()
        }
    }

    pub fn make_response(&self, code: u32) -> Self {
        ChatRequest {
            r#type: String::from(ChatRequestType::Response),
            id: self.id.clone(),
            code,
            topic_id: self.topic_id.clone(),
            seq: self.seq,
            attendee: self.attendee.clone(),
            chat_id: self.chat_id.clone(),
            ..Default::default()
        }
    }
}

impl From<ChatRequest> for String {
    fn from(req: ChatRequest) -> Self {
        serde_json::to_string(&req).unwrap()
    }
}

impl From<&ChatRequest> for String {
    fn from(req: &ChatRequest) -> Self {
        serde_json::to_string(req).unwrap()
    }
}

impl TryFrom<String> for ChatRequest {
    type Error = serde_json::Error;
    fn try_from(data: String) -> Result<Self, Self::Error> {
        serde_json::from_str(&data)
    }
}

#[test]
fn test_chat_request_decode() {
    let data = r#"{"type":"resp","id":"wn8qzkq6nt","code":404}"#;
    let req = serde_json::from_str::<ChatRequest>(data);
    assert!(req.is_ok());

    let data = r#"{}"#;
    serde_json::from_str::<ChatRequest>(data).expect_err("missing field `type`");
    let req = ChatRequest::new_text("greeting", "hello");
    let data = serde_json::to_string(&req).unwrap();
    let r = serde_json::from_str::<ChatRequest>(&data);
    assert!(r.is_ok());

    let r = r.unwrap();
    assert!(r.content.is_some());
    assert!(r.content.unwrap().text.eq_ignore_ascii_case("hello"));
    assert!(r.topic_id.eq_ignore_ascii_case("greeting"));
}

#[test]
fn test_chat_request_encode() {
    let mut req = ChatRequest::new_text("greeting", "hello");
    req.chat_id = "mock_chat_id".to_string();
    req.id = "mock_req_id".to_string();

    let data: String = req.into();
    assert!(data.contains("hello"));
    assert!(data.contains("greeting"));

    assert!(data.eq_ignore_ascii_case(
        r#"{"type":"chat","id":"mock_req_id","topicId":"greeting","chatId":"mock_chat_id","content":{"type":"text","text":"hello"}}"#
    ));
}
