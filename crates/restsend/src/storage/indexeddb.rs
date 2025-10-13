use super::{QueryOption, QueryResult, StoreModel};
use crate::error::ClientError;
use async_trait::async_trait;
use js_sys::Promise;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{borrow::Borrow, cell::RefCell, io::Cursor, rc::Rc, time::Duration, vec};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    DomException, IdbDatabase, IdbIndexParameters, IdbKeyRange, IdbObjectStore,
    IdbObjectStoreParameters, IdbOpenDbRequest, IdbRequest, IdbTransactionMode,
};

const LAST_DB_VERSION: u32 = 1;
pub struct IndexeddbStorage {
    last_version: Option<u32>,
    db_prefix: String,
    memory_storage: super::memory::InMemoryStorage,
}

#[derive(Serialize, Deserialize)]
struct StoreValue {
    sortkey: f64,
    partition: String,
    key: String,
    value: String,
}

impl IndexeddbStorage {
    #[allow(dead_code)]
    pub async fn new_async(db_name: &str) -> Self {
        Self::new(db_name)
    }

    pub fn new(db_name: &str) -> Self {
        let memory_storage = super::memory::InMemoryStorage::new(db_name);
        IndexeddbStorage {
            last_version: None,
            db_prefix: db_name.to_string(),
            memory_storage,
        }
    }
    pub async fn table<T>(&self) -> Box<dyn super::Table<T>>
    where
        T: StoreModel + 'static,
    {
        if self.db_prefix.is_empty() {
            return self.memory_storage.table::<T>().await;
        }
        let tbl_name = format!("{}-{}", self.db_prefix, super::table_name::<T>());

        IndexeddbTable::open_async(
            tbl_name.to_string(),
            self.last_version.unwrap_or(LAST_DB_VERSION),
            false,
        )
        .await
        .unwrap_or(self.memory_storage.table::<T>().await)
    }
    pub async fn readonly_table<T>(&self) -> Box<dyn super::Table<T>>
    where
        T: StoreModel + 'static,
    {
        if self.db_prefix.is_empty() {
            return self.memory_storage.table::<T>().await;
        }
        let tbl_name = format!("{}-{}", self.db_prefix, super::table_name::<T>());

        IndexeddbTable::open_async(
            tbl_name.to_string(),
            self.last_version.unwrap_or(LAST_DB_VERSION),
            true,
        )
        .await
        .unwrap_or(self.memory_storage.table::<T>().await)
    }
}

#[derive(Debug)]
pub(super) struct IndexeddbTable<T>
where
    T: StoreModel,
{
    table_name: String,
    db: IdbDatabase,
    _phantom: std::marker::PhantomData<T>,
    store: IdbObjectStore,
}

