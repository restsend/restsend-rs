use super::{QueryOption, QueryResult, StoreModel};
use crate::error::ClientError;
use async_trait::async_trait;
use rusqlite::{params, Connection};
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

type Session = Arc<Mutex<Option<Connection>>>;
pub struct SqliteStorage {
    tables: Mutex<HashSet<String>>,
    conn: Session,
}

impl SqliteStorage {
    pub fn new(db_name: &str) -> Self {
        let conn = Connection::open(db_name).ok();
        SqliteStorage {
            tables: Mutex::new(HashSet::new()),
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    pub async fn new_async(db_name: &str) -> Self {
        Self::new(db_name)
    }

    fn make_table<T>(&self) -> crate::Result<()> {
        let db = self.conn.clone();
        let mut conn = db.lock().unwrap();
        if conn.is_none() {
            return Err(ClientError::Storage(
                "sqlite connection is not opened".to_string(),
            ));
        }
        let conn = conn.as_mut().unwrap();
        let tbl_name = super::table_name::<T>();
        let create_sql = format!(
            "CREATE TABLE IF NOT EXISTS {0} (partition TEXT, key TEXT, value TEXT, sort_by INTEGER);
            CREATE UNIQUE INDEX IF NOT EXISTS idx_{0}_partition_ikey ON {0} (partition, key);
            CREATE INDEX IF NOT EXISTS idx_{0}_sort_by ON {0} (sort_by);",
            tbl_name
        );
        conn.execute_batch(&create_sql).map_err(|e| {
            ClientError::Storage(format!("create table {} failed: {}", tbl_name, e))
        })?;
        self.tables.lock().unwrap().insert(tbl_name.clone());
        Ok(())
    }

    pub fn table<T>(&self) -> Box<dyn super::Table<T>>
    where
        T: StoreModel + 'static,
    {
        let tbl_name = super::table_name::<T>();
        if self.tables.lock().unwrap().get(&tbl_name).is_none() {
            self.make_table::<T>().unwrap();
        }
        let table = SqliteTable::new(self.conn.clone(), &tbl_name);
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
    ) -> Option<Vec<T>> {
        let db = self.session.clone();
        let mut conn = db.lock().unwrap();
        let conn = conn.as_mut()?;

        let stmt = format!("SELECT value FROM {} WHERE partition = ?", self.name);
        let mut stmt = conn.prepare(&stmt).ok()?;
        let rows = stmt.query(&[&partition]);
        let mut rows = match rows {
            Err(_) => {
                return None;
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

        Some(items)
    }
    async fn query(&self, partition: &str, option: &QueryOption) -> Option<QueryResult<T>> {
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
        let mut rows = stmt
            .query(&[
                &partition,
                format!("{}", start_sort_value).as_str(),
                format!("{}", option.limit).as_str(),
            ])
            .ok()?;

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

        Some(QueryResult {
            start_sort_value,
            end_sort_value,
            items,
        })
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

    async fn set(&self, partition: &str, key: &str, value: Option<&T>) -> crate::Result<()> {
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
                stmt.execute(params![&partition, &key, &value, v.sort_key()])
                    .map(|_| ())
                    .map_err(|e| {
                        ClientError::Storage(format!("{} set {} failed: {}", self.name, key, e))
                    })
            }
            None => self.remove(partition, key).await,
        }
    }

    async fn remove(&self, partition: &str, key: &str) -> crate::Result<()> {
        let db = self.session.clone();
        let mut conn = db.lock().unwrap();
        let conn = conn.as_mut().unwrap();

        let stmt = format!("DELETE FROM {} WHERE partition = ? AND key = ?", self.name);
        let mut stmt = conn.prepare(&stmt).unwrap();
        stmt.execute(&[&partition, &key]).map(|_| ()).map_err(|e| {
            ClientError::Storage(format!("{} remove {} failed: {}", self.name, key, e))
        })
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
    async fn clear(&self) -> crate::Result<()> {
        let db = self.session.clone();
        let mut conn = db.lock().unwrap();
        let conn = conn.as_mut().unwrap();

        let stmt = format!("DELETE FROM {}", self.name);
        let mut stmt = conn.prepare(&stmt).unwrap();
        stmt.execute([])
            .map(|_| ())
            .map_err(|e| ClientError::Storage(format!("{} clear failed: {}", self.name, e)))
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
        storage.make_table::<SqliteStorage>().unwrap();
    }
    {
        let storage = SqliteStorage::new(test_file);
        storage
            .make_table::<SqliteStorage>()
            .expect("prepare sqlite failed");
    }
    std::fs::remove_file(test_file).unwrap_or(());
}

#[tokio::test]
async fn test_store_i32() {
    let storage = SqliteStorage::new(":memory:");
    storage.make_table::<i32>().unwrap();

    let t = storage.table::<i32>();
    t.set("", "1", Some(&1)).await.ok();
    t.set("", "2", Some(&2)).await.ok();

    let not_exist_3 = t.get("", "3").await;
    assert_eq!(not_exist_3, None);
    let value_2 = t.get("", "2").await;
    assert_eq!(value_2, Some(2));

    t.remove("", "2").await.ok();
    let value_2 = t.get("", "2").await;
    assert_eq!(value_2, None);

    t.clear().await.ok();
    let value_1 = t.get("", "1").await;
    assert_eq!(value_1, None);
}
#[tokio::test]
async fn test_sqlite_query() {
    let storage = SqliteStorage::new(":memory:");
    storage.make_table::<i32>().unwrap();
    let table = storage.table::<i32>();
    for i in 0..500 {
        table.set("", &i.to_string(), Some(&i)).await.ok();
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
            .await
            .expect("query failed");

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
            .await
            .expect("query failed");

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
            .await
            .expect("query failed");

        assert_eq!(v.items.len(), 10);
        assert_eq!(v.start_sort_value, 480);
        assert_eq!(v.end_sort_value, 471);

        assert_eq!(v.items[0], 480);
        assert_eq!(v.items[9], 471);
    }
}
