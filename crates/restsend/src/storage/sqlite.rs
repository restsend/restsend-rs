use super::{QueryOption, QueryResult, StoreModel};
use crate::{error::ClientError, Result};
use async_trait::async_trait;
use log::{error, info};
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
}
#[async_trait]
impl<T: StoreModel> super::Table<T> for SqliteTable<T> {
    async fn filter(
        &self,
        partition: &str,
        predicate: Box<dyn Fn(T) -> Option<T> + Send>,
    ) -> Vec<T> {
        let db = self.session.clone();
        let mut conn = db.lock().unwrap();
        let conn = conn.as_mut().unwrap();

        let stmt = format!("SELECT value FROM {} WHERE partition = ?", self.name);
        let mut stmt = conn.prepare(&stmt).unwrap();
        let rows = stmt.query(&[&partition]);
        let mut rows = match rows {
            Err(_) => {
                return vec![];
            }
            Ok(rows) => rows,
        };

        let mut items: Vec<T> = vec![];

        while let Ok(rows) = rows.next() {
            match rows {
                Some(row) => {
                    let value: String = row.get(0).unwrap();
                    match T::from_str(&value) {
                        Ok(v) => match predicate(v) {
                            Some(v) => items.push(v),
                            None => {}
                        },
                        _ => {}
                    };
                }
                None => break,
            }
        }

        items
    }
    async fn query(&self, partition: &str, option: &QueryOption) -> QueryResult<T> {
        let start_sort_value = (match option.start_sort_value {
            Some(v) => v,
            None => match self.last(partition).await {
                Some(v) => v.sort_key(),
                None => 0,
            },
        } - option.limit as i64)
            .max(0);

        let db = self.session.clone();
        let mut conn = db.lock().unwrap();
        let conn = conn.as_mut().unwrap();

        let stmt = format!(
            "SELECT value FROM {} WHERE partition = ? AND sort_by > ? ORDER BY sort_by ASC LIMIT ?",
            self.name
        );

        let mut stmt = conn.prepare(&stmt).unwrap();
        let rows = stmt.query(&[
            &partition,
            format!("{}", start_sort_value).as_str(),
            format!("{}", option.limit).as_str(),
        ]);

        let mut rows = match rows {
            Err(_) => {
                return QueryResult {
                    start_sort_value: start_sort_value,
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
        items.reverse();
        let (start_sort_value, end_sort_value) = if items.len() > 0 {
            (
                items.first().unwrap().sort_key(),
                items.last().unwrap().sort_key(),
            )
        } else {
            (0, 0)
        };

        QueryResult {
            start_sort_value,
            end_sort_value,
            items,
        }
    }

    async fn get(&self, partition: &str, key: &str) -> Option<T> {
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
            Err(_) => {
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
            Err(_) => None,
        }
    }

    async fn set(&self, partition: &str, key: &str, value: Option<&T>) {
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
                match r {
                    Ok(_) => {}
                    Err(e) => {
                        info!("{} set {} failed: {}", self.name, key, e);
                    }
                }
            }
            None => {
                self.remove(partition, key).await;
            }
        }
    }

    async fn remove(&self, partition: &str, key: &str) {
        let db = self.session.clone();
        let mut conn = db.lock().unwrap();
        let conn = conn.as_mut().unwrap();

        let stmt = format!("DELETE FROM {} WHERE partition = ? AND key = ?", self.name);
        let mut stmt = conn.prepare(&stmt).unwrap();
        let r = stmt.execute(&[&partition, &key]);
        match r {
            Ok(_) => {}
            Err(e) => {
                info!("{} remove {} failed: {}", self.name, key, e);
            }
        }
    }

    async fn last(&self, partition: &str) -> Option<T> {
        let db = self.session.clone();
        let mut conn = db.lock().unwrap();
        let conn = conn.as_mut().unwrap();

        let stmt = format!(
            "SELECT value FROM {} WHERE partition = ? ORDER BY sort_by DESC LIMIT 1",
            self.name
        );
        let mut stmt = conn.prepare(&stmt).unwrap();
        let rows = stmt.query(&[&partition]);
        let mut rows = match rows {
            Err(_) => {
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
            Err(_) => None,
        }
    }
    async fn clear(&self) {
        let db = self.session.clone();
        let mut conn = db.lock().unwrap();
        let conn = conn.as_mut().unwrap();

        let stmt = format!("DELETE FROM {}", self.name);
        let mut stmt = conn.prepare(&stmt).unwrap();
        let r = stmt.execute([]);
        match r {
            Ok(_) => {}
            Err(_) => {}
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

#[tokio::test]
async fn test_store_i32() {
    let storage = SqliteStorage::new(":memory:");
    storage.make_table("tests").unwrap();

    let t = storage.table::<i32>("tests");
    t.set("", "1", Some(&1)).await;
    t.set("", "2", Some(&2)).await;

    let not_exist_3 = t.get("", "3").await;
    assert_eq!(not_exist_3, None);
    let value_2 = t.get("", "2").await;
    assert_eq!(value_2, Some(2));

    t.remove("", "2").await;
    let value_2 = t.get("", "2").await;
    assert_eq!(value_2, None);

    t.clear().await;
    let value_1 = t.get("", "1").await;
    assert_eq!(value_1, None);
}
#[tokio::test]
async fn test_sqlite_query() {
    let storage = SqliteStorage::new(":memory:");
    storage.make_table("test").unwrap();
    let table = storage.table::<i32>("test");
    for i in 0..500 {
        table.set("", &i.to_string(), Some(&i)).await;
    }
    {
        let v = table
            .query(
                "",
                &QueryOption {
                    start_sort_value: None,
                    limit: 10,
                    keyword: None,
                },
            )
            .await;

        assert_eq!(v.items.len(), 10);
        assert_eq!(v.start_sort_value, 499);
        assert_eq!(v.end_sort_value, 490);

        assert_eq!(v.items[0], 499);
        assert_eq!(v.items[9], 490);
    }
    {
        let v = table
            .query(
                "",
                &QueryOption {
                    start_sort_value: Some(490),
                    limit: 10,
                    keyword: None,
                },
            )
            .await;

        assert_eq!(v.items.len(), 10);
        assert_eq!(v.start_sort_value, 490);
        assert_eq!(v.end_sort_value, 481);

        assert_eq!(v.items[0], 490);
        assert_eq!(v.items[9], 481);
    }
    {
        let v = table
            .query(
                "",
                &QueryOption {
                    start_sort_value: Some(480),
                    limit: 10,
                    keyword: None,
                },
            )
            .await;

        assert_eq!(v.items.len(), 10);
        assert_eq!(v.start_sort_value, 480);
        assert_eq!(v.end_sort_value, 471);

        assert_eq!(v.items[0], 480);
        assert_eq!(v.items[9], 471);
    }
}
