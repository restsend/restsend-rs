use crate::request::ChatRequest;

use super::DBStore;
use rusqlite::params;
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use serde::{Deserialize, Serialize};

// define content type enum for content
#[derive(Deserialize, Debug)]
pub enum ContentType {
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
    Unknown(String),
}

// impl ContentType into String
impl From<ContentType> for String {
    fn from(value: ContentType) -> Self {
        match value {
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
            ContentType::Unknown(v) => return v.clone(),
        }
        .to_string()
    }
}

impl From<String> for ContentType {
    fn from(value: String) -> Self {
        match value.as_str() {
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
            _ => ContentType::Unknown(value),
        }
    }
}

//消息内容
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Content {
    pub r#type: String,
    #[serde(default)]
    pub encrypted: bool,         // 是否加密
    #[serde(default)]
    pub checksum: u32,           // 内容的checksum,用来做Text解密的校验
    #[serde(default)]
    pub text: String,           // 文本内容,是markdown格式
    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub placeholder: String,    // 用于显示的占位符, 例如: [图片]，文件名, address
    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub thumbnail: String,      // 缩略图
    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub duration: String,       // 声音时长
    #[serde(default)]
    pub size: u64,              // 内容大小或者文件大小
    #[serde(default)]
    pub width: f32,             // 图片或者视频的宽
    #[serde(default)]
    pub height: f32,            // 图片或者视频的高
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub mentions: Vec<String>, // 提到的人或者指定的人
    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub reply_id: String,      // 回复的chat_log
    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub created_at: String,    // 消息的创建时间
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

impl FromSql for Content {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        // unmarshal from json's string
        let s = String::column_result(value)?;
        let v = serde_json::from_str(&s).map_err(|_e| FromSqlError::InvalidType)?;
        Ok(v)
    }
}

impl ToSql for Content {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        let s = serde_json::to_string(&self)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        Ok(ToSqlOutput::from(s))
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

// 每行的消息记录
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
        ChatLog{
            topic_id: req.topic_id.clone(),
            id: req.chat_id.clone(),
            seq: req.seq,
            created_at: content.created_at.clone(),
            sender_id: req.attendee.clone(),
            content,
            read: false,
            recall: req.r#type == "recall",
            status: ChatLogStatus::Received as u32,
            cached_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

impl DBStore {
    pub fn get_chat_log(&self, topic_id: &str, id: &str) -> crate::Result<ChatLog> {
        let conn = self.pool.get()?;
        let chat_log = conn.query_row(
            "SELECT * FROM messages WHERE topic_id = ? AND id = ?",
            params![topic_id, id],
            |row| {
                let mut chat_log = ChatLog::new(
                    &row.get::<_, String>("topic_id")?,
                    &row.get::<_, String>("id")?,
                );
                chat_log.seq = row.get::<_, u64>("seq")?;
                chat_log.created_at = row.get("created_at")?;
                chat_log.sender_id = row.get("sender_id")?;
                chat_log.content = row.get("content")?;
                chat_log.read = row.get("read")?;
                chat_log.recall = row.get("recall")?;
                chat_log.status = row.get("status")?;
                chat_log.cached_at = row.get("cached_at")?;
                Ok(chat_log)
            },
        )?;
        Ok(chat_log)
    }

    pub fn search_chat_log(&self, topic_id: &str, sender_id: &str, keyword: &str) -> crate::Result<Vec<ChatLog>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT * FROM messages WHERE topic_id = ? AND sender_id = ? AND content LIKE ? ORDER BY seq DESC",
        )?;
        let mut rows = stmt.query(params![topic_id, sender_id, format!("%{}%", keyword)])?;
        let mut chat_logs = Vec::new();
        while let Some(row) = rows.next()? {
            let mut chat_log = ChatLog::new(
                &row.get::<_, String>("topic_id")?,
                &row.get::<_, String>("id")?,
            );
            chat_log.seq = row.get::<_, u64>("seq")?;
            chat_log.created_at = row.get("created_at")?;
            chat_log.sender_id = row.get("sender_id")?;
            chat_log.content = row.get("content")?;
            chat_log.read = row.get("read")?;
            chat_log.recall = row.get("recall")?;
            chat_log.status = row.get("status")?;
            chat_log.cached_at = row.get("cached_at")?;
            chat_logs.push(chat_log);
        }
        Ok(chat_logs)
    }

