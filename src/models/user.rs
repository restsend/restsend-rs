use super::DBStore;
use rusqlite::params;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default)]
pub struct AuthInfo {
    pub endpoint: String,
    pub user_id: String,
    pub avatar: String,
    pub name: String,
    pub token: String,
}

//所有的用户都会存储到本地表,只要不管是不是好友
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct User {
    #[serde(default)]
    pub user_id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub avatar: String,
    #[serde(default)]
    pub public_key: String, //e2e的公钥
    #[serde(default)]
    pub remark: String, // 本地或者联系人备注
    #[serde(default)]
    pub is_contact: bool, //是否好友
    #[serde(default)]
    pub is_star: bool, //是否星标
    #[serde(default)]
    pub is_blocked: bool, //是否被拉黑
    #[serde(default)]
    pub locale: String, //语言
    #[serde(default)]
    pub city: String, //城市
    #[serde(default)]
    pub country: String, //国家
    #[serde(default)]
    pub source: String, //来源
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub gender: String, // 性别: f/female, m/male
    #[serde(skip)]
    pub cached_at: String,
}

impl User {
    pub fn new(user_id: &str) -> Self {
        User {
            user_id: String::from(user_id),
            ..Default::default()
        }
    }
    pub fn merge(&self, user: &User) -> User {
        let mut new_user = self.clone();
        if user.name != self.name {
            new_user.name = user.name.clone();
        }
        if user.avatar != String::default() {
            new_user.avatar = user.avatar.clone();
        }
        if user.public_key != String::default() {
            new_user.public_key = user.public_key.clone();
        }
        if user.remark != String::default() {
            new_user.remark = user.remark.clone();
        }
        if user.is_contact != false {
            new_user.is_contact = user.is_contact;
        }
        if user.is_star != false {
            new_user.is_star = user.is_star;
        }
        if user.is_blocked != false {
            new_user.is_blocked = user.is_blocked;
        }
        if user.locale != String::default() {
            new_user.locale = user.locale.clone();
        }
        if user.city != String::default() {
            new_user.city = user.city.clone();
        }
        if user.country != String::default() {
            new_user.country = user.country.clone();
        }
        if user.source != String::default() {
            new_user.source = user.source.clone();
        }
        if user.created_at != String::default() {
            new_user.created_at = user.created_at.clone();
        }
        new_user.cached_at = chrono::Local::now().timestamp_millis().to_string();
        new_user
    }
}

impl DBStore {
    pub fn update_user(&self, new_user: &User) {
        let old_user = self.get_user(new_user.user_id.as_str());
        if old_user.is_err() {
            self.save_user(&new_user).ok();
            return;
        }
        self.save_user(&old_user.unwrap()).ok();
    }

    pub fn save_user(&self, user: &User) -> crate::Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT OR REPLACE INTO users (user_id, name, avatar, public_key, remark, is_contact, is_star, is_blocked, locale, city, country, source, created_at, cached_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![user.user_id, user.name, user.avatar, user.public_key, user.remark, user.is_contact, user.is_star, user.is_blocked, user.locale, user.city, user.country, user.source, user.created_at, user.cached_at],
        )?;
        Ok(())
    }
    #[allow(dead_code)]
    pub fn remove_user(&self, user_id: &str) -> crate::Result<()> {
        let conn = self.pool.get()?;
        conn.execute("DELETE FROM users WHERE user_id = ?", params![user_id])?;
        Ok(())
    }

    pub fn get_user(&self, user_id: &str) -> crate::Result<User> {
        let conn = self.pool.get()?;
        let user = conn.query_row(
            "SELECT * FROM users WHERE user_id = ?",
            params![user_id],
            |row| {
                let mut user = User::new(&row.get::<_, String>("user_id")?);
                user.name = row.get("name")?;
                user.avatar = row.get("avatar")?;
                user.public_key = row.get("public_key")?;
                user.remark = row.get("remark")?;
                user.is_contact = row.get("is_contact")?;
                user.is_star = row.get("is_star")?;
                user.is_blocked = row.get("is_blocked")?;
                user.locale = row.get("locale")?;
                user.city = row.get("city")?;
                user.country = row.get("country")?;
                user.source = row.get("source")?;
                user.created_at = row.get("created_at")?;
                user.cached_at = row.get("cached_at")?;
                Ok(user)
            },
        )?;
        Ok(user)
    }
}

impl DBStore {
    pub fn set_user_star(&self, user_id: &str, is_star: bool) -> crate::Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE users SET is_star = ?1 WHERE user_id = ?2",
            params![is_star, user_id],
        )?;
        Ok(())
    }

    pub fn set_user_remark(&self, user_id: &str, remark: &str) -> crate::Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE users SET remark = ?1 WHERE user_id = ?2",
            params![remark, user_id],
        )?;
        Ok(())
    }

    pub fn set_user_block(&self, user_id: &str, is_blocked: bool) -> crate::Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE users SET is_blocked = ?1 WHERE user_id = ?2",
            params![is_blocked, user_id],
        )?;
        Ok(())
    }
}

#[test]
fn test_user() {
    // basic
    let db = DBStore::new(super::MEMORY_DSN);
    assert!(db.prepare().is_ok());

    let test_user = "test_user";
    let user = User::new(test_user);

    db.save_user(&user).expect("save user failed");

    assert_eq!(db.get_user(test_user).unwrap().user_id, test_user);
    db.set_user_block(test_user, true).expect("set blocked");
    assert_eq!(db.get_user(test_user).unwrap().is_blocked, true);
    // clean
    db.remove_user(test_user).expect("remove user failed");
}
