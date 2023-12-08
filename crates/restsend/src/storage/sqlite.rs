use super::{QueryOption, QueryResult, StoreModel};
use crate::{error::ClientError, Result};
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
            return Err(ClientError::Storage(
                "sqlite connection is not opened".to_string(),
            ));
        }
        let conn = conn.as_mut().unwrap();
        let create_sql = format!(
            "CREATE TABLE IF NOT EXISTS {0} (partition TEXT, key TEXT, value TEXT, sort_by INTEGER);
            CREATE UNIQUE INDEX IF NOT EXISTS idx_{0}_partition_ikey ON {0} (partition, key);
            CREATE INDEX IF NOT EXISTS idx_{0}_sort_by ON {0} (sort_by);",
            name
        );
        conn.execute_batch(&create_sql)
            .map_err(|e| ClientError::Storage(format!("create table {} failed: {}", name, e)))?;
        Ok(())
    }

    pub fn table<T>(&self, name: &str) -> Box<dyn super::Table<T>>
    where
        T: StoreModel + 'static,
    {
        let table = SqliteTable::new(self.conn.clone(), name);
        Box::new(table)
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

    fn get_total(&self, partition: &str) -> u32 {
        let db = self.session.clone();
        let mut conn = db.lock().unwrap();
        let conn = conn.as_mut().unwrap();

        let stmt = format!("SELECT COUNT(*) FROM {} WHERE partition = ?", self.name);
        let mut stmt = conn.prepare(&stmt).unwrap();
        let rows = stmt.query(&[&partition]);

        let mut rows = match rows {
            Err(e) => {
                debug!("{} query {} failed: {}", self.name, partition, e);
                return 0;
            }
            Ok(rows) => rows,
        };

        match rows.next() {
            Ok(rows) => match rows {
                Some(row) => {
                    let value: u32 = row.get(0).unwrap();
                    value
                }
                None => 0,
            },
            Err(e) => {
                debug!("{} get {} failed: {}", self.name, partition, e);
                0
            }
        }
    }
}

impl<T: StoreModel> super::Table<T> for SqliteTable<T> {
    fn query(&self, partition: &str, option: &QueryOption) -> QueryResult<T> {
        let total: u32 = self.get_total(partition);

        let db = self.session.clone();
        let mut conn = db.lock().unwrap();
        let conn = conn.as_mut().unwrap();

        let stmt = format!(
            "SELECT value FROM {} WHERE partition = ? AND sort_by > ? ORDER BY sort_by DESC LIMIT ?",
            self.name
        );

        let mut stmt = conn.prepare(&stmt).unwrap();
        let rows = stmt.query(&[
            &partition,
            format!("{}", option.start_sort_value).as_str(),
            format!("{}", option.limit).as_str(),
        ]);

        let mut rows = match rows {
            Err(e) => {
                debug!("{} query {} failed: {}", self.name, partition, e);
                return QueryResult {
                    total: 0,
                    start_sort_value: option.start_sort_value,
                    end_sort_value: 0,
                    items: vec![],
                };
            }
            Ok(rows) => rows,
        };

        let mut items: Vec<T> = vec![];

        while let Ok(rows) = rows.next() {
            match rows {
                Some(row) => {
                    let value: String = row.get(0).unwrap();
                    match T::from_str(&value) {
                        Ok(v) => items.push(v),
                        _ => {}
                    };
                }
                None => break,
            }
        }

        let (start_sort_value, end_sort_value) = if items.len() > 0 {
            (
                items.first().unwrap().sort_key(),
                items.last().unwrap().sort_key(),
            )
        } else {
            (0, 0)
        };

        QueryResult {
            total,
            start_sort_value,
            end_sort_value,
            items,
        }
    }

    fn get(&self, partition: &str, key: &str) -> Option<T> {
        let db = self.session.clone();
        let mut conn = db.lock().unwrap();
        let conn = conn.as_mut().unwrap();

        let stmt = format!(
            "SELECT value FROM {} WHERE partition = ? AND key = ?",
            self.name
        );
        let mut stmt = conn.prepare(&stmt).unwrap();
        let rows = stmt.query(&[&partition, &key]);
        let mut rows = match rows {
            Err(e) => {
                debug!("{} query {} failed: {}", self.name, key, e);
                return None;
            }
            Ok(rows) => rows,
        };

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

    fn set(&self, partition: &str, key: &str, value: Option<T>) {
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
                let value = v.to_string();
                let r = stmt.execute(params![&partition, &key, &value, v.sort_key()]);
                debug!(
                    "{} set partition:{} key:{} value:{:?}",
                    self.name, partition, key, value
                );
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

    fn remove(&self, partition: &str, key: &str) {
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
    fn clear(&self) {
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
    let storage = SqliteStorage::new(":memory:");
    storage.make_table("tests").unwrap();

    let t = storage.table::<i32>("tests");
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
