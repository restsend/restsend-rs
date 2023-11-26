use std::str::FromStr;

mod indexeddb;
mod memory;
mod sqlite;

pub const MEMORY_DSN: &str = ":memory:";

pub trait SortKey: ToString + FromStr {
    fn sort_key(&self) -> i64;
}

type Storage = sqlite::SqliteStorage;

pub trait Table<T: SortKey> {
    fn get(&mut self, key: &str) -> Option<T>;
    fn set(&mut self, key: &str, value: Option<T>);
    fn remove(&mut self, key: &str);
    fn clear(&mut self);
}

pub fn prepare(storage: &Storage) -> anyhow::Result<()> {
    let tables = vec!["topics", "users", "messages", "conversations"];
    for table in tables {
        storage.make_table(table)?;
    }
    Ok(())
}

#[test]
pub fn test_storage_prepare() {
    let storage = Storage::new(MEMORY_DSN);
    prepare(&storage).unwrap();
}
