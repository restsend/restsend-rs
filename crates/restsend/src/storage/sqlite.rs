use super::{StoreModel, MEMORY_DSN};
use anyhow::{anyhow, Result};
use log::{debug, error};
use rusqlite::{params, Connection};
use std::sync::{Arc, Mutex};

type Session = Arc<Mutex<Option<Connection>>>;
pub struct SqliteStorage {
    conn: Session,
}

impl SqliteStorage {
    pub fn new(name: &str) -> Self {
        let conn = Connection::open(name);
        match conn {
            Err(e) => {
                error!("open sqlite connection failed: {} -> {}", name, e);
                return SqliteStorage {
                    conn: Arc::new(Mutex::new(None)),
                };
            }
            Ok(conn) => {
                return SqliteStorage {
                    conn: Arc::new(Mutex::new(Some(conn))),
                };
            }
        }
    }

    pub fn make_table(&self, name: &str) -> Result<()> {
        let db = self.conn.clone();
        let mut conn = db.lock().unwrap();
        if conn.is_none() {
            return Err(anyhow!("sqlite connection is not opened"));
        }
        let conn = conn.as_mut().unwrap();
        let create_sql = format!(
            "CREATE TABLE IF NOT EXISTS {0} (partition TEXT, key TEXT, value TEXT, sort_by INTEGER);
            CREATE UNIQUE INDEX IF NOT EXISTS idx_{0}_partition_ikey ON {0} (partition, key);
            CREATE INDEX IF NOT EXISTS idx_{0}_sort_by ON {0} (sort_by);",
            name
        );
        conn.execute_batch(&create_sql)?;
        Ok(())
    }

    pub fn table<T>(&self, name: &str) -> Option<Box<dyn super::Table<T>>>
    where
        T: StoreModel + 'static,
    {
        let table = SqliteTable::new(self.conn.clone(), name);
        Some(Box::new(table))
    }
}

struct SqliteTable<T>
where
    T: StoreModel,
{
    session: Session,
    name: String,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: StoreModel> SqliteTable<T> {
    pub fn new(conn: Session, name: &str) -> Self {
        SqliteTable {
            session: conn,
            name: name.to_string(),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T: StoreModel> super::Table<T> for SqliteTable<T> {
    fn get(&mut self, partition: &str, key: &str) -> Option<T> {
        let db = self.session.clone();
        let mut conn = db.lock().unwrap();
        let conn = conn.as_mut().unwrap();

        let stmt = format!(
            "SELECT value FROM {} WHERE partition = ? AND key = ?",
            self.name
        );
        let mut stmt = conn.prepare(&stmt).unwrap();
        let mut rows = stmt.query(&[&partition, &key]).unwrap();

        match rows.next() {
            Ok(rows) => match rows {
                Some(row) => {
                    let value: String = row.get(0).unwrap();
                    match T::from_str(&value) {
                        Ok(v) => Some(v),
                        _ => None,
                    }
                }
                None => None,
            },
            Err(e) => {
                debug!("{} get {} failed: {}", self.name, key, e);
                None
            }
        }
    }

    fn set(&mut self, partition: &str, key: &str, value: Option<T>) {
        match value {
            Some(v) => {
                let db = self.session.clone();
                let mut conn = db.lock().unwrap();
                let conn = conn.as_mut().unwrap();

                let stmt = format!(
                    "INSERT OR REPLACE INTO {} (partition, key, value, sort_by) VALUES (?, ?, ?, ?)",
                    self.name
                );
                let mut stmt = conn.prepare(&stmt).unwrap();
                let r = stmt.execute(params![&partition, &key, &v.to_string(), v.sort_key()]);
                match r {
                    Ok(_) => {}
                    Err(e) => {
                        debug!("{} set {} failed: {}", self.name, key, e);
                    }
                }
            }
            None => {
                self.remove(partition, key);
            }
        }
    }

    fn remove(&mut self, partition: &str, key: &str) {
        let db = self.session.clone();
        let mut conn = db.lock().unwrap();
        let conn = conn.as_mut().unwrap();

        let stmt = format!("DELETE FROM {} WHERE partition = ? AND key = ?", self.name);
        let mut stmt = conn.prepare(&stmt).unwrap();
        let r = stmt.execute(&[&partition, &key]);
        match r {
            Ok(_) => {}
            Err(e) => {
                debug!("{} remove {} failed: {}", self.name, key, e);
            }
        }
    }
    fn clear(&mut self) {
        let db = self.session.clone();
        let mut conn = db.lock().unwrap();
        let conn = conn.as_mut().unwrap();

        let stmt = format!("DELETE FROM {}", self.name);
        let mut stmt = conn.prepare(&stmt).unwrap();
        let r = stmt.execute([]);
        match r {
            Ok(_) => {}
            Err(e) => {
                debug!("{} clear failed: {}", self.name, e);
            }
        }
    }
}

#[cfg(test)]
impl StoreModel for i32 {
    fn sort_key(&self) -> i64 {
        *self as i64
    }
}

#[test]
pub fn test_prepare() {
    let test_file = "test_prepare.db";
    {
        let storage = SqliteStorage::new(test_file);
        storage.make_table("tests").unwrap();
    }
    {
        let storage = SqliteStorage::new(test_file);
        storage.make_table("tests").expect("prepare sqlite failed");
    }
    std::fs::remove_file(test_file).unwrap_or(());
}

#[test]
pub fn test_store_i32() {
    let storage = SqliteStorage::new(MEMORY_DSN);
    storage.make_table("tests").unwrap();

    let mut t = storage.table::<i32>("tests").unwrap();
    t.set("", "1", Some(1));
    t.set("", "2", Some(2));

    let not_exist_3 = t.get("", "3");
    assert_eq!(not_exist_3, None);
    let value_2 = t.get("", "2");
    assert_eq!(value_2, Some(2));

    t.remove("", "2");
    let value_2 = t.get("", "2");
    assert_eq!(value_2, None);

    t.clear();
    let value_1 = t.get("", "1");
    assert_eq!(value_1, None);
}
