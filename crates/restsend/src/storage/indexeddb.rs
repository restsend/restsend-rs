use super::QueryOption;
use super::QueryResult;
use super::StoreModel;
use crate::error::ClientError;
use async_trait::async_trait;
use js_sys::Promise;
use serde::Deserialize;
use serde::Serialize;
use std::sync::{Arc, Mutex};
use std::vec;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::wasm_bindgen::closure::Closure;
use web_sys::wasm_bindgen::JsValue;
use web_sys::DomException;
use web_sys::IdbDatabase;
use web_sys::IdbIndexParameters;
use web_sys::IdbKeyRange;
use web_sys::IdbObjectStoreParameters;
use web_sys::IdbOpenDbRequest;
use web_sys::IdbRequest;
use web_sys::IdbTransactionMode;

pub struct IndexeddbStorage {
    db: Arc<Mutex<IdbDatabase>>,
}

#[derive(Serialize, Deserialize)]
struct ValueItem {
    sortkey: i64,
    partition: String,
    key: String,
    value: String,
}

impl IndexeddbStorage {
    pub async fn open_db(db_name: &str, version: Option<u32>) -> crate::Result<IdbDatabase> {
        let idb = web_sys::window()
            .ok_or(ClientError::Storage("window is none".to_string()))?
            .indexed_db()?
            .ok_or(ClientError::Storage("indexed_db is none".to_string()))?;

        let open_req = idb.open_with_u32(db_name, version.unwrap_or(1))?;
        let p = Promise::new(&mut |resolve, reject| {
            let on_success_callback = Closure::wrap(Box::new(move |e: web_sys::Event| {
                let result = e
                    .target()
                    .and_then(|v| v.dyn_into::<IdbOpenDbRequest>().ok())
                    .map(|v| v.result().unwrap_or(JsValue::UNDEFINED))
                    .unwrap_or(JsValue::UNDEFINED);
                resolve.call1(&JsValue::null(), &result).ok();
            })
                as Box<dyn FnMut(web_sys::Event)>);
            let on_error_callback = Closure::wrap(Box::new(move |e: DomException| {
                let _ = reject.call1(&JsValue::null(), &e).ok();
            }) as Box<dyn FnMut(DomException)>);

            open_req.set_onsuccess(Some(on_success_callback.as_ref().unchecked_ref()));
            on_success_callback.forget();

            open_req.set_onerror(Some(on_error_callback.as_ref().unchecked_ref()));
            on_error_callback.forget();
        });
        JsFuture::from(p)
            .await?
            .dyn_into::<IdbDatabase>()
            .map_err(Into::into)
    }

    #[allow(dead_code)]
    pub async fn new_async(db_name: &str) -> Self {
        let db = Self::open_db(db_name, None).await.unwrap();
        IndexeddbStorage {
            db: Arc::new(Mutex::new(db)),
        }
    }

    #[allow(dead_code)]
    pub fn new(_db_name: &str) -> Self {
        todo!()
    }

    pub fn make_table(&self, name: &str) -> crate::Result<()> {
        let db = self
            .db
            .lock()
            .map_err(|e| ClientError::Storage(e.to_string()))?;
        let key_path_id = js_sys::Array::new();
        key_path_id.push(&"partition".into());
        key_path_id.push(&"key".into());
        let mut create_params = IdbObjectStoreParameters::new();
        let db_store = db.create_object_store_with_optional_parameters(
            name,
            &create_params.key_path(Some(&key_path_id)),
        )?;
        let key_path_sortkey = js_sys::Array::new();
        key_path_sortkey.push(&"partition".into());
        key_path_sortkey.push(&"sortkey".into());
        let mut params = IdbIndexParameters::new();
        db_store.create_index_with_str_sequence_and_optional_parameters(
            "partition+sortkey",
            &key_path_sortkey,
            &params.unique(true),
        )?;

        Ok(())
    }

    pub fn table<T>(&self, name: &str) -> Box<dyn super::Table<T>>
    where
        T: StoreModel + 'static,
    {
        let table = IndexeddbTable::from(name.to_string(), self.db.clone());
        table
    }
}

#[derive(Debug)]
pub(super) struct IndexeddbTable<T>
where
    T: StoreModel,
{
    table_name: String,
    db: Arc<Mutex<IdbDatabase>>,
    _phantom: std::marker::PhantomData<T>,
}

