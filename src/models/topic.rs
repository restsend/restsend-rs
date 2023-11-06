use super::DBStore;
use rusqlite::params;
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TopicNotice {
    pub text: String,
    #[serde(default)]
    pub publisher: String,
    #[serde(default)]
    pub updated_at: String,
}

impl TopicNotice {
    pub fn new(text: &str, publisher: &str, updated_at: &str) -> Self {
        TopicNotice {
            text: String::from(text),
            publisher: String::from(publisher),
            updated_at: String::from(updated_at),
        }
    }
}

impl FromSql for TopicNotice {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        // unmarshal from json's string
        let s = String::column_result(value)?;
        let v = serde_json::from_str(&s).map_err(|_e| FromSqlError::InvalidType)?;
        Ok(v)
    }
}

impl ToSql for TopicNotice {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        let s = serde_json::to_string(&self)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        Ok(ToSqlOutput::from(s))
    }
}

fn column_to_strings(value: &str) -> FromSqlResult<Vec<String>> {
    let v = serde_json::from_str(value).map_err(|_e| FromSqlError::InvalidType)?;
    Ok(v)
}

fn strings_to_column(v: &Vec<String>) -> rusqlite::Result<ToSqlOutput<'_>> {
    let s = serde_json::to_string(&v)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
    Ok(ToSqlOutput::from(s))
}

#[derive(Serialize, Deserialize, Debug,Default)]
#[serde(rename_all = "camelCase")]
pub struct Topic {
    // 群id
    pub id: String,
    #[serde(default)]
    pub name: String,         // 群名称
    #[serde(default)]
    pub icon: String,         // 群头像
    #[serde(default)]
    pub remark: String,       // 备注
    #[serde(default)]
    pub owner_id: String,     // 群主
    #[serde(default)]
    pub attendee_id: String,  // 如果是单聊, 则是对方的id
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub admins: Vec<String>,  // 群聊的管理员
    #[serde(default)]
    pub members: u32,         // 成员数量
    #[serde(default)]
    pub last_seq: u64,        // 最后一条消息的seq
    #[serde(default)]
    pub multiple: bool,       //是否群聊
    #[serde(default)]
    pub private: bool,        // 是否私聊
    #[serde(default)]
    pub created_at: String,   // 创建时间
    #[serde(default)]
    pub updated_at: String,   // 创建时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notice: Option<TopicNotice>, // 群公告
    #[serde(default)]
    pub silent: bool,        // 是否禁言
    #[serde(skip)]
    pub cached_at: String,   // 缓存时间
    #[serde(skip)]
    pub unread: u32,         // 未读消息数量
}

impl Topic {
    pub fn new(topic_id: &str) -> Self {
        Topic {
            id: String::from(topic_id),
            ..Default::default()
        }
    }
}

impl DBStore {
    pub fn get_topic(&self, id: &str) -> crate::Result<Topic> {
        let conn = self.pool.get()?;
        let topic = conn.query_row("SELECT * FROM topics WHERE id = ?", params![id], |row| {
            let mut topic = Topic::new(&row.get::<_, String>("id")?);
            topic.name = row.get("name")?;
            topic.icon = row.get("icon")?;
            topic.remark = row.get("remark")?;
            topic.owner_id = row.get("owner_id")?;
            topic.attendee_id = row.get("attendee_id")?;
            topic.admins = column_to_strings(&row.get::<_, String>("admins")?)?;
            topic.members = row.get("members")?;
            topic.last_seq = row.get("last_seq")?;
            topic.multiple = row.get("multiple")?;
            topic.private = row.get("private")?;
            topic.notice = row.get("notice")?;
            topic.silent = row.get("silent")?;
            topic.created_at = row.get("created_at")?;
            topic.cached_at = row.get("cached_at")?;
            topic.unread = row.get("unread")?;
            Ok(topic)
        })?;
        Ok(topic)
    }

