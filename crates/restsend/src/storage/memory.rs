use super::{QueryOption, QueryResult, StoreModel};
use lru::LruCache;
use std::{num::NonZeroUsize, sync::Mutex};

pub struct InMemoryStorage {}

impl InMemoryStorage {
    pub fn new(_db_name: &str) -> Self {
        InMemoryStorage {}
    }

    pub async fn new_async(db_name: &str) -> Self {
        Self::new(db_name)
    }

    pub fn make_table(&self, name: &str) -> crate::Result<()> {
        let mut tables = self.tables.lock().unwrap();
        match tables.get(name) {
            Some(_) => return Ok(()),
            None => {}
        };
        tables.insert(name.to_string(), Table::default());
        Ok(())
    }
    pub fn table<T>(&self, _name: &str) -> Box<dyn super::Table<T>>
    where
        T: StoreModel + 'static,
    {
        let table = MemoryTable::new(100);
        table
    }
}

#[derive(Debug)]
pub(super) struct MemoryTable<T>
where
    T: StoreModel,
{
    data: Mutex<LruCache<String, String>>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: StoreModel + 'static> MemoryTable<T> {
    pub fn new(max_items: usize) -> Box<dyn super::Table<T>> {
        Box::new(MemoryTable {
            data: Mutex::new(LruCache::new(NonZeroUsize::new(max_items).unwrap())),
            _phantom: std::marker::PhantomData,
        })
    }
}

impl<T: StoreModel> super::Table<T> for MemoryTable<T> {
    fn query(&self, partition: &str, option: &QueryOption) -> QueryResult<T> {
        let data = self.data.lock().unwrap();
        let mut items = Vec::<T>::new();
        for (k, v) in data.iter() {
            if !k.starts_with(partition) {
                continue;
            }
            let v = match T::from_str(v) {
                Ok(v) => v,
                _ => continue,
            };

            if v.sort_key() < option.start_sort_value {
                continue;
            }

            if let Some(keyword) = &option.keyword {
                if !v.to_string().contains(keyword) {
                    continue;
                }
            }
            items.push(v);
        }
        items.sort_by_key(|v| v.sort_key());
        items.reverse();
        let total = items.len() as u32;
        items.truncate(option.limit as usize);

        QueryResult {
            total,
            start_sort_value: items.first().map(|v| v.sort_key()).unwrap_or(0),
            end_sort_value: items.last().map(|v| v.sort_key()).unwrap_or(0),
            items,
        }
    }
    fn get(&self, partition: &str, key: &str) -> Option<T> {
        let key = format!("{}:{}", partition, key);
        let mut data = self.data.lock().unwrap();
        let v = data.get(&key);
        v.and_then(|v| match T::from_str(v) {
            Ok(v) => Some(v),
            _ => None,
        })
    }

    fn set(&self, partition: &str, key: &str, value: Option<T>) {
        match value {
            Some(v) => {
                let key = format!("{}:{}", partition, key);
                self.data.lock().unwrap().push(key, v.to_string());
            }
            None => {
                self.remove(partition, key);
            }
        }
    }

    fn remove(&self, partition: &str, key: &str) {
        let key = format!("{}:{}", partition, key);
        self.data.lock().unwrap().pop(&key);
    }
    fn clear(&self) {
        self.data.lock().unwrap().clear();
    }
}

#[test]
fn test_memory_table() {
    let table = MemoryTable::new(100);
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
    let storage = InMemoryStorage::new("");
    storage.make_table("test").unwrap();
    let table = storage.table::<i32>("test");
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