#[allow(dead_code)]
impl<T: StoreModel + 'static> IndexeddbTable<T> {
    pub fn from(table_name: String, db: Arc<Mutex<IdbDatabase>>) -> Box<dyn super::Table<T>> {
        Box::new(IndexeddbTable {
            table_name,
            db,
            _phantom: std::marker::PhantomData,
        })
    }
}

unsafe impl<T: StoreModel> Send for IndexeddbTable<T> {}
unsafe impl<T: StoreModel> Sync for IndexeddbTable<T> {}

#[async_trait]
impl<T: StoreModel + 'static> super::Table<T> for IndexeddbTable<T> {
    async fn filter(
        &self,
        partition: &str,
        predicate: Box<dyn Fn(T) -> Option<T> + Send>,
    ) -> Vec<T> {
        let items = Arc::new(Mutex::new(Some(vec![])));
        let (tx, rx) = tokio::sync::oneshot::channel();
        {
            let store = match self
                .db
                .lock()
                .unwrap()
                .transaction_with_str_and_mode(&self.table_name, IdbTransactionMode::Readwrite)
                .and_then(|tx| tx.object_store(&self.table_name))
            {
                Ok(v) => v,
                Err(_) => return items.lock().unwrap().take().unwrap_or_default(),
            };
            let index = match store.index("partition+sortkey") {
                Ok(v) => v,
                Err(_) => return items.lock().unwrap().take().unwrap_or_default(),
            };
            let query_range: IdbKeyRange = match web_sys::IdbKeyRange::bound(
                &js_sys::Array::of2(&partition.into(), &"-Infinity".into()).into(),
                &js_sys::Array::of2(&partition.into(), &"Infinity".into()).into(),
            ) {
                Ok(v) => v,
                Err(_) => return items.lock().unwrap().take().unwrap_or_default(),
            };

            let cursor_req = match index.open_cursor_with_range(&query_range) {
                Ok(v) => v,
                Err(_) => return items.lock().unwrap().take().unwrap_or_default(),
            };
            let tx = Arc::new(Mutex::new(Some(tx)));
            let tx_ref = tx.clone();
            let items_ref = items.clone();
            let on_success_callback = Closure::wrap(Box::new(move |e: web_sys::Event| {
                let cursor = match e
                    .target()
                    .and_then(|v| v.dyn_into::<IdbRequest>().ok())
                    .and_then(|cursor_req| cursor_req.result().ok())
                {
                    Some(result) => match result.dyn_into::<web_sys::IdbCursorWithValue>() {
                        Ok(v) => v,
                        Err(e) => {
                            tx_ref
                                .lock()
                                .unwrap()
                                .take()
                                .and_then(|tx| tx.send(Err(e.into())).ok());
                            return;
                        }
                    },
                    None => {
                        tx_ref
                            .lock()
                            .unwrap()
                            .take()
                            .and_then(|tx| tx.send(Ok(())).ok());
                        return;
                    }
                };

                match cursor.value() {
                    Ok(v) => match serde_wasm_bindgen::from_value::<ValueItem>(v) {
                        Ok(v) => {
                            if let Ok(Some(item)) =
                                T::from_str(&v.value).map(|item| predicate(item))
                            {
                                items_ref.lock().unwrap().as_mut().unwrap().push(item);
                            }
                            cursor.continue_().ok();
                        }
                        Err(e) => {
                            tx_ref.lock().unwrap().take().and_then(|tx| {
                                tx.send(Err(ClientError::Storage(e.to_string()))).ok()
                            });
                        }
                    },
                    Err(e) => {
                        tx_ref
                            .lock()
                            .unwrap()
                            .take()
                            .and_then(|tx| tx.send(Err(e.into())).ok());
                    }
                }
            })
                as Box<dyn FnMut(web_sys::Event)>);
            let tx_ref = tx.clone();
            let on_error_callback = Closure::wrap(Box::new(move |e: DomException| {
                tx_ref
                    .lock()
                    .unwrap()
                    .take()
                    .and_then(|tx| tx.send(Err(ClientError::Storage(e.message()))).ok());
            }) as Box<dyn FnMut(DomException)>);

            cursor_req.set_onerror(Some(on_error_callback.as_ref().unchecked_ref()));
            on_error_callback.forget();

            cursor_req.set_onsuccess(Some(on_success_callback.as_ref().unchecked_ref()));
            on_success_callback.forget();
        }
        rx.await.ok();
        let x = items.lock().unwrap().take().unwrap_or_default();
        x
    }

    async fn query(&self, partition: &str, option: &QueryOption) -> QueryResult<T> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let items = Arc::new(Mutex::new(Some(Vec::<T>::new())));
        let start_sort_value = (match option.start_sort_value {
            Some(v) => v,
            None => match self.last(partition).await {
                Some(v) => v.sort_key(),
                None => 0,
            },
        } - option.limit as i64)
            .max(0);
        {
            let store = match self
                .db
                .lock()
                .unwrap()
                .transaction_with_str_and_mode(&self.table_name, IdbTransactionMode::Readwrite)
                .and_then(|tx| tx.object_store(&self.table_name))
            {
                Ok(v) => v,
                Err(_) => {
                    return QueryResult {
                        start_sort_value: 0,
                        end_sort_value: 0,
                        items: Vec::new(),
                    }
                }
            };
            let index = match store.index("partition+sortkey") {
                Ok(v) => v,
                Err(_) => {
                    return QueryResult {
                        start_sort_value: 0,
                        end_sort_value: 0,
                        items: Vec::new(),
                    }
                }
            };
            let query_range: IdbKeyRange = match web_sys::IdbKeyRange::bound(
                &js_sys::Array::of2(&partition.into(), &start_sort_value.into()).into(),
                &js_sys::Array::of2(&partition.into(), &"Infinity".into()).into(),
            ) {
                Ok(v) => v,
                Err(_) => {
                    return QueryResult {
                        start_sort_value: 0,
                        end_sort_value: 0,
                        items: Vec::new(),
                    }
                }
            };

            let cursor_req = match index.open_cursor_with_range(&query_range) {
                Ok(v) => v,
                Err(_) => {
                    return QueryResult {
                        start_sort_value: 0,
                        end_sort_value: 0,
                        items: Vec::new(),
                    }
                }
            };

            let tx = Arc::new(Mutex::new(Some(tx)));
            let tx_ref = tx.clone();
            let items_ref = items.clone();
            let limit = option.limit;
            let on_success_callback = Closure::wrap(Box::new(move |e: web_sys::Event| {
                let cursor = match e
                    .target()
                    .and_then(|v| v.dyn_into::<IdbRequest>().ok())
                    .and_then(|cursor_req| cursor_req.result().ok())
                {
                    Some(result) => match result.dyn_into::<web_sys::IdbCursorWithValue>() {
                        Ok(v) => v,
                        Err(e) => {
                            tx_ref
                                .lock()
                                .unwrap()
                                .take()
                                .and_then(|tx| tx.send(Err(e.into())).ok());
                            return;
                        }
                    },
                    None => {
                        tx_ref
                            .lock()
                            .unwrap()
                            .take()
                            .and_then(|tx| tx.send(Ok(())).ok());
                        return;
                    }
                };

                match cursor.value() {
                    Ok(v) => match serde_wasm_bindgen::from_value::<ValueItem>(v) {
                        Ok(v) => {
                            if let Ok(item) = T::from_str(&v.value) {
                                if let Some(items) = items_ref.lock().unwrap().as_mut() {
                                    items.push(item);
                                    if items.len() >= limit as usize {
                                        tx_ref
                                            .lock()
                                            .unwrap()
                                            .take()
                                            .and_then(|tx| tx.send(Ok(())).ok());
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            tx_ref.lock().unwrap().take().and_then(|tx| {
                                tx.send(Err(ClientError::Storage(e.to_string()))).ok()
                            });
                        }
                    },
                    Err(e) => {
                        tx_ref
                            .lock()
                            .unwrap()
                            .take()
                            .and_then(|tx| tx.send(Err(e.into())).ok());
                    }
                }
            })
                as Box<dyn FnMut(web_sys::Event)>);
            let tx_ref = tx.clone();
            let on_error_callback = Closure::wrap(Box::new(move |e: DomException| {
                tx_ref
                    .lock()
                    .unwrap()
                    .take()
                    .and_then(|tx| tx.send(Err(ClientError::Storage(e.message()))).ok());
            }) as Box<dyn FnMut(DomException)>);

            cursor_req.set_onerror(Some(on_error_callback.as_ref().unchecked_ref()));
            on_error_callback.forget();

            cursor_req.set_onsuccess(Some(on_success_callback.as_ref().unchecked_ref()));
            on_success_callback.forget();
        }
        rx.await.ok();

        let mut items = items.lock().unwrap().take().unwrap_or_default();
        items.reverse();

        QueryResult {
            start_sort_value: items.first().map(|v| v.sort_key()).unwrap_or(0),
            end_sort_value: items.last().map(|v| v.sort_key()).unwrap_or(0),
            items,
        }
    }

    async fn get(&self, partition: &str, key: &str) -> Option<T> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        {
            let store = match self
                .db
                .lock()
                .unwrap()
                .transaction_with_str_and_mode(&self.table_name, IdbTransactionMode::Readwrite)
                .and_then(|tx| tx.object_store(&self.table_name))
            {
                Ok(v) => v,
                Err(_) => return None,
            };

            let query_keys = js_sys::Array::new();
            query_keys.push(&partition.into());
            query_keys.push(&key.into());

            let get_req = store.get(&query_keys).ok()?;
            let tx = Arc::new(Mutex::new(Some(tx)));
            let tx_ref = tx.clone();
            let on_success_callback = Closure::wrap(Box::new(move |e: web_sys::Event| {
                let result = e
                    .target()
                    .and_then(|v| v.dyn_into::<IdbRequest>().ok())
                    .map(|v| v.result().unwrap_or(JsValue::UNDEFINED))
                    .unwrap_or(JsValue::UNDEFINED);
                tx_ref.lock().unwrap().take().and_then(|tx| {
                    tx.send(
                        serde_wasm_bindgen::from_value::<ValueItem>(result)
                            .map_err(|e| ClientError::Storage(e.to_string())),
                    )
                    .ok()
                });
            })
                as Box<dyn FnMut(web_sys::Event)>);

            let tx_ref = tx.clone();
            let on_error_callback = Closure::wrap(Box::new(move |e: DomException| {
                tx_ref
                    .lock()
                    .unwrap()
                    .take()
                    .and_then(|tx| tx.send(Err(ClientError::Storage(e.message()))).ok());
            }) as Box<dyn FnMut(DomException)>);

            get_req.set_onsuccess(Some(on_success_callback.as_ref().unchecked_ref()));
            on_success_callback.forget();

            get_req.set_onerror(Some(on_error_callback.as_ref().unchecked_ref()));
            on_error_callback.forget();
        }
        match rx.await {
            Ok(Ok(v)) => T::from_str(&v.value).ok(),
            _ => None,
        }
    }

    async fn set(&self, partition: &str, key: &str, value: Option<&T>) {
        if let Some(v) = value {
            let store = match self
                .db
                .lock()
                .unwrap()
                .transaction_with_str_and_mode(&self.table_name, IdbTransactionMode::Readwrite)
                .and_then(|tx| tx.object_store(&self.table_name))
            {
                Ok(v) => v,
                Err(_) => return,
            };

            let value = ValueItem {
                sortkey: v.sort_key(),
                partition: partition.to_string(),
                key: key.to_string(),
                value: v.to_string(),
            };

            serde_wasm_bindgen::to_value(&value)
                .map(|item| store.put(&item))
                .ok();
        } else {
            self.remove(partition, key).await;
        }
    }

    async fn remove(&self, partition: &str, key: &str) {
        if !key.is_empty() {
            let store = match self
                .db
                .lock()
                .unwrap()
                .transaction_with_str_and_mode(&self.table_name, IdbTransactionMode::Readwrite)
                .and_then(|tx| tx.object_store(&self.table_name))
            {
                Ok(v) => v,
                Err(_) => return,
            };
            let query_keys = js_sys::Array::new();
            query_keys.push(&partition.into());
            query_keys.push(&key.into());
            store.delete(&key.into()).ok();
            return;
        }
        let (tx, rx) = tokio::sync::oneshot::channel();
        {
            let store = match self
                .db
                .lock()
                .unwrap()
                .transaction_with_str_and_mode(&self.table_name, IdbTransactionMode::Readwrite)
                .and_then(|tx| tx.object_store(&self.table_name))
            {
                Ok(v) => v,
                Err(_) => return,
            };
            let index = match store.index("partition+sortkey") {
                Ok(v) => v,
                Err(_) => return,
            };
            let query_range: IdbKeyRange = match web_sys::IdbKeyRange::bound(
                &js_sys::Array::of2(&partition.into(), &"-Infinity".into()).into(),
                &js_sys::Array::of2(&partition.into(), &"Infinity".into()).into(),
            ) {
                Ok(v) => v,
                Err(_) => return,
            };

            let cursor_req = match index.open_cursor_with_range(&query_range) {
                Ok(v) => v,
                Err(_) => return,
            };

            let tx = Arc::new(Mutex::new(Some(tx)));
            let tx_ref = tx.clone();
            let on_success_callback = Closure::wrap(Box::new(move |e: web_sys::Event| {
                let result = e
                    .target()
                    .and_then(|v| v.dyn_into::<IdbRequest>().ok())
                    .map(|v| v.result().unwrap_or(JsValue::UNDEFINED));
                match result {
                    Some(v) => match v.dyn_into::<web_sys::IdbCursorWithValue>() {
                        Ok(cursor) => {
                            cursor.delete().ok();
                            cursor.continue_().ok();
                        }
                        Err(e) => {
                            tx_ref
                                .lock()
                                .unwrap()
                                .take()
                                .and_then(|tx| tx.send(Err(e.into())).ok());
                        }
                    },
                    None => {
                        tx_ref
                            .lock()
                            .unwrap()
                            .take()
                            .and_then(|tx| tx.send(Ok(())).ok());
                    }
                }
            })
                as Box<dyn FnMut(web_sys::Event)>);

            let tx_ref = tx.clone();
            let on_error_callback = Closure::wrap(Box::new(move |e: DomException| {
                tx_ref
                    .lock()
                    .unwrap()
                    .take()
                    .and_then(|tx| tx.send(Err(ClientError::Storage(e.message()))).ok());
            }) as Box<dyn FnMut(DomException)>);

            cursor_req.set_onerror(Some(on_error_callback.as_ref().unchecked_ref()));
            on_error_callback.forget();

            cursor_req.set_onsuccess(Some(on_success_callback.as_ref().unchecked_ref()));
            on_success_callback.forget();
        }
        rx.await.ok();
    }

    async fn last(&self, partition: &str) -> Option<T> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        {
            let store = self
                .db
                .lock()
                .unwrap()
                .transaction_with_str_and_mode(&self.table_name, IdbTransactionMode::Readwrite)
                .and_then(|tx| tx.object_store(&self.table_name))
                .ok()?;

            let index = store.index("partition+sortkey").ok()?;
            let key_range =
                IdbKeyRange::upper_bound_with_open(&JsValue::from_str(partition), true).ok()?;
            let cursor_request = index
                .open_cursor_with_range_and_direction(&key_range, web_sys::IdbCursorDirection::Prev)
                .ok()?;

            let tx = Arc::new(Mutex::new(Some(tx)));
            let tx_ref = tx.clone();
            let on_success_callback = Closure::wrap(Box::new(move |e: web_sys::Event| {
                let result = e
                    .target()
                    .and_then(|v| v.dyn_into::<IdbRequest>().ok())
                    .and_then(|cursor_req| cursor_req.result().ok())
                    .and_then(|result| result.dyn_into::<web_sys::IdbCursorWithValue>().ok())
                    .map(|result| result.value().ok().unwrap_or(JsValue::UNDEFINED))
                    .unwrap_or(JsValue::UNDEFINED);
                tx_ref.lock().unwrap().take().and_then(|tx| {
                    tx.send(
                        serde_wasm_bindgen::from_value::<ValueItem>(result)
                            .map_err(|e| ClientError::Storage(e.to_string())),
                    )
                    .ok()
                });
            })
                as Box<dyn FnMut(web_sys::Event)>);

            let tx_ref = tx.clone();
            let on_error_callback = Closure::wrap(Box::new(move |e: DomException| {
                tx_ref
                    .lock()
                    .unwrap()
                    .take()
                    .and_then(|tx| tx.send(Err(ClientError::Storage(e.message()))).ok());
            }) as Box<dyn FnMut(DomException)>);

            cursor_request.set_onsuccess(Some(on_success_callback.as_ref().unchecked_ref()));
            on_success_callback.forget();

            cursor_request.set_onerror(Some(on_error_callback.as_ref().unchecked_ref()));
            on_error_callback.forget();
        }
        match rx.await {
            Ok(Ok(v)) => T::from_str(&v.value).ok(),
            _ => None,
        }
    }

    async fn clear(&self) {
        let tx = match self
            .db
            .lock()
            .unwrap()
            .transaction_with_str_and_mode(&self.table_name, IdbTransactionMode::Readwrite)
        {
            Ok(v) => v,
            Err(_) => return,
        };
        tx.object_store(&self.table_name)
            .and_then(|store| store.clear())
            .ok();
    }
}
