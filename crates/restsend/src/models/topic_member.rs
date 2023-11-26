use super::DBStore;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct TopicMember {
    pub topic_id: String,
    pub user_id: String,
    #[serde(default)]
    pub is_owner: bool, // 是否群主
    #[serde(default)]
    pub is_admin: bool, // 是否管理员
    #[serde(default)]
    pub remark: String, // 在群里面的备注
    #[serde(default)]
    pub silent: bool, // 是否禁言
    #[serde(default)]
    pub joined_at: String,
    #[serde(skip)]
    pub cached_at: String,
}

impl TopicMember {
    pub fn new(topic_id: &str, user_id: &str) -> Self {
        TopicMember {
            topic_id: String::from(topic_id),
            user_id: String::from(user_id),
            ..Default::default()
        }
    }
}

// impl DBStore {
//     #[allow(unused)]
//     pub fn get_topic_member(&self, topic_id: &str, user_id: &str) -> Result<TopicMember> {
//         let conn = self.pool.get()?;
//         let topic_member = conn.query_row(
//             "SELECT * FROM topic_members WHERE topic_id = ? AND user_id = ?",
//             params![topic_id, user_id],
//             |row| {
//                 Ok(TopicMember {
//                     topic_id: row.get("topic_id")?,
//                     user_id: row.get("user_id")?,
//                     is_owner: row.get("is_owner")?,
//                     is_admin: row.get("is_admin")?,
//                     remark: row.get("remark")?,
//                     silent: row.get("silent")?,
//                     joined_at: row.get("joined_at")?,
//                     cached_at: row.get("cached_at")?,
//                 })
//             },
//         )?;
//         Ok(topic_member)
//     }

//     pub fn save_topic_member(&self, topic_member: &TopicMember) -> Result<()> {
//         let conn = self.pool.get()?;
//         conn.execute(
//             "INSERT OR REPLACE INTO topic_members (topic_id, user_id, is_owner, is_admin, remark, silent, joined_at, cached_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
//             params![topic_member.topic_id, topic_member.user_id, topic_member.is_owner, topic_member.is_admin, topic_member.remark, topic_member.silent,  topic_member.joined_at, topic_member.cached_at]
//         )?;
//         Ok(())
//     }

//     pub fn remove_topic_member(&self, topic_id: &str, user_id: &str) -> Result<()> {
//         let conn = self.pool.get()?;
//         conn.execute(
//             "DELETE FROM topic_members WHERE topic_id = ? AND user_id = ?",
//             params![topic_id, user_id],
//         )?;
//         Ok(())
//     }

//     pub fn silent_topic_member(
//         &self,
//         topic_id: &str,
//         user_id: &str,
//         silent: bool,
//     ) -> Result<()> {
//         let conn = self.pool.get()?;
//         conn.execute(
//             "UPDATE topic_members SET silent = ? WHERE topic_id = ? AND user_id = ?",
//             params![silent, topic_id, user_id],
//         )?;
//         Ok(())
//     }
// }

#[test]
fn test_topic_member() {
    // basic
    let db = DBStore::new(super::MEMORY_DSN);
    assert!(db.prepare().is_ok());

    let test_topic = "test_topic";
    let test_member = "test_member";
    let topic_member = TopicMember::new(test_topic, test_member);

    assert!(db
        .save_topic_member(&topic_member)
        .map_err(|e| println!("{:?}", e))
        .is_ok());

    assert!(
        db.get_topic_member(test_topic, test_member)
            .unwrap()
            .user_id
            == test_member
    );

    assert!(db
        .silent_topic_member(test_topic, test_member, true)
        .map_err(|e| println!("{:?}", e))
        .is_ok());

    assert_eq!(
        db.get_topic_member(test_topic, test_member).unwrap().silent,
        true
    );

    assert!(db
        .remove_topic_member(test_topic, test_member)
        .map_err(|e| println!("{:?}", e))
        .is_ok());
    assert!(db
        .remove_topic_member(test_topic, test_member)
        .map_err(|e| println!("{:?}", e))
        .is_ok());
}
