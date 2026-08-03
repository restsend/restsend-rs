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
        // A key may move to a new sort_key bucket on update; drop it from any
        // other bucket first so it is never returned more than once by filter.
        for (_, indices) in self.index.iter_mut() {
            indices.retain(|v| v != &key);
        }
        self.index.retain(|_, indices| !indices.is_empty());
        self.data.insert(key.clone(), value.clone());
        let indices = self.index.entry(sort_key).or_default();
        indices.push(key);
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
    pub async fn table<T>(&self) -> crate::Result<Box<dyn super::Table<T>>>
    where
        T: StoreModel + 'static,
    {
        Ok(MemoryTable::from(self.make_table::<T>()))
    }
    pub async fn readonly_table<T>(&self) -> crate::Result<Box<dyn super::Table<T>>>
    where
        T: StoreModel + 'static,
    {
        self.table::<T>().await
    }
}

impl super::ConversationRouting for InMemoryStorage {}

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

        let start_sort_value: Bound<i64> = match start_sort_value {
            Some(v) => Bound::Included(v),
            None => Bound::Unbounded,
        };

        let mut iter = table
            .index
            .range((Bound::Unbounded, start_sort_value))
            .rev();

        for (_, indices) in iter {
            for index in indices {
                let v = match table.get(index) {
                    Some(v) => match T::from_str(v) {
                        Ok(v) => v,
                        Err(_) => {
                            log::warn!("memory filter deserialize error, value:{}", v);
                            continue;
                        }
                    },
                    None => continue,
                };
                if let Some(v) = predicate(v) {
                    items.push(v)
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

        for (_, indices) in iter {
            for index in indices {
                let v = match table.get(index) {
                    Some(v) => match T::from_str(v) {
                        Ok(v) => v,
                        Err(_) => {
                            log::warn!("memory query deserialize error, value:{}", v);
                            continue;
                        }
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
        }
        let has_more = items.len() > option.limit as usize;
        if has_more {
            items.truncate(option.limit as usize);
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
        let value = table?.get(key)?;
        match T::from_str(value) {
            Ok(v) => Some(v),
            Err(_) => {
                log::warn!("memory get deserialize error, key:{} value:{}", key, value);
                None
            }
        }
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
                if let Some(table) = table {
                    let sort_key = v.sort_key();
                    table.insert(key.to_string(), sort_key, v.to_string());
                }
                Ok(())
            }
            None => self.remove(partition, key).await,
        }
    }

    async fn batch_update(&self, items: &[ValueItem<T>]) -> crate::Result<()> {
        let mut data = self.data.lock().unwrap();
        for item in items {
            let mut table = data.get_mut(&item.partition);
            if table.is_none() {
                data.insert(item.partition.to_string(), TableInner::default());
                table = data.get_mut(&item.partition);
            }
            if let Some(table) = table {
                match item.value.as_ref() {
                    Some(v) => {
                        table.insert(item.key.to_string(), item.sort_key, v.to_string());
                    }
                    None => {
                        table.remove(&item.key, item.sort_key);
                    }
                }
            }
        }
        Ok(())
    }

    async fn remove(&self, partition: &str, key: &str) -> crate::Result<()> {
        let mut data = self.data.lock().unwrap();
        let mut table = data.get_mut(partition);
        if let Some(table) = table {
            if let Some(value) = table.get(key) {
                match T::from_str(value) {
                    Ok(v) => {
                        table.remove(key, v.sort_key());
                    }
                    Err(e) => {
                        log::warn!("memory remove deserialize error, key:{} value:{}", key, value);
                    }
                }
            }
        };
        Ok(())
    }

    async fn last(&self, partition: &str) -> Option<T> {
        let mut data = self.data.lock().unwrap();
        let mut table = data.get_mut(partition);
        let value = table?.last()?;
        match T::from_str(value) {
            Ok(v) => Some(v),
            Err(_) => {
                log::warn!("memory last deserialize error, value:{}", value);
                None
            }
        }
    }

    async fn clear(&self, partition: &str) -> crate::Result<()> {
        let mut data = self.data.lock().unwrap();
        let mut table = data.get_mut(partition);
        if let Some(table) = table {
            table.clear();
        }
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
    async fn batch_update(&self, items: &[ValueItem<T>]) -> crate::Result<()> {
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
    async fn clear(&self, partition: &str) -> crate::Result<()> {
        Self::clear(self, partition).await
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
    async fn batch_update(&self, items: &[ValueItem<T>]) -> crate::Result<()> {
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
    async fn clear(&self, partition: &str) -> crate::Result<()> {
        Self::clear(self, partition).await
    }
}

#[tokio::test]
async fn test_memory_table() {
    let t = TableInnerRef::default();
    let table = MemoryTable::from(t);
    table.set("test", "1", Some(&1)).await;
    table.set("test", "2", Some(&2)).await;
    table.set("test", "3", Some(&3)).await;
    let v = table.get("test", "1").await.expect("must value");
    assert_eq!(v, 1);
    table.remove("test", "1").await;
    let v = table.get("test", "1").await;
    assert_eq!(v, None);
    table.clear("test").await;
    let v = table.get("test", "2").await;
    assert_eq!(v, None);
}

#[tokio::test]
async fn test_memory_table_no_duplicate_on_sort_key_change() {
    #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
    struct Sorted(i64, i32);

    impl std::str::FromStr for Sorted {
        type Err = serde_json::Error;
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            serde_json::from_str(s)
        }
    }
    impl std::fmt::Display for Sorted {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(&serde_json::to_string(self).unwrap())
        }
    }
    impl super::StoreModel for Sorted {
        fn sort_key(&self) -> i64 {
            self.0
        }
    }

    let t = TableInnerRef::default();
    let table = MemoryTable::from(t);

    // Same key updated with a newer sort_key must not be returned twice.
    table.set("t", "a", Some(&Sorted(1, 1))).await.unwrap();
    table.set("t", "a", Some(&Sorted(2, 2))).await.unwrap();
    table.set("t", "b", Some(&Sorted(1, 3))).await.unwrap();
    table.set("t", "b", Some(&Sorted(3, 4))).await.unwrap();

    let items = table.filter("t", Box::new(|c| Some(c)), None, None).await.unwrap();
    let keys: Vec<String> = items.iter().map(|v| format!("{}:{}", v.0, v.1)).collect();
    assert_eq!(keys.len(), 2, "same keys updated with new sort_key must not duplicate");
    assert!(keys.contains(&"3:4".to_string()));
    assert!(keys.contains(&"2:2".to_string()));
}
