use crate::Result;
use std::str::FromStr;

#[allow(unused)]
mod memory;

#[cfg(not(target_family = "wasm"))]
mod sqlite;

pub trait StoreModel: ToString + FromStr {
    fn sort_key(&self) -> i64;
}
#[derive(Debug)]
pub struct QueryOption {
    pub keyword: Option<String>,
    pub start_sort_value: Option<i64>,
    pub limit: u32,
}
#[derive(Debug)]
pub struct QueryResult<T: StoreModel> {
    pub start_sort_value: i64,
    pub end_sort_value: i64,
    pub items: Vec<T>,
}

#[cfg(target_family = "wasm")]
pub type Storage = memory::InMemoryStorage;

#[cfg(not(target_family = "wasm"))]
pub type Storage = sqlite::SqliteStorage;

pub trait Table<T: StoreModel> {
    fn filter(&self, partition: &str, predicate: Box<dyn Fn(T) -> Option<T>>) -> Vec<T>;
    fn query(&self, partition: &str, option: &QueryOption) -> QueryResult<T>;
    fn get(&self, partition: &str, key: &str) -> Option<T>;
    fn set(&self, partition: &str, key: &str, value: Option<&T>);
    fn remove(&self, partition: &str, key: &str);
    fn last(&self, partition: &str) -> Option<T>;
    fn clear(&self);
}

pub fn prepare(storage: &Storage) -> Result<()> {
    let tables = vec!["topics", "users", "messages", "conversations", "chat_logs"];
    for table in tables {
        storage.make_table(table)?;
    }
    Ok(())
}

#[test]
pub fn test_storage_prepare() {
    let storage = Storage::new(":memory:");
    prepare(&storage).unwrap();
}
