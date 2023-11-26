use super::{Content, DBStore, Topic};
use log::warn;
use serde::{Deserialize, Serialize};

//会话信息
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    pub owner_id: String,
    pub topic_id: String,

    #[serde(default)]
    pub last_seq: u64,

    #[serde(default)]
    pub last_read_seq: u64, //最后一条已读消息的seq

    #[serde(default)]
    pub multiple: bool, //是否群聊

    #[serde(default)]
    pub attendee: String, // 如果是单聊, 则是对方的id

    #[serde(default)]
    pub name: String,

    #[serde(default)]
    pub icon: String,

    #[serde(default)]
    pub sticky: bool, //是否置顶

    #[serde(default)]
    pub mute: bool, //是否静音

    #[serde(default)]
    pub source: String,

    #[serde(default)]
    pub unread: u64, //未读消息数量

    #[serde(default)]
    pub last_sender_id: String, //最后一条消息的发送者

    #[serde(default)]
    pub last_message: Option<Content>, // 最后一条消息

    #[serde(default)]
    pub last_message_at: String, // 最后一条消息的时间

    #[serde(skip)]
    pub cached_at: String,
}

impl Conversation {
    pub fn new(topic_id: &str) -> Self {
        Conversation {
            topic_id: String::from(topic_id),
            ..Default::default()
        }
    }
}

impl From<&Topic> for Conversation {
    fn from(topic: &Topic) -> Conversation {
        Conversation {
            topic_id: topic.id.clone(),
            owner_id: topic.owner_id.clone(),
            last_seq: topic.last_seq,
            multiple: topic.multiple,
            source: topic.source.clone(),
            name: topic.name.clone(),
            icon: topic.icon.clone(),
            attendee: topic.attendee_id.clone(),
            ..Default::default()
        }
    }
}

// impl DBStore {
//     pub fn get_conversations_count(&self) -> Result<u32> {
//         let conn = self.pool.get()?;
//         let count = conn.query_row("SELECT COUNT(*) FROM conversations", [], |row| {
//             let count: u32 = row.get(0)?;
//             Ok(count)
//         })?;
//         Ok(count)
//     }

//     pub fn get_conversation(&self, topic_id: &str) -> Result<Conversation> {
//         let conn = self.pool.get()?;
//         let conversation = conn.query_row(
//             "SELECT * FROM conversations WHERE topic_id = ?",
//             params![topic_id],
//             |row| {
//                 let mut conversation = Conversation::new(&row.get::<_, String>("topic_id")?);
//                 conversation.owner_id = row.get("owner_id")?;
//                 conversation.cached_at = row.get("cached_at")?;
//                 conversation.last_seq = row.get("last_seq")?;
//                 conversation.last_read_seq = row.get("last_read_seq")?;
//                 conversation.multiple = row.get("multiple")?;
//                 conversation.attendee = row.get("attendee")?;
//                 conversation.name = row.get("name")?;
//                 conversation.icon = row.get("icon")?;
//                 conversation.sticky = row.get("sticky")?;
//                 conversation.mute = row.get("mute")?;
//                 conversation.source = row.get("source")?;
//                 conversation.last_sender_id = row.get("last_sender_id")?;

//                 let last_message: String = row.get("last_message")?;
//                 conversation.last_message = serde_json::from_str(&last_message).ok();

//                 conversation.last_message_at = row.get("last_message_at")?;
//                 conversation.unread = std::cmp::max(conversation.last_seq - conversation.last_read_seq, 0);
//                 Ok(conversation)
//             },
//         );
//         match conversation {
//             Ok(conversation) =>{
//                 Ok(conversation)
//             },
//             Err(e) => {
//                 warn!("get_conversation fail: {:?} {:?}", topic_id, e);
//                 Err(e.into())
//             }
//         }
//     }

//     pub fn save_conversation(&self, conversation: &Conversation) -> Result<()> {
//         let conn = self.pool.get()?;
//         conn.execute(
//             "INSERT OR REPLACE INTO conversations (topic_id, owner_id, cached_at, last_seq, last_read_seq, multiple, attendee, name, icon, sticky, mute, source, last_sender_id, last_message, last_message_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
//             params![conversation.topic_id,
//             conversation.owner_id,
//             conversation.cached_at,
//             conversation.last_seq,
//             conversation.last_read_seq,
//             conversation.multiple,
//             conversation.attendee,
//             conversation.name,
//             conversation.icon,
//             conversation.sticky,
//             conversation.mute,
//             conversation.source,
//             conversation.last_sender_id,
//             serde_json::to_string(&conversation.last_message)?,
//             conversation.last_message_at]
//         )?;
//         Ok(())
//     }

//     pub fn remove_conversation(&self, topic_id: &str) -> Result<()> {
//         let conn = self.pool.get()?;
//         conn.execute(
//             "DELETE FROM conversations WHERE topic_id = ?",
//             params![topic_id],
//         )?;
//         Ok(())
//     }

//     pub fn set_conversation_sticky(&self, topic_id: &str, sticky: bool) -> Result<()> {
//         let conn = self.pool.get()?;
//         conn.execute(
//             "UPDATE conversations SET sticky = ? WHERE topic_id = ?",
//             params![sticky, topic_id],
//         )?;
//         Ok(())
//     }

//     pub fn set_conversation_mute(&self, topic_id: &str, mute: bool) -> Result<()> {
//         let conn = self.pool.get()?;
//         conn.execute(
//             "UPDATE conversations SET mute = ? WHERE topic_id = ?",
//             params![mute, topic_id],
//         )?;
//         Ok(())
//     }

//     pub fn set_conversation_read(&self, topic_id: &str) -> Result<()> {
//         let conn = self.pool.get()?;
//         conn.execute(
//             "UPDATE conversations SET last_read_seq = last_seq WHERE topic_id = ?",
//             params![topic_id],
//         )?;
//         Ok(())
//     }
// }

#[test]
fn test_conversation() {
    let db = DBStore::new(super::MEMORY_DSN);
    assert!(db.prepare().is_ok());

    let test_topic_id = "test_topic_id";

    let content = Content::new_text(super::ContentType::Text, "test");
    let mut conversation = Conversation::new(test_topic_id);
    conversation.last_message = Some(content);

    assert!(db
        .save_conversation(&conversation)
        .map_err(|e| println!("{:?}", e))
        .is_ok());

    assert_eq!(
        db.get_conversation(test_topic_id).unwrap().topic_id,
        test_topic_id
    );

    assert_eq!(db.get_conversations_count().unwrap(), 1);

    assert!(db.set_conversation_sticky(test_topic_id, true).is_ok());

    assert_eq!(db.get_conversation(test_topic_id).unwrap().sticky, true);

    assert!(db.set_conversation_mute(test_topic_id, true).is_ok());

    assert!(db
        .set_conversation_read(test_topic_id)
        .map_err(|e| println!("{:?}", e))
        .is_ok());

    assert!(db.get_conversation(test_topic_id).unwrap().unread == 0);

    let test_topic_id2 = "test_topic_id2";
    let content2 = Content::new_text(super::ContentType::Text, "test2");
    let mut conversation2 = Conversation::new(test_topic_id2);
    conversation2.last_message = Some(content2);

    assert!(db
        .save_conversation(&conversation2)
        .map_err(|e| println!("{:?}", e))
        .is_ok());

    assert!(db
        .remove_conversation(test_topic_id)
        .map_err(|e| println!("{:?}", e))
        .is_ok());
}
