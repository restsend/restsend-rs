use super::{QueryOption, QueryResult, StoreModel};
use std::{
    collections::{BTreeMap, HashMap},
    ops::{Bound, Range},
    sync::{Arc, Mutex},
};
#[derive(Debug, Default)]
pub struct TableInner {
    pub(super) data: HashMap<String, String>,
    pub(super) index: BTreeMap<i64, String>,
}

impl TableInner {
    fn get(&self, key: &str) -> Option<&String> {
        self.data.get(key)
    }

    fn insert(&mut self, key: String, sort_key: i64, value: String) {
        self.data.insert(key.clone(), value.clone());
        self.index.insert(sort_key, key);
    }

    fn remove(&mut self, key: &str, sort_key: i64) {
        self.index.remove(&sort_key);
        self.data.remove(key);
    }

    fn clear(&mut self) {
        self.data.clear();
        self.index.clear();
    }
}
type Table = Arc<Mutex<HashMap<String, TableInner>>>;

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
        tables.insert(name.to_string(), Table::default());
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
    fn filter(&self, partition: &str, predicate: Box<dyn Fn(T) -> Option<T>>) -> Vec<T> {
        let mut data = self.data.lock().unwrap();
        let mut table = data.get_mut(partition);
        let mut table = match table {
            Some(table) => table,
            None => return vec![],
        };
        let mut items = Vec::<T>::new();
        for (_, v) in table.data.iter() {
            let v = match T::from_str(v) {
                Ok(v) => v,
                _ => continue,
            };
            match predicate(v) {
                Some(v) => items.push(v),
                None => {}
            }
        }
        items
    }

    fn query(&self, partition: &str, option: &QueryOption) -> QueryResult<T> {
        let mut data = self.data.lock().unwrap();
        let mut items = Vec::<T>::new();
        let mut table = data.get_mut(partition);
        let mut table = match table {
            Some(table) => table,
            None => {
                return QueryResult {
                    start_sort_value: 0,
                    end_sort_value: 0,
                    items,
                }
            }
        };
        let mut iter = table
            .index
            .range((Bound::Excluded(&option.start_sort_value), Bound::Unbounded));

        while let Some((_, key)) = iter.next() {
            let v = table.get(key);
            match v {
                Some(v) => {
                    let v = match T::from_str(v) {
                        Ok(v) => v,
                        _ => continue,
                    };
                    if let Some(keyword) = &option.keyword {
                        if !v.to_string().contains(keyword) {
                            continue;
                        }
                    }
                    items.push(v);
                    if items.len() >= option.limit as usize {
                        break;
                    }
                }
                None => {}
            }
        }

        items.reverse();

        QueryResult {
            start_sort_value: items.first().map(|v| v.sort_key()).unwrap_or(0),
            end_sort_value: items.last().map(|v| v.sort_key()).unwrap_or(0),
            items,
        }
    }
    fn get(&self, partition: &str, key: &str) -> Option<T> {
        let mut data = self.data.lock().unwrap();
        let mut table = data.get_mut(partition);
        match table {
            Some(table) => {
                let v = table.get(&key);
                match v {
                    Some(v) => {
                        let v = match T::from_str(v) {
                            Ok(v) => v,
                            _ => return None,
                        };
                        return Some(v);
                    }
                    None => {}
                }
            }
            None => {}
        }
        None
    }

    fn set(&self, partition: &str, key: &str, value: Option<T>) {
        match value {
            Some(v) => {
                let mut data = self.data.lock().unwrap();
                let mut table = data.get_mut(partition);
                if table.is_none() {
                    data.insert(partition.to_string(), TableInner::default());
                    table = data.get_mut(partition);
                }
                match table {
                    Some(table) => {
                        let sort_key = v.sort_key();
                        table.insert(key.to_string(), sort_key, v.to_string());
                    }
                    None => {}
                }
            }
            None => {
                self.remove(partition, key);
            }
        }
    }

    fn remove(&self, partition: &str, key: &str) {
        let mut data = self.data.lock().unwrap();
        let mut table = data.get_mut(partition);
        match table {
            Some(table) => {
                let v = table.get(&key);
                match v {
                    Some(v) => {
                        let v = match T::from_str(v) {
                            Ok(v) => v,
                            _ => return,
                        };
                        table.remove(&key, v.sort_key());
                    }
                    None => {}
                }
            }
            None => {}
        }
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
    let v = table.get("", "1").expect("must value");
    assert_eq!(v, 1);
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