    pub fn save_topic(&self, topic: &Topic) -> crate::Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT OR REPLACE INTO topics (id, name, icon, remark, owner_id, attendee_id, admins, members, last_seq, multiple, private, notice, silent, unread, created_at, cached_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13 ,?14, ?15, ?16)",
            params![topic.id, 
            topic.name, 
            topic.icon, 
            topic.remark,
            topic.owner_id, 
            topic.attendee_id, 
            strings_to_column(&topic.admins)?,
            topic.members, 
            topic.last_seq, 
            topic.multiple, 
            topic.private, 
            topic.notice, 
            topic.silent,
            topic.unread,
            topic.created_at,
            topic.cached_at,]
        ).map(|_|{Ok(())})?
    }

    pub fn update_topic_notice(&self, id: &str, notice: Option<TopicNotice>) -> crate::Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE topics SET notice = ? WHERE id = ?",
            params![notice, id],
        ).map(|_|{Ok(())})?
    }

    pub fn update_topic_read(&self, id: &str) -> crate::Result<()> {
        let conn = self.pool.get()?;
        conn.execute("UPDATE topics SET unread = ? WHERE id = ?", params![0, id])
        .map(|_|{Ok(())})?
    }

    pub fn get_topic_admins(&self, id: &str) -> crate::Result<Vec<String>> {
        let conn = self.pool.get()?;
        let admins = conn.query_row(
            "SELECT admins FROM topics WHERE id = ?",
            params![id],
            |row| {
                let admins = column_to_strings(&row.get::<_, String>("admins")?)?;
                Ok(admins)
            },
        )?;
        Ok(admins)
    }

    pub fn get_topic_owner(&self, id: &str) -> crate::Result<String> {
        let conn = self.pool.get()?;
        let owner = conn.query_row(
            "SELECT owner_id FROM topics WHERE id = ?",
            params![id],
            |row| Ok(row.get::<_, String>("owner_id")?),
        )?;
        Ok(owner)
    }

    pub fn silent_topic(&self, id: &str, silent: bool) -> crate::Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE topics SET silent = ? WHERE id = ?",
            params![silent, id],
        )?;
        Ok(())
    }

    pub fn dismiss_topic(&self, id: &str) -> crate::Result<()> {
        let conn = self.pool.get()?;
        conn.execute("DELETE FROM topics WHERE id = ?", params![id])?;
        Ok(())
    }
}

#[test]
fn test_vec_string_decode() {
    use rusqlite::types::ToSql;

    let src = r#"["a","b","c"]"#;
    let value = serde_json::from_str::<Vec<String>>(src).unwrap();
    let r = strings_to_column(&value);
    assert!(r.is_ok());
    let r = r.unwrap();
    let r = r.to_sql();
    assert!(r.is_ok());
}

#[test]
fn test_topic() {
    let db = DBStore::new(super::MEMORY_DSN);
    assert!(db.prepare().is_ok());

    let test_topic = "test_topic";
    let test_owner = "test_user";
    let test_admin = "admin";

    let mut topic = Topic::new(test_topic);
    topic.admins = vec![test_owner.to_string(), test_admin.to_string()];
    topic.owner_id = test_owner.to_string();
    topic.silent = true;

    assert!(db
        .save_topic(&topic)
        .map_err(|e| println!("{:?}", e))
        .is_ok());
    assert_eq!(db.get_topic(test_topic).unwrap().id, test_topic);

    let admins = db.get_topic_admins(test_topic).unwrap();
    assert!(
        admins[0] == test_owner && admins[1] == test_admin
            || admins[1] == test_owner && admins[0] == test_admin
    );

    assert_eq!(db.get_topic_owner(test_topic).unwrap(), test_owner);

    assert!(db
        .silent_topic(test_topic, true)
        .map_err(|e| println!("{:?}", e))
        .is_ok());
    assert_eq!(db.get_topic(test_topic).unwrap().silent, true);

    let notice = Some(TopicNotice::new(
        "notice text",
        test_owner,
        &chrono::Utc::now().to_rfc3339(),
    ));

    assert!(db
        .update_topic_notice(test_topic, notice)
        .map_err(|e| println!("{:?}", e))
        .is_ok());
    assert_eq!(
        db.get_topic(test_topic).unwrap().notice.unwrap().text,
        "notice text"
    );

    assert!(db
        .dismiss_topic(test_topic)
        .map_err(|e| println!("{:?}", e))
        .is_ok());
}
