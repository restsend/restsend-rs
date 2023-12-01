use super::omit_empty;
use crate::request::ChatRequest;
use serde::{Deserialize, Serialize};

// define content type enum for content
#[derive(Deserialize, Debug)]
pub enum ContentType {
    None,
    Recall,
    Text,
    Image,
    Video,
    Voice,
    File,
    Location,
    Sticker,
    Contact,
    Link,
    Invite,
    Logs,
    TopicCreate,
    TopicDismiss,
    TopicQuit,
    TopicKickout,
    TopicJoin,
    TopicNotice,
    TopicKnock,
    TopicKnockAccept,
    TopicKnockReject,
    TopicSilent,
    TopicSilentMember,
    TopicChangeOwner,
    Unknown(String),
}

// impl ContentType into String
impl From<ContentType> for String {
    fn from(value: ContentType) -> Self {
        match value {
            ContentType::None => "",
            ContentType::Recall => "recall",
            ContentType::Text => "text",
            ContentType::Image => "image",
            ContentType::Video => "video",
            ContentType::Voice => "voice",
            ContentType::File => "file",
            ContentType::Location => "location",
            ContentType::Sticker => "sticker",
            ContentType::Contact => "contact",
            ContentType::Link => "link",
            ContentType::Invite => "invite",
            ContentType::Logs => "logs",
            ContentType::TopicCreate => "topic.create",
            ContentType::TopicDismiss => "topic.dismiss",
            ContentType::TopicQuit => "topic.quit",
            ContentType::TopicKickout => "topic.kickout",
            ContentType::TopicJoin => "topic.join",
            ContentType::TopicNotice => "topic.notice",
            ContentType::TopicKnock => "topic.knock",
            ContentType::TopicKnockAccept => "topic.knock.accept",
            ContentType::TopicKnockReject => "topic.knock.reject",
            ContentType::TopicSilent => "topic.silent",
            ContentType::TopicSilentMember => "topic.silent.member",
            ContentType::TopicChangeOwner => "topic.changeowner",
            ContentType::Unknown(v) => return v.clone(),
        }
        .to_string()
    }
}

impl From<String> for ContentType {
    fn from(value: String) -> Self {
        match value.as_str() {
            "" => ContentType::None,
            "recall" => ContentType::Recall,
            "text" => ContentType::Text,
            "image" => ContentType::Image,
            "video" => ContentType::Video,
            "voice" => ContentType::Voice,
            "file" => ContentType::File,
            "location" => ContentType::Location,
            "sticker" => ContentType::Sticker,
            "contact" => ContentType::Contact,
            "link" => ContentType::Link,
            "invite" => ContentType::Invite,
            "topic.create" => ContentType::TopicCreate,
            "topic.dismiss" => ContentType::TopicDismiss,
            "topic.quit" => ContentType::TopicQuit,
            "topic.kickout" => ContentType::TopicKickout,
            "topic.join" => ContentType::TopicJoin,
            "topic.notice" => ContentType::TopicNotice,
            "topic.knock" => ContentType::TopicKnock,
            "topic.knock.accept" => ContentType::TopicKnockAccept,
            "topic.knock.reject" => ContentType::TopicKnockReject,
            "topic.silent" => ContentType::TopicSilent,
            "topic.silent.member" => ContentType::TopicSilentMember,
            "topic.changeowner" => ContentType::TopicChangeOwner,
            _ => ContentType::Unknown(value),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Content {
    pub r#type: String,

    #[serde(skip_serializing_if = "omit_empty")]
    #[serde(default)]
    pub encrypted: bool,

    #[serde(skip_serializing_if = "omit_empty")]
    #[serde(default)]
    pub checksum: u32,

    #[serde(default)]
    pub text: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub placeholder: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub thumbnail: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub duration: String,

    #[serde(skip_serializing_if = "omit_empty")]
    #[serde(default)]
    pub size: u64,

    #[serde(skip_serializing_if = "omit_empty")]
    #[serde(default)]
    pub width: f32,

    #[serde(skip_serializing_if = "omit_empty")]
    #[serde(default)]
    pub height: f32,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub mentions: Vec<String>,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub reply: String,

    #[serde(skip)]
    pub created_at: String,
}

impl Content {
    pub fn new(r#type: ContentType) -> Self {
        Content {
            r#type: String::from(r#type),
            ..Default::default()
        }
    }

    pub fn new_text(r#type: ContentType, text: &str) -> Self {
        Content {
            r#type: String::from(r#type),
            text: String::from(text),
            ..Default::default()
        }
    }
}

#[allow(dead_code)]
pub enum ChatLogStatus {
    Sending = 0,
    Sent = 1,
    Received = 2,
    Read = 3,
    Failed = 4,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChatLog {
    pub topic_id: String,
    pub id: String,
    pub seq: u64,
    pub created_at: String,
    pub sender_id: String,
    pub content: Content,
    pub read: bool,
    pub recall: bool,
    #[serde(skip)]
    pub status: u32,
    #[serde(skip)]
    pub cached_at: String,
}

impl ChatLog {
    pub fn new(topic_id: &str, id: &str) -> Self {
        ChatLog {
            topic_id: String::from(topic_id),
            id: String::from(id),
            ..Default::default()
        }
    }
}

impl From<&ChatRequest> for ChatLog {
    fn from(req: &ChatRequest) -> Self {
        let content = req.content.clone().unwrap_or_default();
        ChatLog {
            topic_id: req.topic_id.clone(),
            id: req.chat_id.clone(),
            seq: req.seq,
            created_at: req.created_at.clone(),
            sender_id: req.attendee.clone(),
            content,
            read: false,
            recall: req.r#type == "recall",
            status: ChatLogStatus::Received as u32,
            cached_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

#[test]
fn test_chat_content_decode() {
    let data = r#"{"type":"text","encrypted":true,"checksum":404}"#;
    assert!(serde_json::from_str::<Content>(data).is_ok());
}