#[allow(dead_code)]
impl<T: StoreModel + 'static> IndexeddbTable<T> {
    pub async fn open_async(
        table_name: String,
        version: u32,
        readonly: bool,
    ) -> crate::Result<Box<dyn super::Table<T>>> {
        let idb = web_sys::window()
            .ok_or(ClientError::Storage("window is none".to_string()))?
            .indexed_db()?
            .ok_or(ClientError::Storage("indexed_db is none".to_string()))?;

        let open_req = idb.open_with_u32(&table_name, version)?;
        let table_name_clone = table_name.to_string();
        let open_req_ref = open_req.clone();

        let (done_tx, mut done_rx) = tokio::sync::mpsc::unbounded_channel();
        let done_tx_clone = done_tx.clone();

        let table_name_ref = table_name_clone.clone();
        let reject_tx = done_tx.clone();
        let on_upgradeneeded_callback = Closure::wrap(Box::new(move |e: web_sys::Event| {
            let db = match e
                .target()
                .and_then(|v| v.dyn_into::<IdbOpenDbRequest>().ok())
                .map(|v| v.result().unwrap_or(JsValue::UNDEFINED))
                .unwrap_or(JsValue::UNDEFINED)
                .dyn_into::<IdbDatabase>()
            {
                Ok(v) => v,
                Err(e) => {
                    reject_tx.send(Err(ClientError::from(e))).ok();
                    return;
                }
            };

            let key_path_id = js_sys::Array::new();
            key_path_id.push(&"partition".into());
            key_path_id.push(&"key".into());
            let mut create_params = IdbObjectStoreParameters::new();
            create_params.set_key_path(&key_path_id);
            let db_store = match db
                .create_object_store_with_optional_parameters(&table_name_ref, &create_params)
            {
                Ok(v) => v,
                Err(e) => {
                    reject_tx.send(Err(ClientError::from(e))).ok();
                    return;
                }
            };

            let key_path_sortkey = js_sys::Array::new();
            key_path_sortkey.push(&"partition".into());
            key_path_sortkey.push(&"sortkey".into());
            let mut params = IdbIndexParameters::new();
            params.set_unique(false);
            match db_store.create_index_with_str_sequence_and_optional_parameters(
                "partition+sortkey",
                &key_path_sortkey,
                &params,
            ) {
                Ok(_) => {}
                Err(e) => {
                    reject_tx.send(Err(ClientError::from(e))).ok();
                    return;
                }
            }
        }) as Box<dyn FnMut(web_sys::Event)>);

        let on_success = Closure::wrap(Box::new(move |e: web_sys::Event| {
            match e
                .target()
                .and_then(|v| v.dyn_into::<IdbOpenDbRequest>().ok())
                .and_then(|open_req| open_req.result().ok())
            {
                Some(v) => {
                    done_tx.send(Ok(v)).ok();
                }
                None => {
                    done_tx
                        .send(Err(ClientError::Storage("open db failed".to_string())))
                        .ok();
                }
            }
        }) as Box<dyn FnMut(web_sys::Event)>);

        let on_error = Closure::once(move |e: DomException| {
            done_tx_clone.send(Err(ClientError::from(e))).ok();
        });

        open_req.set_onupgradeneeded(Some(on_upgradeneeded_callback.as_ref().unchecked_ref()));
        open_req.set_onsuccess(Some(on_success.as_ref().unchecked_ref()));
        open_req.set_onerror(Some(on_error.as_ref().unchecked_ref()));

        let result = done_rx
            .recv()
            .await
            .unwrap_or(Err(ClientError::Storage("No response".to_string())))?;
        let db_result = result
            .dyn_into::<IdbDatabase>()
            .map_err(|e| ClientError::from(e))?;

        open_req_ref.set_onupgradeneeded(None);
        open_req_ref.set_onsuccess(None);
        open_req_ref.set_onerror(None);
        let mode = if readonly {
            IdbTransactionMode::Readonly
        } else {
            IdbTransactionMode::Readwrite
        };

        let store = db_result
            .transaction_with_str_and_mode(&table_name, mode)
            .and_then(|tx| tx.object_store(&table_name))?;

        Ok(Box::new(IndexeddbTable {
            table_name: table_name.to_string(),
            db: db_result,
            _phantom: std::marker::PhantomData,
            store,
        }))
    }
}

impl<T: StoreModel> Drop for IndexeddbTable<T> {
    fn drop(&mut self) {
        self.db.close();
    }
}

unsafe impl<T: StoreModel> Send for IndexeddbTable<T> {}
unsafe impl<T: StoreModel> Sync for IndexeddbTable<T> {}

