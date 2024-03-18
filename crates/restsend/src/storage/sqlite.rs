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

    pub async fn table<T>(&self) -> Box<dyn super::Table<T>>
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
        let sort_by_cond = match option.start_sort_value {
            Some(v) => format!("AND sort_by <= {}", v), // exclude start_sort_value
            None => "".to_string(),
        };

        let db = self.session.clone();
        let mut conn = db.lock().unwrap();
        let conn = conn.as_mut().unwrap();

        let stmt = format!(
            "SELECT value FROM {} WHERE partition = ? {} ORDER BY sort_by DESC LIMIT ?",
            self.name, sort_by_cond,
        );

        let mut stmt = conn.prepare(&stmt).unwrap();
        let mut rows = stmt
            .query(&[&partition, format!("{}", option.limit).as_str()])
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
