use crate::Result;
use async_trait::async_trait;
use std::str::FromStr;

#[allow(unused)]
mod indexeddb;

#[allow(unused)]
mod memory;

#[cfg(not(target_family = "wasm"))]
mod sqlite;

pub trait StoreModel: ToString + FromStr + Sync + Send {
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

#[cfg(not(feature = "indexeddb"))]
#[cfg(target_family = "wasm")]
pub type Storage = memory::InMemoryStorage;

#[cfg(not(target_family = "wasm"))]
pub type Storage = sqlite::SqliteStorage;

#[cfg(feature = "indexeddb")]
pub type Storage = indexeddb::IndexeddbStorage;

#[cfg(target_family = "wasm")]
#[async_trait(?Send)]
pub trait Table<T: StoreModel>: Send + Sync {
    async fn filter(
        &self,
        partition: &str,
        predicate: Box<dyn Fn(T) -> Option<T> + Send>,
    ) -> Option<Vec<T>>;
    async fn query(&self, partition: &str, option: &QueryOption) -> Option<QueryResult<T>>;
    async fn get(&self, partition: &str, key: &str) -> Option<T>;
    async fn set(&self, partition: &str, key: &str, value: Option<&T>) -> Result<()>;
    async fn remove(&self, partition: &str, key: &str) -> Result<()>;
    async fn last(&self, partition: &str) -> Option<T>;
    async fn clear(&self) -> Result<()>;
}

#[cfg(not(target_family = "wasm"))]
#[async_trait]
pub trait Table<T: StoreModel>: Send + Sync {
    async fn filter(
        &self,
        partition: &str,
        predicate: Box<dyn Fn(T) -> Option<T> + Send>,
    ) -> Option<Vec<T>>;
    async fn query(&self, partition: &str, option: &QueryOption) -> Option<QueryResult<T>>;
    async fn get(&self, partition: &str, key: &str) -> Option<T>;
    async fn set(&self, partition: &str, key: &str, value: Option<&T>) -> Result<()>;
    async fn remove(&self, partition: &str, key: &str) -> Result<()>;
    async fn last(&self, partition: &str) -> Option<T>;
    async fn clear(&self) -> Result<()>;
}

pub(super) fn table_name<T>() -> String {
    let full_name = std::any::type_name::<T>();
    let parts: Vec<&str> = full_name.split("::").collect();
    match parts.last() {
        Some(v) => v,
        None => full_name,
    }
    .into()
}

#[tokio::test]
async fn test_store_i32() {
    let storage = Storage::new(":memory:");

    let t = storage.table::<i32>().await;
    t.set("", "1", Some(&1)).await.ok();
    t.set("", "2", Some(&2)).await.ok();

    let not_exist_3 = t.get("", "3").await;
    assert_eq!(not_exist_3, None);
    let value_2 = t.get("", "2").await;
    assert_eq!(value_2, Some(2));

    t.remove("", "2").await.ok();
    let value_2 = t.get("", "2").await;
    assert_eq!(value_2, None);

    t.clear().await.ok();
    let value_1 = t.get("", "1").await;
    assert_eq!(value_1, None);
}
#[tokio::test]
async fn test_store_query() {
    let storage = Storage::new(":memory:");
    let table = storage.table::<i32>().await;
    for i in 0..500 {
        table.set("", &i.to_string(), Some(&i)).await.ok();
    }
    {
        let v = table
            .query(
                "",
                &QueryOption {
                    start_sort_value: None,
                    limit: 10,
                    keyword: None,
                },
            )
            .await
            .expect("query failed");

        assert_eq!(v.items.len(), 10);
        assert_eq!(v.start_sort_value, 499);
        assert_eq!(v.end_sort_value, 490);

        assert_eq!(v.items[0], 499);
        assert_eq!(v.items[9], 490);
    }
    {
        let v = table
            .query(
                "",
                &QueryOption {
                    start_sort_value: Some(490),
                    limit: 10,
                    keyword: None,
                },
            )
            .await
            .expect("query failed");

        assert_eq!(v.items.len(), 10);
        assert_eq!(v.start_sort_value, 490);
        assert_eq!(v.end_sort_value, 481);

        assert_eq!(v.items[0], 490);
        assert_eq!(v.items[9], 481);
    }
    {
        let v = table
            .query(
                "",
                &QueryOption {
                    start_sort_value: Some(480),
                    limit: 10,
                    keyword: None,
                },
            )
            .await
            .expect("query failed");

        assert_eq!(v.items.len(), 10);
        assert_eq!(v.start_sort_value, 480);
        assert_eq!(v.end_sort_value, 471);

        assert_eq!(v.items[0], 480);
        assert_eq!(v.items[9], 471);
    }
}
