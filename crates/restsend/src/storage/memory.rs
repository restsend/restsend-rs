use super::StoreModel;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::str::FromStr;

pub struct InMemoryStorage {}

impl InMemoryStorage {
    pub fn new() -> Self {
        InMemoryStorage {}
    }
    pub fn make_table(&self, _name: &str) -> anyhow::Result<()> {
        Ok(())
    }
    pub fn table<T>(&self, _name: &str) -> Option<Box<dyn super::Table<T>>>
    where
        T: StoreModel + 'static,
    {
        let table = MemoryTable::new(100);
        Some(table)
    }
}

#[derive(Debug)]
pub(super) struct MemoryTable<T>
where
    T: StoreModel,
{
    data: LruCache<String, String>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: StoreModel + 'static> MemoryTable<T> {
    pub fn new(max_items: usize) -> Box<dyn super::Table<T>> {
        Box::new(MemoryTable {
            data: LruCache::new(NonZeroUsize::new(max_items).unwrap()),
            _phantom: std::marker::PhantomData,
        })
    }
}

impl<T: StoreModel> super::Table<T> for MemoryTable<T> {
    fn get(&mut self, partition: &str, key: &str) -> Option<T> {
        let key = format!("{}:{}", partition, key);
        let v = self.data.get(&key);
        v.and_then(|v| match T::from_str(v) {
            Ok(v) => Some(v),
            _ => None,
        })
    }

    fn set(&mut self, partition: &str, key: &str, value: Option<T>) {
        match value {
            Some(v) => {
                let key = format!("{}:{}", partition, key);
                self.data.push(key, v.to_string());
            }
            None => {
                self.remove(partition, key);
            }
        }
    }

    fn remove(&mut self, partition: &str, key: &str) {
        let key = format!("{}:{}", partition, key);
        self.data.pop(&key);
    }
    fn clear(&mut self) {
        self.data.clear();
    }
}

#[test]
fn test_memory_table() {
    let mut table = MemoryTable::new(100);
    table.set("", "1", Some(1));
    table.set("", "2", Some(2));
    table.set("", "3", Some(3));
    let v = table.get("", "1");
    assert_eq!(v, Some(1));
    table.remove("", "1");
    let v = table.get("", "1");
    assert_eq!(v, None);
    table.clear();
    let v = table.get("", "2");
    assert_eq!(v, None);
}

#[test]
fn test_memory_storage() {
    let storage = InMemoryStorage::new();
    storage.make_table("test").unwrap();
    let mut table = storage.table::<i32>("test").unwrap();
    table.set("", "1", Some(1));
    table.set("", "2", Some(2));
    table.set("", "3", Some(3));
    let v = table.get("", "1");
    assert_eq!(v, Some(1));
    table.remove("", "1");
    let v = table.get("", "1");
    assert_eq!(v, None);
    table.clear();
    let v = table.get("", "2");
    assert_eq!(v, None);
}
