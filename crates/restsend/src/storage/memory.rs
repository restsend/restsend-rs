use super::{QueryOption, QueryResult, StoreModel, ValueItem};
use async_trait::async_trait;
use std::{
    collections::{BTreeMap, HashMap},
    ops::Bound,
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
        let indices = self.index.entry(sort_key).or_insert_with(Vec::new);
        if indices.iter().find(|v| v == &&key).is_none() {
            indices.push(key);
        }
    }

    fn remove(&mut self, key: &str, sort_key: i64) {
        self.data.remove(key);
        let indices = match self.index.get_mut(&sort_key) {
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
type TableInnerRef = Arc<Mutex<HashMap<String, TableInner>>>;

pub struct InMemoryStorage {
    tables: Mutex<HashMap<String, TableInnerRef>>,
}

impl InMemoryStorage {
    pub fn new(_db_name: &str) -> Self {
        InMemoryStorage {
            tables: Mutex::new(HashMap::new()),
        }
    }

    pub async fn new_async(db_name: &str) -> Self {
        Self::new(db_name)
    }

    fn make_table<T>(&self) -> TableInnerRef {
        let tbl_name = super::table_name::<T>();
        let mut tables = self.tables.lock().unwrap();
        if let Some(t) = tables.get(&tbl_name) {
            return t.clone();
        }
        let t = TableInnerRef::default();
        tables.insert(tbl_name, t.clone());
        t
    }
    pub async fn table<T>(&self) -> Box<dyn super::Table<T>>
    where
        T: StoreModel + 'static,
    {
        MemoryTable::from(self.make_table::<T>())
    }
}

#[derive(Debug)]
pub(super) struct MemoryTable<T>
where
    T: StoreModel,
{
    data: TableInnerRef,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: StoreModel + 'static> MemoryTable<T> {
    pub fn from(t: TableInnerRef) -> Box<dyn super::Table<T>> {
        Box::new(MemoryTable {
            data: t,
            _phantom: std::marker::PhantomData,
        })
    }
}

impl<T: StoreModel> MemoryTable<T> {
    async fn filter(
        &self,
        partition: &str,
        predicate: Box<dyn Fn(T) -> Option<T> + Send>,
        start_sort_value: Option<i64>,
        limit: Option<u32>,
    ) -> Option<Vec<T>> {
        let mut data = self.data.lock().unwrap();
        let mut table = data.get_mut(partition)?;
        let mut items = Vec::<T>::new();

        let start_sort_value = match start_sort_value {
            Some(v) => Bound::Included(v),
            None => Bound::Unbounded,
        };

        let mut iter = table
            .index
            .range((Bound::Unbounded, start_sort_value))
            .rev();

        while let Some((_, indices)) = iter.next() {
            for index in indices {
                let v = match table.get(index) {
                    Some(v) => match T::from_str(v) {
                        Ok(v) => v,
                        _ => continue,
                    },
                    None => continue,
                };
                match predicate(v) {
                    Some(v) => items.push(v),
                    None => {}
                }
                if let Some(limit) = limit {
                    if items.len() >= limit as usize {
                        break;
                    }
                }
            }
            if let Some(limit) = limit {
                if items.len() >= limit as usize {
                    break;
                }
            }
        }
        Some(items)
    }

    async fn query(&self, partition: &str, option: &QueryOption) -> Option<QueryResult<T>> {
        let mut data = self.data.lock().unwrap();
        let mut items = Vec::<T>::new();
        let mut table = data.get_mut(partition)?;

        let start_sort_value = match option.start_sort_value {
            Some(v) => Bound::Included(v),
            None => Bound::Unbounded,
        };

        let mut iter = table
            .index
            .range((Bound::Unbounded, start_sort_value))
            .rev();

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
                if items.len() >= (option.limit + 1) as usize {
                    break;
                }
            }
            if items.len() >= (option.limit + 1) as usize {
                break;
            }
        }
        let has_more = items.len() > option.limit as usize;
        if has_more {
            items.pop();
        }
        Some(QueryResult {
            start_sort_value: items.first().map(|v| v.sort_key()).unwrap_or(0),
            end_sort_value: items.last().map(|v| v.sort_key()).unwrap_or(0),
            items,
            has_more,
        })
    }
    async fn get(&self, partition: &str, key: &str) -> Option<T> {
        let mut data = self.data.lock().unwrap();
        let mut table = data.get_mut(partition);
        table?.get(&key).and_then(|v| T::from_str(v).ok())
    }

    async fn set(&self, partition: &str, key: &str, value: Option<&T>) -> crate::Result<()> {
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
                Ok(())
            }
            None => self.remove(partition, key).await,
        }
    }

    async fn batch_update(&self, items: &Vec<ValueItem<T>>) -> crate::Result<()> {
        let mut data = self.data.lock().unwrap();
        for item in items {
            let mut table = data.get_mut(&item.partition);
            if table.is_none() {
                data.insert(item.partition.to_string(), TableInner::default());
                table = data.get_mut(&item.partition);
            }
            match table {
                Some(table) => match item.value.as_ref() {
                    Some(v) => {
                        table.insert(item.key.to_string(), item.sort_key, v.to_string());
                    }
                    None => {
                        table.remove(&item.key, item.sort_key);
                    }
                },
                None => {}
            }
        }
        Ok(())
    }

    async fn remove(&self, partition: &str, key: &str) -> crate::Result<()> {
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
        Ok(())
    }

    async fn last(&self, partition: &str) -> Option<T> {
        let mut data = self.data.lock().unwrap();
        let mut table = data.get_mut(partition);
        table?.last().and_then(|v| T::from_str(v).ok())
    }

    async fn clear(&self) -> crate::Result<()> {
        self.data.lock().unwrap().clear();
        Ok(())
    }
}

