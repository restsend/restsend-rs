/*
    Tables:
     - configs: 处理这个账号的配置信息
     - conversations: 会话表，*需要和服务器同步*， 用来记录会话的信息， 会话的类型， 会话的名称， 会话的头像， 会话的最后一条消息， 会话的未读消息数
     - messages: 消息表
     - users: 用户（联系人和群成员）缓存信息 *联系人和群成员需要分别和服务器同步*
     - topics: topic信息表
     - topic_members: 群成员表， 用来记录用户和群的关系
*/

use anyhow::Result;
use serde::{Deserialize, Serialize};
pub const MEMORY_DSN: &str = ":memory:";

pub struct DBStore {}

impl DBStore {
    pub fn new(path: &str) -> Self {
        // let manager = if path == MEMORY_DSN {
        //     SqliteConnectionManager::memory().with_init(|_conn| {
        //         #[cfg(test)]
        //         _conn.trace(Some(|trace| {
        //             println!("SQL: {}", trace);
        //         }));
        //         Ok(())
        //     })
        // } else {
        //     SqliteConnectionManager::file(path)
        // };

        DBStore {}
    }

    pub fn prepare(&self) -> Result<()> {
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ListUserResult {
    pub has_more: bool,
    pub updated_at: String,
    #[serde(default)]
    pub items: Vec<User>,
    #[serde(default)]
    pub removed: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ListConversationResult {
    pub has_more: bool,
    pub updated_at: String,
    #[serde(default)]
    pub items: Vec<Conversation>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ListChatLogResult {
    pub has_more: bool,
    pub updated_at: String,
    pub last_seq: u64,
    #[serde(default)]
    pub items: Vec<ChatLog>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TopicKnock {
    pub created_at: String,
    pub updated_at: String,
    pub topic_id: String,
    pub user_id: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub source: String,
    pub status: String,
    #[serde(default)]
    pub admin_id: String,
}

impl TopicKnock {
    pub fn new(topic_id: &str, user_id: &str) -> Self {
        TopicKnock {
            created_at: String::default(),
            updated_at: String::default(),
            topic_id: String::from(topic_id),
            user_id: String::from(user_id),
            message: String::default(),
            source: String::default(),
            status: String::default(),
            admin_id: String::default(),
        }
    }
}

pub mod chat_log;
pub mod conversation;
pub mod topic;
pub mod topic_member;
pub mod user;

pub use chat_log::ChatLog;
pub use chat_log::Content;
pub use chat_log::ContentType;
pub use conversation::Conversation;
pub use topic::Topic;
pub use topic::TopicNotice;
pub use topic_member::TopicMember;
pub use user::{AuthInfo, User};
