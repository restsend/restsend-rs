use crate::models::{Content, ContentType, User};
use crate::utils::random_text;
use serde::{Deserialize, Serialize};
use tokio::time::Instant;

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

impl From<String> for ChatRequestType {
    fn from(value: String) -> Self {
        match value.as_str() {
            "chat" => ChatRequestType::Chat,
            "typing" => ChatRequestType::Typing,
            "read" => ChatRequestType::Read,
            "resp" => ChatRequestType::Response,
            "kickout" => ChatRequestType::Kickout,
            "system" => ChatRequestType::System,
            "nop" => ChatRequestType::Nop,
            _ => ChatRequestType::Unknown(value),
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

// 服务器与客户端的通讯的消息格式
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChatRequest {
    pub r#type: String,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub code: u32,
    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub topic_id: String,
    #[serde(default)]
    pub seq: u64,
    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub attendee: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attendee_profile: Option<User>,
    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub chat_id: String,
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
    pub fn new_typing(topic_id: &str) -> Self {
        ChatRequest {
            r#type: String::from(ChatRequestType::Typing),
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

    pub fn new_image(topic_id: &str, url_or_data: &str) -> Self {
        Self::new_chat(topic_id, ContentType::Image).text(url_or_data)
    }

    pub fn new_voice(topic_id: &str, url_or_data: &str, duration: &str) -> Self {
        Self::new_chat(topic_id, ContentType::Voice)
            .text(url_or_data)
            .duration(duration)
    }

    pub fn new_video(topic_id: &str, url_or_data: &str, thumbnail: &str, duration: &str) -> Self {
        Self::new_chat(topic_id, ContentType::Video)
            .text(url_or_data)
            .duration(duration)
            .thumbnail(thumbnail)
    }

    pub fn new_file(topic_id: &str, url_or_data: &str, filename: &str, size: u64) -> Self {
        Self::new_chat(topic_id, ContentType::File)
            .text(url_or_data)
            .placeholder(filename)
            .size(size)
    }

    pub fn new_location(topic_id: &str, latitude: &str, longitude: &str, address: &str) -> Self {
        let lat_lng = format!("{},{}", latitude, longitude);
        Self::new_chat(topic_id, ContentType::Location)
            .text(&lat_lng)
            .placeholder(address)
    }

    pub fn new_link(topic_id: &str, url: &str) -> Self {
        Self::new_chat(topic_id, ContentType::Link).text(url)
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

pub struct PendingRequest {
    pub req: ChatRequest,
    pub retry: usize,
    pub created_at: Instant,
}

impl PendingRequest {
    pub fn new(req: &ChatRequest, retry: usize) -> Self {
        PendingRequest {
            req: req.clone(),
            retry,
            created_at: Instant::now(),
        }
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