#[cfg(target_family = "wasm")]
#[async_trait(?Send)]
impl<T: StoreModel> super::Table<T> for MemoryTable<T> {
    async fn filter(
        &self,
        partition: &str,
        predicate: Box<dyn Fn(T) -> Option<T> + Send>,
        end_sort_value: Option<i64>,
        limit: Option<u32>,
    ) -> Option<Vec<T>> {
        Self::filter(self, partition, predicate, end_sort_value, limit).await
    }
    async fn query(&self, partition: &str, option: &QueryOption) -> Option<QueryResult<T>> {
        Self::query(self, partition, option).await
    }
    async fn get(&self, partition: &str, key: &str) -> Option<T> {
        Self::get(self, partition, key).await
    }
    async fn batch_update(&self, items: &Vec<ValueItem<T>>) -> crate::Result<()> {
        Self::batch_update(self, items).await
    }
    async fn set(&self, partition: &str, key: &str, value: Option<&T>) -> crate::Result<()> {
        Self::set(self, partition, key, value).await
    }
    async fn remove(&self, partition: &str, key: &str) -> crate::Result<()> {
        Self::remove(self, partition, key).await
    }
    async fn last(&self, partition: &str) -> Option<T> {
        Self::last(self, partition).await
    }
    async fn clear(&self) -> crate::Result<()> {
        Self::clear(self).await
    }
}

#[cfg(not(target_family = "wasm"))]
#[async_trait]
impl<T: StoreModel> super::Table<T> for MemoryTable<T> {
    async fn filter(
        &self,
        partition: &str,
        predicate: Box<dyn Fn(T) -> Option<T> + Send>,
        start_sort_value: Option<i64>,
        limit: Option<u32>,
    ) -> Option<Vec<T>> {
        Self::filter(self, partition, predicate, start_sort_value, limit).await
    }
    async fn query(&self, partition: &str, option: &QueryOption) -> Option<QueryResult<T>> {
        Self::query(self, partition, option).await
    }
    async fn get(&self, partition: &str, key: &str) -> Option<T> {
        Self::get(self, partition, key).await
    }
    async fn batch_update(&self, items: &Vec<ValueItem<T>>) -> crate::Result<()> {
        Self::batch_update(self, items).await
    }
    async fn set(&self, partition: &str, key: &str, value: Option<&T>) -> crate::Result<()> {
        Self::set(self, partition, key, value).await
    }
    async fn remove(&self, partition: &str, key: &str) -> crate::Result<()> {
        Self::remove(self, partition, key).await
    }
    async fn last(&self, partition: &str) -> Option<T> {
        Self::last(self, partition).await
    }
    async fn clear(&self) -> crate::Result<()> {
        Self::clear(self).await
    }
}

#[tokio::test]
async fn test_memory_table() {
    let t = TableInnerRef::default();
    let table = MemoryTable::from(t);
    table.set("", "1", Some(&1)).await;
    table.set("", "2", Some(&2)).await;
    table.set("", "3", Some(&3)).await;
    let v = table.get("", "1").await.expect("must value");
    assert_eq!(v, 1);
    table.remove("", "1").await;
    let v = table.get("", "1").await;
    assert_eq!(v, None);
    table.clear().await;
    let v = table.get("", "2").await;
    assert_eq!(v, None);
}