impl<T: StoreModel + 'static> IndexeddbTable<T> {
    async fn filter(
        &self,
        partition: &str,
        predicate: Box<dyn Fn(T) -> Option<T> + Send>,
        start_sort_value: Option<i64>,
        limit: Option<u32>,
    ) -> Option<Vec<T>> {
        let start_sort_value = match start_sort_value {
            Some(v) => v as f64,
            None => js_sys::Number::POSITIVE_INFINITY,
        };

        let index = self.store.index("partition+sortkey").ok()?;
        let query_range: IdbKeyRange = web_sys::IdbKeyRange::bound(
            &js_sys::Array::of2(&partition.into(), &js_sys::Number::NEGATIVE_INFINITY.into())
                .into(),
            &js_sys::Array::of2(&partition.into(), &start_sort_value.into()).into(),
        )
        .ok()?;

        let cursor_req = index
            .open_cursor_with_range_and_direction(&query_range, web_sys::IdbCursorDirection::Prev)
            .ok()?;

        let items = Rc::new(RefCell::new(Some(vec![])));
        let items_clone = items.clone();
        let predicate = Rc::new(predicate);
        let cursor_req_ref = cursor_req.clone();

        let (done_tx, mut done_rx) = tokio::sync::mpsc::unbounded_channel();
        let done_tx_clone = done_tx.clone();

        let reject_tx = done_tx.clone();
        let predicate_ref = predicate.clone();
        let items_ref = items_clone.clone();
        let on_success_callback = Closure::wrap(Box::new(move |e: web_sys::Event| {
            if let Some(limit) = limit {
                let items_count = items_ref
                    .borrow_mut()
                    .as_mut()
                    .map(|v| v.len())
                    .unwrap_or_default() as u32;
                if items_count >= limit {
                    done_tx.send(JsValue::NULL).ok();
                    return;
                }
            }

            let cursor = match e
                .target()
                .and_then(|v| v.dyn_into::<IdbRequest>().ok())
                .and_then(|cursor_req| cursor_req.result().ok())
                .and_then(|result| result.dyn_into::<web_sys::IdbCursorWithValue>().ok())
            {
                Some(v) => v,
                None => {
                    done_tx.send(JsValue::NULL).ok();
                    return;
                }
            };
            let r = match cursor.value() {
                Ok(v) => match serde_wasm_bindgen::from_value::<StoreValue>(v) {
                    Ok(v) => {
                        if let Ok(Some(item)) =
                            T::from_str(&v.value).map(|item| predicate_ref(item))
                        {
                            items_ref.borrow_mut().as_mut().unwrap().push(item);
                        }
                        cursor.continue_().ok();
                        Ok(())
                    }
                    Err(e) => Err(JsValue::from_str(&e.to_string())),
                },
                Err(e) => Err(e.into()),
            };
            match r {
                Ok(_) => {}
                Err(e) => {
                    reject_tx.send(e).ok();
                }
            }
        }) as Box<dyn FnMut(web_sys::Event)>);

        let on_error_callback = Closure::once(move |e: DomException| {
            done_tx_clone.send(e.into()).ok();
        });

        cursor_req.set_onerror(Some(on_error_callback.as_ref().unchecked_ref()));
        cursor_req.set_onsuccess(Some(on_success_callback.as_ref().unchecked_ref()));

        let r = done_rx.recv().await;
        cursor_req_ref.set_onerror(None);
        cursor_req_ref.set_onsuccess(None);
        _ = r?;
        items.take()
    }

    async fn query(&self, partition: &str, option: &QueryOption) -> Option<QueryResult<T>> {
        let items = Rc::new(RefCell::new(Some(Vec::<T>::new())));
        let start_sort_value = match option.start_sort_value {
            Some(v) => v as f64,
            None => js_sys::Number::POSITIVE_INFINITY,
        };

        let index = self.store.index("partition+sortkey").ok()?;
        let query_range: IdbKeyRange = web_sys::IdbKeyRange::bound(
            &js_sys::Array::of2(&partition.into(), &js_sys::Number::NEGATIVE_INFINITY.into())
                .into(),
            &js_sys::Array::of2(&partition.into(), &start_sort_value.into()).into(),
        )
        .ok()?;

        let cursor_req = index
            .open_cursor_with_range_and_direction(&query_range, web_sys::IdbCursorDirection::Prev)
            .ok()?;

        let limit = option.limit;
        let option_start_sort_value = option.start_sort_value;
        let items_clone = items.clone();
        let cursor_req_ref = cursor_req.clone();

        let items_ref = items_clone.clone();
        let (done_tx, mut done_rx) = tokio::sync::mpsc::unbounded_channel();
        let done_tx_clone = done_tx.clone();
        let on_success = Closure::wrap(Box::new(move |e: web_sys::Event| {
            let cursor = match e
                .target()
                .and_then(|v| v.dyn_into::<IdbRequest>().ok())
                .and_then(|cursor_req| cursor_req.result().ok())
                .and_then(|result| result.dyn_into::<web_sys::IdbCursorWithValue>().ok())
            {
                Some(v) => v,
                None => {
                    done_tx.send(JsValue::NULL).ok();
                    return;
                }
            };
            let r = match cursor.value() {
                Ok(v) => match serde_wasm_bindgen::from_value::<StoreValue>(v) {
                    Ok(v) => {
                        if let Ok(item) = T::from_str(&v.value) {
                            if let Some(items) = items_ref.borrow_mut().as_mut() {
                                items.push(item);
                                if items.len() < (limit + 1) as usize {
                                    cursor.continue_().ok();
                                    return;
                                }
                            }
                        } else {
                            cursor.continue_().ok();
                            return;
                        }
                        Ok(())
                    }
                    Err(e) => Err(JsValue::from_str(&e.to_string())),
                },
                Err(e) => Err(e.into()),
            };
            match r {
                Ok(_) => {
                    done_tx.send(JsValue::NULL).ok();
                }
                Err(e) => {
                    done_tx.send(e).ok();
                }
            }
        }) as Box<dyn FnMut(web_sys::Event)>);

        let on_error = Closure::once(move |e: DomException| {
            done_tx_clone.send(e.into()).ok();
        });

        cursor_req.set_onerror(Some(on_error.as_ref().unchecked_ref()));
        cursor_req.set_onsuccess(Some(on_success.as_ref().unchecked_ref()));

        let r = done_rx.recv().await;

        cursor_req_ref.set_onerror(None);
        cursor_req_ref.set_onsuccess(None);

        match r {
            Some(v) if v.is_instance_of::<DomException>() => {
                return None;
            }
            _ => {}
        }

        // take only limit items
        let mut items = items.take().unwrap_or_default();
        let has_more = items.len() > limit as usize;
        if has_more {
            items.truncate(limit as usize);
        }

        Some(QueryResult {
            start_sort_value: items.first().map(|v| v.sort_key()).unwrap_or(0),
            end_sort_value: items.last().map(|v| v.sort_key()).unwrap_or(0),
            items,
            has_more,
        })
    }

    async fn get(&self, partition: &str, key: &str) -> Option<T> {
        let query_keys = js_sys::Array::new();
        query_keys.push(&partition.into());
        query_keys.push(&key.into());
        let get_req = self.store.get(&query_keys).ok()?;
        let get_req_ref = get_req.clone();

        let (done_tx, mut done_rx) = tokio::sync::mpsc::unbounded_channel();
        let done_tx_clone = done_tx.clone();

        let on_success_callback = Closure::wrap(Box::new(move |e: web_sys::Event| {
            let result = e
                .target()
                .and_then(|v| v.dyn_into::<IdbRequest>().ok())
                .map(|v| v.result().unwrap_or(JsValue::UNDEFINED))
                .unwrap_or(JsValue::UNDEFINED);
            done_tx.send(result).ok();
        }) as Box<dyn FnMut(web_sys::Event)>);

        let on_error_callback = Closure::once(move |e: DomException| {
            done_tx_clone.send(e.into()).ok();
        });

        get_req.set_onsuccess(Some(on_success_callback.as_ref().unchecked_ref()));
        get_req.set_onerror(Some(on_error_callback.as_ref().unchecked_ref()));

        let result = done_rx.recv().await;
        get_req_ref.set_onsuccess(None);
        get_req_ref.set_onerror(None);

        let result = result?;
        serde_wasm_bindgen::from_value::<StoreValue>(result)
            .map_err(|e| ClientError::Storage(e.to_string()))
            .ok()
            .and_then(|v| T::from_str(&v.value).ok())
    }

    async fn batch_update(&self, items: &Vec<super::ValueItem<T>>) -> crate::Result<()> {
        for item in items {
            match item.value.as_ref() {
                None => {
                    let query_keys = js_sys::Array::new();
                    query_keys.push(&item.partition.to_string().into());
                    query_keys.push(&item.key.to_string().into());
                    self.store.delete(&query_keys).ok();
                }
                Some(v) => {
                    let value = StoreValue {
                        sortkey: v.sort_key() as f64,
                        partition: item.partition.to_string(),
                        key: item.key.to_string(),
                        value: v.to_string(),
                    };
                    let item = serde_wasm_bindgen::to_value(&value)
                        .map_err(|e| ClientError::Storage(e.to_string()))?;
                    self.store.put(&item).ok();
                }
            }
        }
        Ok(())
    }

    async fn set(&self, partition: &str, key: &str, value: Option<&T>) -> crate::Result<()> {
        let value = match value {
            None => return self.remove(partition, key).await,
            Some(v) => v,
        };

        let item = StoreValue {
            sortkey: value.sort_key() as f64,
            partition: partition.to_string(),
            key: key.to_string(),
            value: value.to_string(),
        };

        let item =
            serde_wasm_bindgen::to_value(&item).map_err(|e| ClientError::Storage(e.to_string()))?;
        self.store.put(&item)?;
        Ok(())
    }

    async fn remove(&self, partition: &str, key: &str) -> crate::Result<()> {
        let cursor_req = if !key.is_empty() {
            let query_keys = js_sys::Array::new();
            query_keys.push(&partition.into());
            query_keys.push(&key.into());
            self.store.delete(&query_keys.into())
        } else {
            let index = self.store.index("partition+sortkey")?;
            let query_range: IdbKeyRange = web_sys::IdbKeyRange::bound(
                &js_sys::Array::of2(&partition.into(), &js_sys::Number::NEGATIVE_INFINITY.into()),
                &js_sys::Array::of2(&partition.into(), &js_sys::Number::POSITIVE_INFINITY.into()),
            )?;
            index.open_key_cursor_with_range(&query_range)
        }?;
        let cursor_req_ref = cursor_req.clone();

        let (done_tx, mut done_rx) = tokio::sync::mpsc::unbounded_channel();
        let done_tx_clone = done_tx.clone();

        let reject_tx = done_tx.clone();
        let partition_clone = partition.to_string();
        let on_success_callback = Closure::wrap(Box::new(move |e: web_sys::Event| {
            let cursor = match e
                .target()
                .and_then(|v| v.dyn_into::<IdbRequest>().ok())
                .and_then(|cursor_req| cursor_req.result().ok())
                .and_then(|result| result.dyn_into::<web_sys::IdbCursor>().ok())
            {
                Some(v) => v,
                None => {
                    done_tx.send(JsValue::NULL).ok();
                    return;
                }
            };

            let r = match cursor.key() {
                Ok(keys) => match keys.dyn_into::<js_sys::Array>() {
                    Ok(v) => {
                        if v.get(0).as_string().unwrap_or_default() == partition_clone {
                            cursor.delete().ok();
                        }
                        cursor.continue_().ok();
                        Ok(())
                    }
                    Err(e) => Err(e),
                },
                Err(e) => Err(e.into()),
            };
            match r {
                Ok(_) => {}
                Err(e) => {
                    reject_tx.send(e).ok();
                }
            }
        }) as Box<dyn FnMut(web_sys::Event)>);

        let on_error_callback = Closure::once(move |e: DomException| {
            done_tx_clone.send(e.into()).ok();
        });

        cursor_req.set_onerror(Some(on_error_callback.as_ref().unchecked_ref()));
        cursor_req.set_onsuccess(Some(on_success_callback.as_ref().unchecked_ref()));

        let _ = done_rx.recv().await;
        cursor_req_ref.set_onerror(None);
        cursor_req_ref.set_onsuccess(None);
        Ok(())
    }

    async fn last(&self, partition: &str) -> Option<T> {
        let index = self.store.index("partition+sortkey").ok()?;
        let query_range: IdbKeyRange = web_sys::IdbKeyRange::bound(
            &js_sys::Array::of2(&partition.into(), &js_sys::Number::NEGATIVE_INFINITY.into()),
            &js_sys::Array::of2(&partition.into(), &js_sys::Number::POSITIVE_INFINITY.into()),
        )
        .ok()?;

        let cursor_request = index
            .open_cursor_with_range_and_direction(&query_range, web_sys::IdbCursorDirection::Prev)
            .ok()?;
        let cursor_request_ref = cursor_request.clone();

        let (done_tx, mut done_rx) = tokio::sync::mpsc::unbounded_channel();
        let done_tx_clone = done_tx.clone();

        let on_success_callback = Closure::wrap(Box::new(move |e: web_sys::Event| {
            let result = e
                .target()
                .and_then(|v| v.dyn_into::<IdbRequest>().ok())
                .and_then(|cursor_req| cursor_req.result().ok())
                .and_then(|result| result.dyn_into::<web_sys::IdbCursorWithValue>().ok())
                .map(|result| result.value().ok().unwrap_or(JsValue::UNDEFINED))
                .unwrap_or(JsValue::UNDEFINED);

            done_tx.send(result).ok();
        }) as Box<dyn FnMut(web_sys::Event)>);

        let on_error_callback = Closure::once(move |e: DomException| {
            done_tx_clone.send(e.into()).ok();
        });

        cursor_request.set_onsuccess(Some(on_success_callback.as_ref().unchecked_ref()));
        cursor_request.set_onerror(Some(on_error_callback.as_ref().unchecked_ref()));

        let result = done_rx
            .recv()
            .await
            .and_then(|v| {
                serde_wasm_bindgen::from_value::<StoreValue>(v)
                    .map_err(|e| ClientError::Storage(e.to_string()))
                    .ok()
            })
            .and_then(|v| T::from_str(&v.value).ok());
        cursor_request_ref.set_onsuccess(None);
        cursor_request_ref.set_onerror(None);
        result
    }

    async fn clear(&self, partition: &str) -> crate::Result<()> {
        self.remove(partition, "").await
    }
}

