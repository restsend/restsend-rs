use super::QueryOption;
use super::QueryResult;
use super::StoreModel;
use crate::error::ClientError;
use async_trait::async_trait;
use js_sys::Promise;
use serde::Deserialize;
use serde::Serialize;
use std::io::Cursor;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::vec;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::DomException;
use web_sys::IdbDatabase;
use web_sys::IdbIndexParameters;
use web_sys::IdbKeyRange;
use web_sys::IdbObjectStoreParameters;
use web_sys::IdbOpenDbRequest;
use web_sys::IdbRequest;
use web_sys::IdbTransactionMode;

const LAST_DB_VERSION: u32 = 1;
pub struct IndexeddbStorage {
    last_version: Option<u32>,
    db_prefix: String,
    memory_storage: super::memory::InMemoryStorage,
}

#[derive(Serialize, Deserialize)]
struct ValueItem {
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
    pub fn table<T>(&self) -> Box<dyn super::Table<T>>
    where
        T: StoreModel + 'static,
    {
        if self.db_prefix.is_empty() {
            return self.memory_storage.table::<T>();
        }
        let tbl_name = format!("{}-{}", self.db_prefix, super::table_name::<T>());

        IndexeddbTable::from(
            tbl_name.to_string(),
            self.last_version.unwrap_or(LAST_DB_VERSION),
        )
        .unwrap_or(self.memory_storage.table::<T>())
    }
}

#[derive(Debug)]
pub(super) struct IndexeddbTable<T>
where
    T: StoreModel,
{
    opened_rx: Arc<Mutex<Option<tokio::sync::oneshot::Receiver<()>>>>,
    opened_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
    table_name: String,
    last_error: Arc<Mutex<Option<ClientError>>>,
    db: Arc<Mutex<Option<IdbDatabase>>>,
    _phantom: std::marker::PhantomData<T>,
}

#[allow(dead_code)]
impl<T: StoreModel + 'static> IndexeddbTable<T> {
    pub fn from(table_name: String, version: u32) -> crate::Result<Box<dyn super::Table<T>>> {
        //TODO: sadly, ugly hack to make sure the table is created
        let idb = web_sys::window()
            .ok_or(ClientError::Storage("window is none".to_string()))?
            .indexed_db()?
            .ok_or(ClientError::Storage("indexed_db is none".to_string()))?;

        let open_req = idb.open_with_u32(&table_name, version)?;
        let db_result = Arc::new(Mutex::new(None));
        let last_error = Arc::new(Mutex::new(None));
        let table_name_ref = table_name.to_string();
        let on_upgradeneeded_callback = Closure::wrap(Box::new(move |e: web_sys::Event| {
            let db = e
                .target()
                .and_then(|v| v.dyn_into::<IdbOpenDbRequest>().ok())
                .map(|v| v.result().unwrap_or(JsValue::UNDEFINED))
                .unwrap_or(JsValue::UNDEFINED)
                .dyn_into::<IdbDatabase>()
                .unwrap();

            let key_path_id = js_sys::Array::new();
            key_path_id.push(&"partition".into());
            key_path_id.push(&"key".into());
            let mut create_params = IdbObjectStoreParameters::new();
            let db_store = db
                .create_object_store_with_optional_parameters(
                    &table_name_ref,
                    &create_params.key_path(Some(&key_path_id)),
                )
                .unwrap();

            let key_path_sortkey = js_sys::Array::new();
            key_path_sortkey.push(&"partition".into());
            key_path_sortkey.push(&"sortkey".into());
            let mut params = IdbIndexParameters::new();
            db_store
                .create_index_with_str_sequence_and_optional_parameters(
                    "partition+sortkey",
                    &key_path_sortkey,
                    &params.unique(false),
                )
                .unwrap();
        }) as Box<dyn FnMut(web_sys::Event)>);
        let db_result_ref = db_result.clone();
        let (tx, rx) = tokio::sync::oneshot::channel();
        let tx = Arc::new(Mutex::new(Some(tx)));
        let tx_ref = tx.clone();
        let table_name_ref = table_name.to_string();
        let on_success_callback = Closure::wrap(Box::new(move |e: web_sys::Event| {
            let result = e
                .target()
                .and_then(|v| v.dyn_into::<IdbOpenDbRequest>().ok())
                .map(|v| v.result().unwrap_or(JsValue::UNDEFINED))
                .unwrap_or(JsValue::UNDEFINED);
            db_result_ref
                .lock()
                .unwrap()
                .replace(result.dyn_into::<IdbDatabase>().unwrap());

            tx_ref.lock().unwrap().take().map(|tx| tx.send(()).ok());
        }) as Box<dyn FnMut(web_sys::Event)>);

        let last_error_ref = last_error.clone();
        let tx_ref = tx.clone();
        let table_name_ref = table_name.to_string();
        let on_error_callback = Closure::wrap(Box::new(move |e: DomException| {
            last_error_ref.lock().unwrap().replace(e.into());
            tx_ref.lock().unwrap().take().map(|tx| tx.send(()).ok());
        }) as Box<dyn FnMut(DomException)>);

        open_req.set_onupgradeneeded(Some(on_upgradeneeded_callback.as_ref().unchecked_ref()));
        on_upgradeneeded_callback.forget();

        open_req.set_onsuccess(Some(on_success_callback.as_ref().unchecked_ref()));
        on_success_callback.forget();

        open_req.set_onerror(Some(on_error_callback.as_ref().unchecked_ref()));
        on_error_callback.forget();

        Ok(Box::new(IndexeddbTable {
            opened_rx: Arc::new(Mutex::new(Some(rx))),
            opened_tx: tx,
            table_name: table_name.to_string(),
            db: db_result.clone(),
            last_error: last_error.clone(),
            _phantom: std::marker::PhantomData,
        }))
    }
}

