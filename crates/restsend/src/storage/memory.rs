use super::{QueryOption, QueryResult, StoreModel};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

type Table = Arc<Mutex<HashMap<String, String>>>;

pub struct InMemoryStorage {
    tables: Mutex<HashMap<String, Table>>,
}

impl InMemoryStorage {
    pub fn new(_db_name: &str) -> Self {
        InMemoryStorage {
            tables: Mutex::new(HashMap::new()),
        }
    }

    pub fn make_table(&self, name: &str) -> crate::Result<()> {
        let mut tables = self.tables.lock().unwrap();
        match tables.get(name) {
            Some(_) => return Ok(()),
            None => {}
        };
        tables.insert(name.to_string(), Arc::new(Mutex::new(HashMap::new())));
        Ok(())
    }
    pub fn table<T>(&self, _name: &str) -> Box<dyn super::Table<T>>
    where
        T: StoreModel + 'static,
    {
        let tables = self.tables.lock().unwrap();
        let t = tables.get(_name).unwrap().clone();
        let table = MemoryTable::from(t);
        table
    }
}

#[derive(Debug)]
pub(super) struct MemoryTable<T>
where
    T: StoreModel,
{
    data: Table,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: StoreModel + 'static> MemoryTable<T> {
    pub fn from(t: Table) -> Box<dyn super::Table<T>> {
        Box::new(MemoryTable {
            data: t,
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

            if v.sort_key() <= option.start_sort_value {
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

        if total > option.limit {
            let discard = total - option.limit;
            items.truncate(discard as usize);
        }

        QueryResult {
            total,
            start_sort_value: items.first().map(|v| v.sort_key()).unwrap_or(0),
            end_sort_value: items.last().map(|v| v.sort_key()).unwrap_or(0),
            items,
        }
    }
    fn get(&self, partition: &str, key: &str) -> Option<T> {
        let key = format!("{}:{}", partition, key);
        let data = self.data.lock().unwrap();
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
                self.data.lock().unwrap().insert(key, v.to_string());
            }
            None => {
                self.remove(partition, key);
            }
        }
    }

    fn remove(&self, partition: &str, key: &str) {
        let key = format!("{}:{}", partition, key);
        self.data.lock().unwrap().remove(&key);
    }
    fn clear(&self) {
        self.data.lock().unwrap().clear();
    }
}

#[test]
fn test_memory_table() {
    let t = Table::default();
    let table = MemoryTable::from(t);
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
