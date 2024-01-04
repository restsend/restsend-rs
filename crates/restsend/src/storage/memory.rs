use super::{QueryOption, QueryResult, StoreModel};
use std::{
    collections::{BTreeMap, HashMap},
    ops::{Bound, Range},
    sync::{Arc, Mutex},
};
#[derive(Debug, Default)]
pub struct TableInner {
    pub(super) data: HashMap<String, String>,
    pub(super) index: BTreeMap<i64, Vec<String>>,
}

impl TableInner {
    fn get(&self, key: &str) -> Option<&String> {
        self.data.get(key)
    }

    fn insert(&mut self, key: String, sort_key: i64, value: String) {
        self.data.insert(key.clone(), value.clone());
        let mut indices = self.index.entry(sort_key).or_insert_with(Vec::new);
        if indices.iter().find(|v| v == &&key).is_none() {
            indices.push(key);
        }
    }

    fn remove(&mut self, key: &str, sort_key: i64) {
        self.data.remove(key);
        let mut indices = match self.index.get_mut(&sort_key) {
            Some(v) => v,
            None => return,
        };
        indices.retain(|v| v != key);
    }

    fn last(&self) -> Option<&String> {
        self.index
            .iter()
            .last()
            .and_then(|(_, v)| v.last())
            .and_then(|v| self.data.get(v))
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

        let start_sort_value = (match option.start_sort_value {
            Some(v) => v,
            None => match table.last() {
                Some(v) => match T::from_str(v) {
                    Ok(v) => v.sort_key(),
                    _ => 0,
                },
                None => 0,
            },
        } - option.limit as i64)
            .max(0);

        let mut iter = table
            .index
            .range((Bound::Excluded(start_sort_value), Bound::Unbounded));

        while let Some((_, indices)) = iter.next() {
            for index in indices {
                let v = match table.get(index) {
                    Some(v) => match T::from_str(v) {
                        Ok(v) => v,
                        _ => continue,
                    },
                    None => continue,
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
            if items.len() >= option.limit as usize {
                break;
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
        table?.get(&key).and_then(|v| T::from_str(v).ok())
    }

    fn set(&self, partition: &str, key: &str, value: Option<&T>) {
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
                table
                    .get(&key)
                    .and_then(|v| T::from_str(v).ok())
                    .and_then(|v| {
                        table.remove(&key, v.sort_key());
                        Some(())
                    });
            }
            None => {}
        };
    }

    fn last(&self, partition: &str) -> Option<T> {
        let mut data = self.data.lock().unwrap();
        let mut table = data.get_mut(partition);
        table?.last().and_then(|v| T::from_str(v).ok())
    }

    fn clear(&self) {
        self.data.lock().unwrap().clear();
    }
}

#[test]
fn test_memory_table() {
    let t = Table::default();
    let table = MemoryTable::from(t);
    table.set("", "1", Some(&1));
    table.set("", "2", Some(&2));
    table.set("", "3", Some(&3));
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
    table.set("", "1", Some(&1));
    table.set("", "2", Some(&2));
    table.set("", "3", Some(&3));
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
fn test_memory_query() {
    let storage = InMemoryStorage::new("");
    storage.make_table("test").unwrap();
    let table = storage.table::<i32>("test");
    for i in 0..500 {
        table.set("", &i.to_string(), Some(&i));
    }
    {
        let v = table.query(
            "",
            &QueryOption {
                start_sort_value: None,
                limit: 10,
                keyword: None,
            },
        );

        assert_eq!(v.items.len(), 10);
        assert_eq!(v.start_sort_value, 499);
        assert_eq!(v.end_sort_value, 490);

        assert_eq!(v.items[0], 499);
        assert_eq!(v.items[9], 490);
    }
    {
        let v = table.query(
            "",
            &QueryOption {
                start_sort_value: Some(490),
                limit: 10,
                keyword: None,
            },
        );

        assert_eq!(v.items.len(), 10);
        assert_eq!(v.start_sort_value, 490);
        assert_eq!(v.end_sort_value, 481);

        assert_eq!(v.items[0], 490);
        assert_eq!(v.items[9], 481);
    }
    {
        let v = table.query(
            "",
            &QueryOption {
                start_sort_value: Some(480),
                limit: 10,
                keyword: None,
            },
        );

        assert_eq!(v.items.len(), 10);
        assert_eq!(v.start_sort_value, 480);
        assert_eq!(v.end_sort_value, 471);

        assert_eq!(v.items[0], 480);
        assert_eq!(v.items[9], 471);
    }
}
