use super::{conversation::Extra, omit_empty};
use crate::{request::ChatRequest, storage::StoreModel, utils::now_millis};
use restsend_macros::export_wasm_or_ffi;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

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
    TopicUpdate,
    TopicKnock,
    TopicKnockAccept,
    TopicKnockReject,
    TopicSilent,
    TopicSilentMember,
    TopicChangeOwner,
    ConversationUpdate,
    UpdateExtra,
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
            ContentType::TopicUpdate => "topic.update",
            ContentType::TopicKnock => "topic.knock",
            ContentType::TopicKnockAccept => "topic.knock.accept",
            ContentType::TopicKnockReject => "topic.knock.reject",
            ContentType::TopicSilent => "topic.silent",
            ContentType::TopicSilentMember => "topic.silent.member",
            ContentType::TopicChangeOwner => "topic.changeowner",
            ContentType::ConversationUpdate => "conversation.update",
            ContentType::UpdateExtra => "update.extra",
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
            "logs" => ContentType::Logs,
            "topic.create" => ContentType::TopicCreate,
            "topic.dismiss" => ContentType::TopicDismiss,
            "topic.quit" => ContentType::TopicQuit,
            "topic.kickout" => ContentType::TopicKickout,
            "topic.join" => ContentType::TopicJoin,
            "topic.notice" => ContentType::TopicNotice,
            "topic.update" => ContentType::TopicUpdate,
            "topic.knock" => ContentType::TopicKnock,
            "topic.knock.accept" => ContentType::TopicKnockAccept,
            "topic.knock.reject" => ContentType::TopicKnockReject,
            "topic.silent" => ContentType::TopicSilent,
            "topic.silent.member" => ContentType::TopicSilentMember,
            "topic.changeowner" => ContentType::TopicChangeOwner,
            "conversation.update" => ContentType::ConversationUpdate,
            "update.extra" => ContentType::UpdateExtra,
            _ => ContentType::Unknown(value),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[export_wasm_or_ffi(#[derive(uniffi::Enum)])]
pub enum AttachmentStatus {
    #[default]
    ToUpload,
    ToDownload,
    Uploading,
    Downloading,
    Paused,
    Done,
    Failed,
}

#[cfg(not(target_family = "wasm"))]
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
#[export_wasm_or_ffi(#[derive(uniffi::Record)])]
pub struct Attachment {
    /// if url is not empty, it means the attachment is from remote, without upload
    pub url: String,
    pub size: i64,
    pub thumbnail: String,
    pub file_name: String,
    pub file_path: String,
    pub url_or_data: String,
    pub is_private: bool,
    pub status: AttachmentStatus,
}

#[cfg(target_family = "wasm")]
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    pub url: String,
    pub size: i64,
    pub thumbnail: String,
    pub file_name: String,
    pub file_path: String,
    pub url_or_data: String,
    pub is_private: bool,
    pub status: AttachmentStatus,
    #[serde(skip)]
    pub file: Option<web_sys::Blob>,
}

#[export_wasm_or_ffi]
/// create attachment from url, without upload to server
pub fn attachment_from_url(url: String, is_private: bool, size: i64) -> Attachment {
    Attachment::from_url(&url, is_private, size)
}

#[cfg(not(target_family = "wasm"))]
#[export_wasm_or_ffi]
/// create attachment from local file, will upload to server when send message
pub fn attachment_from_local(file_name: String, file_path: String, is_private: bool) -> Attachment {
    Attachment::from_local(&file_name, &file_path, is_private)
}

impl Attachment {
    pub fn from_url(url: &str, is_private: bool, size: i64) -> Self {
        let file_name = match url::Url::parse(url) {
            Ok(u) => u
                .path_segments()
                .and_then(|segments| segments.last())
                .unwrap_or_default()
                .to_string(),
            Err(_) => "".to_string(),
        };

        Attachment {
            url: url.to_string(),
            size,
            file_name,
            is_private,
            ..Default::default()
        }
    }

    #[cfg(not(target_family = "wasm"))]
    pub fn from_local(file_name: &str, file_path: &str, is_private: bool) -> Self {
        Attachment {
            file_name: String::from(file_name),
            file_path: String::from(file_path),
            is_private,
            ..Default::default()
        }
    }

    #[allow(unused_variables)]
    pub fn from_blob(
        blob_stream: web_sys::Blob,
        file_name: Option<String>,
        is_private: bool,
        size: i64,
    ) -> Self {
        let file_name = file_name.unwrap_or("<blob>".to_string());
        Attachment {
            file_name: file_name.clone(),
            file_path: file_name,
            #[cfg(target_family = "wasm")]
            file: Some(blob_stream),
            is_private,
            size,
            ..Default::default()
        }
    }
}
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
#[export_wasm_or_ffi(#[derive(uniffi::Record)])]
pub struct Content {
    pub r#type: String,

    #[serde(skip_serializing_if = "omit_empty")]
    #[serde(default)]
    pub encrypted: bool,

    #[serde(skip_serializing_if = "omit_empty")]
    #[serde(default)]
    pub checksum: u32,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
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

    #[serde(skip_serializing_if = "omit_empty")]
    #[serde(default)]
    pub mention_all: bool,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub reply: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub reply_content: Option<String>,

    #[serde(skip)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub created_at: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub attachment: Option<Attachment>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub extra: Option<Extra>,
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
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
#[export_wasm_or_ffi(#[derive(uniffi::Enum)])]
pub enum ChatLogStatus {
    Uploading,
    #[default]
    Sending,
    Sent,
    Downloading,
    Received,
    Read,
    SendFailed,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
#[export_wasm_or_ffi(#[derive(uniffi::Record)])]
pub struct ChatLog {
    pub topic_id: String,
    pub id: String,
    pub seq: i64,
    pub created_at: String,
    pub sender_id: String,
    pub content: Content,
    pub read: bool,
    pub recall: bool,

    #[serde(default)]
    pub status: ChatLogStatus,

    #[serde(default)]
    pub cached_at: i64,
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
impl FromStr for ChatLog {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str::<ChatLog>(s)
    }
}

impl ToString for ChatLog {
    fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

impl StoreModel for ChatLog {
    fn sort_key(&self) -> i64 {
        self.seq as i64
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
            status: ChatLogStatus::Received,
            cached_at: now_millis(),
        }
    }
}

#[test]
fn test_chat_content_decode() {
    let data = r#"{"type":"text","encrypted":true,"checksum":404}"#;
    assert!(serde_json::from_str::<Content>(data).is_ok());
}
