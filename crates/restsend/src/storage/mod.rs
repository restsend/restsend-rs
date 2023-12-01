use std::str::FromStr;
mod indexeddb;
mod memory;
mod sqlite;

pub trait StoreModel: ToString + FromStr {
    fn sort_key(&self) -> i64;
}

pub struct SearchOption {
    pub keyword: String,
    pub pos: u32,
    pub limit: u32,
}

type Storage = sqlite::SqliteStorage;

pub trait Table<T: StoreModel> {
    fn get(&self, partition: &str, key: &str) -> Option<T>;
    fn set(&self, partition: &str, key: &str, value: Option<T>);
    fn remove(&self, partition: &str, key: &str);
    fn clear(&self);
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
    let storage = Storage::new(":memory:");
    prepare(&storage).unwrap();
}