    pub fn save_chat_log(&self, chat_log: &ChatLog) -> crate::Result<()> {
        let conn = self.pool.get()?;
        let mut prefix = "INSERT" ;
        if chat_log.recall {
            prefix = "INSERT OR REPLACE";
        }
        conn.execute(
            &format!("{} INTO messages (topic_id, id, seq, created_at, sender_id, content, read, recall, status, cached_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)", prefix),
            params![chat_log.topic_id, chat_log.id, chat_log.seq, chat_log.created_at, chat_log.sender_id, serde_json::to_string(&chat_log.content)?, chat_log.read, chat_log.recall, chat_log.status as u32, chat_log.cached_at],
        )?;
        Ok(())
    }

    pub fn add_pending_chat_log(
        &self,
        topic_id: &str,
        sender_id: &str,
        id: &str,
        content: &Content,
    ) -> crate::Result<()> {
        let conn = self.pool.get()?;
        let mut prefix = "INSERT" ;
        if content.r#type == "recall" {
            prefix = "INSERT OR REPLACE";
        }
        conn.execute(
            &format!("{} INTO messages (topic_id, id, seq, created_at, sender_id, content, read, recall, status, cached_at) VALUES (?1, ?2, ?3, datetime('now'), ?4, ?5, ?6, ?7, ?8, datetime('now'))", prefix),
            params![topic_id, id, 
            0, // seq
            sender_id, serde_json::to_string(&content)?, 
            false, // read
            false, // recall
            ChatLogStatus::Sending as u32, // pending,
            ], 
        )?;
        Ok(())
    }

    pub fn update_chat_log_sent(&self, topic_id: &str, id: &str, seq: u64, status:ChatLogStatus) -> crate::Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE messages SET status = ?1, seq=?2 WHERE topic_id = ?3 AND id = ?4 AND status=?5",
            params![
                ChatLogStatus::Sent as u32, // sent
                seq,
                topic_id,
                id,
                status as u32, // pending
            ],
        )?;
        Ok(())
    }

    pub fn update_chat_log_fail(&self, topic_id: &str, id: &str) -> crate::Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE messages SET status = ?1 WHERE topic_id = ?2 AND id = ?3 AND status=?4",
            params![
                ChatLogStatus::Failed as u32, // sent
                topic_id,
                id,
                ChatLogStatus::Sending as u32, // pending
            ],
        )?;
        Ok(())
    }
}

#[test]
fn test_chat_content_decode() {
    let data = r#"{"type":"text","encrypted":true,"checksum":404}"#;
    assert!(serde_json::from_str::<Content>(data).is_ok());
}

#[test]
fn test_chat_log() {
    let db = DBStore::new(super::MEMORY_DSN);
    assert!(db.prepare().is_ok());

    let test_topic = "test_topic";
    let test_chat = "test_chat";
    let test_sender = "test_sender";
    let test_content = "test_content";

    let mut chat_log = ChatLog::new(test_topic, test_chat);
    chat_log.sender_id = test_sender.to_string();
    chat_log.content = Content::new_text(ContentType::Text, test_content);
    assert!(db
        .save_chat_log(&chat_log)
        .map_err(|e| println!("{:?}", e))
        .is_ok());

    assert_eq!(
        db.get_chat_log(test_topic, test_chat).unwrap().content.text,
        chat_log.content.text
    );

    assert_eq!(
        db.search_chat_log(test_topic, test_sender, test_content).unwrap()[0].content.text, test_content
    );

    let test_chat2: &str = "test_chat2";

    assert!(db
        .add_pending_chat_log(test_topic, test_sender, test_chat2, &chat_log.content)
        .map_err(|e| println!("{:?}", e))
        .is_ok());

    assert_eq!(
        db.get_chat_log(test_topic, test_chat2).unwrap().status,
        ChatLogStatus::Sending as u32
    );

    assert!(db
        .update_chat_log_sent(test_topic, test_chat2, 1, ChatLogStatus::Sending)
        .map_err(|e| println!("{:?}", e))
        .is_ok());

    assert_eq!(
        db.get_chat_log(test_topic, test_chat2).unwrap().status,
        ChatLogStatus::Sent as u32
    );

    let test_chat3: &str = "test_chat3";
    assert!(db
        .add_pending_chat_log(test_topic, test_sender, test_chat3, &chat_log.content)
        .map_err(|e| println!("{:?}", e))
        .is_ok());

    assert!(db
        .update_chat_log_fail(test_topic, test_chat3)
        .map_err(|e| println!("{:?}", e))
        .is_ok());

    assert_eq!(
        db.get_chat_log(test_topic, test_chat3).unwrap().status,
        ChatLogStatus::Failed as u32
    );

    chat_log.content.r#type = ContentType::Recall.into();
    assert!(db
        .add_pending_chat_log(test_topic, test_sender, test_chat3, &chat_log.content)
        .map_err(|e| println!("{:?}", e))
        .is_ok());

    assert_eq!(
        db.get_chat_log(test_topic, test_chat3).unwrap().status,
        ChatLogStatus::Sending as u32
    );
}