#[cfg(target_family = "wasm")]
#[async_trait(?Send)]
impl<T: StoreModel + 'static> super::Table<T> for IndexeddbTable<T> {
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
    async fn batch_update(&self, items: &Vec<super::ValueItem<T>>) -> crate::Result<()> {
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
impl<T: StoreModel + 'static> super::Table<T> for IndexeddbTable<T> {
    async fn filter(
        &self,
        partition: &str,
        predicate: Box<dyn Fn(T) -> Option<T> + Send>,
        end_sort_value: Option<i64>,
        limit: Option<u32>,
    ) -> Option<Vec<T>> {
        None
    }
    async fn query(&self, partition: &str, option: &QueryOption) -> Option<QueryResult<T>> {
        None
    }
    async fn get(&self, partition: &str, key: &str) -> Option<T> {
        None
    }
    async fn set(&self, partition: &str, key: &str, value: Option<&T>) -> crate::Result<()> {
        Ok(())
    }
    async fn batch_update(&self, items: &Vec<super::ValueItem<T>>) -> crate::Result<()> {
        Ok(())
    }
    async fn remove(&self, partition: &str, key: &str) -> crate::Result<()> {
        Ok(())
    }
    async fn last(&self, partition: &str) -> Option<T> {
        None
    }
    async fn clear(&self, partition: &str) -> crate::Result<()> {
        Ok(())
    }
}