unsafe impl<T: StoreModel> Send for IndexeddbTable<T> {}
unsafe impl<T: StoreModel> Sync for IndexeddbTable<T> {}

impl<T: StoreModel> IndexeddbTable<T> {
    async fn wait_opened(&self) -> crate::Result<()> {
        loop {
            if self.last_error.lock().unwrap().is_some() {
                return Err(self.last_error.lock().unwrap().as_ref().unwrap().clone());
            }
            if self.db.lock().unwrap().is_some() {
                return Ok(());
            }

            let rx = self.opened_rx.lock().unwrap().take();
            match rx {
                Some(rx) => {
                    return rx
                        .await
                        .map_err(|_| ClientError::Storage("wait_opened rx error".to_string()));
                }
                _ => {}
            }
        }
    }
}

#[async_trait]
impl<T: StoreModel + 'static> super::Table<T> for IndexeddbTable<T> {
    async fn filter(
        &self,
        partition: &str,
        predicate: Box<dyn Fn(T) -> Option<T> + Send>,
    ) -> Option<Vec<T>> {
        self.wait_opened().await.ok()?;

        let items = Arc::new(Mutex::new(Some(vec![])));
        let (tx, rx) = tokio::sync::oneshot::channel();
        {
            let store = self
                .db
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .transaction_with_str_and_mode(&self.table_name, IdbTransactionMode::Readonly)
                .and_then(|tx| tx.object_store(&self.table_name))
                .ok()?;

            let index = store.index("partition+sortkey").ok()?;
            let cursor_req = index.open_cursor().ok()?;
            let tx = Arc::new(Mutex::new(Some(tx)));
            let tx_ref = tx.clone();
            let items_ref = items.clone();
            let on_success_callback = Closure::wrap(Box::new(move |e: web_sys::Event| {
                let cursor = match e
                    .target()
                    .and_then(|v| v.dyn_into::<IdbRequest>().ok())
                    .and_then(|cursor_req| cursor_req.result().ok())
                    .and_then(|result| result.dyn_into::<web_sys::IdbCursorWithValue>().ok())
                {
                    Some(v) => v,
                    None => {
                        tx_ref
                            .lock()
                            .unwrap()
                            .take()
                            .and_then(|tx| tx.send(Ok(())).ok());
                        return;
                    }
                };
                let r = match cursor.value() {
                    Ok(v) => match serde_wasm_bindgen::from_value::<ValueItem>(v) {
                        Ok(v) => {
                            if let Ok(Some(item)) =
                                T::from_str(&v.value).map(|item| predicate(item))
                            {
                                items_ref.lock().unwrap().as_mut().unwrap().push(item);
                            }
                            cursor.continue_().ok();
                            None
                        }
                        Err(e) => Some(Err(ClientError::Storage(e.to_string()))),
                    },
                    Err(e) => Some(Err(e.into())),
                };
                if let Some(r) = r {
                    tx_ref.lock().unwrap().take().and_then(|tx| tx.send(r).ok());
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
        let items = items.lock().unwrap().take().unwrap_or_default();
        Some(items)
    }

    async fn query(&self, partition: &str, option: &QueryOption) -> Option<QueryResult<T>> {
        self.wait_opened().await.ok()?;

        let (tx, rx) = tokio::sync::oneshot::channel();
        let items = Arc::new(Mutex::new(Some(Vec::<T>::new())));
        let start_sort_value = (match option.start_sort_value {
            Some(v) => v,
            None => match self.last(partition).await {
                Some(v) => v.sort_key(),
                None => 0,
            },
        } - option.limit as i64)
            .max(0) as f64;
        {
            let store = self
                .db
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .transaction_with_str_and_mode(&self.table_name, IdbTransactionMode::Readonly)
                .and_then(|tx| tx.object_store(&self.table_name))
                .ok()?;

            let index = store.index("partition+sortkey").ok()?;
            let query_range: IdbKeyRange = web_sys::IdbKeyRange::bound_with_lower_open(
                &js_sys::Array::of2(&partition.into(), &start_sort_value.into()).into(),
                &js_sys::Array::of2(&partition.into(), &js_sys::Number::POSITIVE_INFINITY.into())
                    .into(),
                true,
            )
            .ok()?;

            let cursor_req = index.open_cursor_with_range(&query_range).ok()?;
            let tx = Arc::new(Mutex::new(Some(tx)));
            let tx_ref = tx.clone();
            let items_ref = items.clone();
            let limit = option.limit;
            let on_success_callback = Closure::wrap(Box::new(move |e: web_sys::Event| {
                let cursor = match e
                    .target()
                    .and_then(|v| v.dyn_into::<IdbRequest>().ok())
                    .and_then(|cursor_req| cursor_req.result().ok())
                    .and_then(|result| result.dyn_into::<web_sys::IdbCursorWithValue>().ok())
                {
                    Some(v) => v,
                    None => {
                        tx_ref
                            .lock()
                            .unwrap()
                            .take()
                            .and_then(|tx| tx.send(Ok(())).ok());
                        return;
                    }
                };

                let r = match cursor.value() {
                    Ok(v) => match serde_wasm_bindgen::from_value::<ValueItem>(v) {
                        Ok(v) => {
                            let mut items_count = 0;
                            if let Ok(item) = T::from_str(&v.value) {
                                if let Some(items) = items_ref.lock().unwrap().as_mut() {
                                    items.push(item);
                                    items_count = items.len();
                                }
                            }
                            if items_count >= limit as usize {
                                Some(Ok(()))
                            } else {
                                cursor.continue_().ok();
                                None
                            }
                        }
                        Err(e) => Some(Err(ClientError::Storage(e.to_string()))),
                    },
                    Err(e) => Some(Err(e.into())),
                };
                if let Some(r) = r {
                    tx_ref.lock().unwrap().take().and_then(|tx| tx.send(r).ok());
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

        Some(QueryResult {
            start_sort_value: items.first().map(|v| v.sort_key()).unwrap_or(0),
            end_sort_value: items.last().map(|v| v.sort_key()).unwrap_or(0),
            items,
        })
    }

    async fn get(&self, partition: &str, key: &str) -> Option<T> {
        self.wait_opened().await.ok()?;
        let (tx, rx) = tokio::sync::oneshot::channel();
        {
            let store = self
                .db
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .transaction_with_str_and_mode(&self.table_name, IdbTransactionMode::Readonly)
                .and_then(|tx| tx.object_store(&self.table_name))
                .ok()?;

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

    async fn set(&self, partition: &str, key: &str, value: Option<&T>) -> crate::Result<()> {
        self.wait_opened().await?;
        let value = match value {
            None => return self.remove(partition, key).await,
            Some(v) => v,
        };

        let (tx, rx) = tokio::sync::oneshot::channel();
        {
            let store = self
                .db
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .transaction_with_str_and_mode(&self.table_name, IdbTransactionMode::Readwrite)
                .and_then(|tx| tx.object_store(&self.table_name))?;

            let item = ValueItem {
                sortkey: value.sort_key() as f64,
                partition: partition.to_string(),
                key: key.to_string(),
                value: value.to_string(),
            };

            let item = serde_wasm_bindgen::to_value(&item)
                .map_err(|e| ClientError::Storage(e.to_string()))?;
            let put_req = store.put(&item)?;
            let tx = Arc::new(Mutex::new(Some(tx)));
            let tx_ref = tx.clone();
            let sortkey = value.sort_key();
            let key_ref = key.to_string();
            let partition_ref = partition.to_string();
            let on_success_callback = Closure::wrap(Box::new(move |e: web_sys::Event| {
                tx_ref
                    .lock()
                    .unwrap()
                    .take()
                    .and_then(|tx| tx.send(Ok(())).ok());
            })
                as Box<dyn FnMut(web_sys::Event)>);

            put_req.set_onsuccess(Some(on_success_callback.as_ref().unchecked_ref()));
            on_success_callback.forget();

            let tx_ref = tx.clone();
            let on_error_callback = Closure::wrap(Box::new(move |e: DomException| {
                tx_ref
                    .lock()
                    .unwrap()
                    .take()
                    .and_then(|tx| tx.send(Err(ClientError::Storage(e.message()))).ok());
            }) as Box<dyn FnMut(DomException)>);
            put_req.set_onerror(Some(on_error_callback.as_ref().unchecked_ref()));
            on_error_callback.forget();
        }
        match rx.await {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(e)) => Err(e),
            _ => Ok(()),
        }
    }

    async fn remove(&self, partition: &str, key: &str) -> crate::Result<()> {
        self.wait_opened().await?;
        let (tx, rx) = tokio::sync::oneshot::channel();
        {
            let store = self
                .db
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .transaction_with_str_and_mode(&self.table_name, IdbTransactionMode::Readwrite)
                .and_then(|tx| tx.object_store(&self.table_name))?;

            let cursor_req = if !key.is_empty() {
                let query_keys = js_sys::Array::new();
                query_keys.push(&partition.into());
                query_keys.push(&key.into());
                store.delete(&key.into())
            } else {
                let index = store.index("partition+sortkey")?;
                let query_range: IdbKeyRange = web_sys::IdbKeyRange::bound(
                    &js_sys::Array::of2(
                        &partition.into(),
                        &js_sys::Number::NEGATIVE_INFINITY.into(),
                    ),
                    &js_sys::Array::of2(
                        &partition.into(),
                        &js_sys::Number::POSITIVE_INFINITY.into(),
                    ),
                )?;
                index.open_key_cursor_with_range(&query_range)
            }?;

            let tx = Arc::new(Mutex::new(Some(tx)));
            let tx_ref = tx.clone();
            let partition = partition.to_string();
            let on_success_callback = Closure::wrap(Box::new(move |e: web_sys::Event| {
                let cursor = match e
                    .target()
                    .and_then(|v| v.dyn_into::<IdbRequest>().ok())
                    .and_then(|cursor_req| cursor_req.result().ok())
                    .and_then(|result| result.dyn_into::<web_sys::IdbCursor>().ok())
                {
                    Some(v) => v,
                    None => {
                        tx_ref
                            .lock()
                            .unwrap()
                            .take()
                            .and_then(|tx| tx.send(Ok(())).ok());
                        return;
                    }
                };

                let r = match cursor.key() {
                    Ok(keys) => match keys.dyn_into::<js_sys::Array>() {
                        Ok(v) => {
                            if v.get(0).as_string().unwrap_or_default() == partition {
                                cursor.delete().ok();
                            }
                            cursor.continue_().ok();
                            None
                        }
                        Err(e) => Some((Err(e.into()))),
                    },
                    Err(e) => Some((Err(e.into()))),
                };
                if let Some(r) = r {
                    tx_ref.lock().unwrap().take().and_then(|tx| tx.send(r).ok());
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
        match rx.await {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(e)) => Err(e),
            _ => Ok(()),
        }
    }

    async fn last(&self, partition: &str) -> Option<T> {
        self.wait_opened().await.ok()?;
        let (tx, rx) = tokio::sync::oneshot::channel();
        {
            let store = self
                .db
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .transaction_with_str_and_mode(&self.table_name, IdbTransactionMode::Readonly)
                .and_then(|tx| tx.object_store(&self.table_name))
                .ok()?;

            let index = store.index("partition+sortkey").ok()?;
            let query_range: IdbKeyRange = web_sys::IdbKeyRange::bound(
                &js_sys::Array::of2(&partition.into(), &js_sys::Number::NEGATIVE_INFINITY.into()),
                &js_sys::Array::of2(&partition.into(), &js_sys::Number::POSITIVE_INFINITY.into()),
            )
            .ok()?;

            let cursor_request = index
                .open_cursor_with_range_and_direction(
                    &query_range,
                    web_sys::IdbCursorDirection::Prev,
                )
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

    async fn clear(&self) -> crate::Result<()> {
        self.db
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .delete_object_store(&self.table_name)
            .map_err(Into::into)
    }
}
