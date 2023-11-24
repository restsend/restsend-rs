use super::DBStore;
use rusqlite::params;

// 本地的配置参数
pub struct Config {
    pub key: String,
    pub value: String,
}

impl DBStore {
    pub fn get_value(&self, key: &str) -> crate::Result<String> {
        let conn = self.pool.get()?;
        let kv = conn
            .query_row(
                "SELECT key, value FROM configs WHERE key = ?",
                params![key],
                |row| {
                    Ok(Config {
                        key: row.get("key")?,
                        value: row.get("value")?,
                    })
                },
            )
            .unwrap_or(Config {
                key: String::from(key),
                value: String::default(),
            });
        Ok(kv.value)
    }

    pub fn set_value(&self, key: &str, value: &str) -> crate::Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT OR REPLACE INTO configs (key, value) VALUES (?, ?)",
            params![key, value],
        )?;
        Ok(())
    }
}

#[test]
fn test_config() {
    const TEST_KEY: &str = "test_key";
    const TEST_KEY_NOT_EXIST: &str = "test_key_not_exist";
    const TEST_VALUE: &str = "test_value";
    const TEST_NEW_VALUE: &str = "test_new_value";

    let db = DBStore::new(super::MEMORY_DSN);
    assert!(db.prepare().is_ok());
    assert!(db.set_value(TEST_KEY, TEST_VALUE).is_ok());

    assert_eq!(db.get_value(TEST_KEY).unwrap(), TEST_VALUE);

    assert_eq!(db.get_value(TEST_KEY_NOT_EXIST).unwrap(), "");

    assert!(db.set_value(TEST_KEY, TEST_NEW_VALUE).is_ok());
    assert_eq!(db.get_value(TEST_KEY).unwrap(), TEST_NEW_VALUE);
}
