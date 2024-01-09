use super::QueryOption;
use super::QueryResult;
use super::StoreModel;
use crate::error::ClientError;
use async_trait::async_trait;
use serde::Deserialize;
use serde::Serialize;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::wasm_bindgen::closure::Closure;
use web_sys::wasm_bindgen::JsValue;
use web_sys::DomException;
use web_sys::IdbCursorWithValue;
use web_sys::IdbDatabase;
use web_sys::IdbIndex;
use web_sys::IdbIndexParameters;
use web_sys::IdbKeyRange;
use web_sys::IdbObjectStore;
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
        let (open_req_tx, open_req_rx) = tokio::sync::oneshot::channel();
        let open_req_tx = Arc::new(Mutex::new(Some(open_req_tx)));
        let open_req_tx_ref = open_req_tx.clone();
        let on_error_callback = Closure::wrap(Box::new(move |e: DomException| {
            open_req_tx_ref.lock().map(|mut tx| {
                tx.take().and_then(|tx| tx.send(Err(e.into())).ok());
            });
        }) as Box<dyn FnMut(DomException)>);
        open_req.set_onerror(Some(on_error_callback.as_ref().unchecked_ref()));
        on_error_callback.forget();

        let on_success_callback = Closure::wrap(Box::new(move |e: web_sys::Event| {
            open_req_tx.lock().map(|mut tx| {
                tx.take().and_then(|tx| tx.send(Ok(())).ok());
            });
        }) as Box<dyn FnMut(web_sys::Event)>);
        on_success_callback.forget();

        match open_req_rx.await {
            Ok(v) => match v {
                Ok(_) => match open_req.result() {
                    Ok(v) => return v.dyn_into::<IdbDatabase>().map_err(|e| e.into()),
                    Err(e) => return Err(e.into()),
                },
                Err(e) => return Err(e),
            },
            Err(e) => return Err(ClientError::Storage(e.to_string())),
        }
    }

    pub async fn new_async(db_name: &str) -> Self {
        let db = Self::open_db(db_name, None).await.unwrap();
        IndexeddbStorage {
            db: Arc::new(Mutex::new(db)),
        }
    }

    pub fn new(db_name: &str) -> Self {
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
impl<T: StoreModel> super::Table<T> for IndexeddbTable<T> {
    async fn filter(
        &self,
        partition: &str,
        predicate: Box<dyn Fn(T) -> Option<T> + Send>,
    ) -> Vec<T> {
        todo!()
    }

    async fn query(&self, partition: &str, option: &QueryOption) -> QueryResult<T> {
        todo!()
    }

    async fn get(&self, partition: &str, key: &str) -> Option<T> {
        let (get_req_tx, get_req_rx) = tokio::sync::oneshot::channel();
        {
            let tx = self
                .db
                .lock()
                .ok()?
                .transaction_with_str_and_mode(&self.table_name, IdbTransactionMode::Readonly)
                .ok()?;
            let store = tx.object_store(&self.table_name).ok()?;

            let query_keys = js_sys::Array::new();
            query_keys.push(&partition.into());
            query_keys.push(&key.into());

            let get_req = match store.get(&query_keys) {
                Ok(v) => v,
                Err(e) => return None,
            };

            let get_req_tx = Arc::new(Mutex::new(Some(get_req_tx)));
            let get_req_tx_ref = get_req_tx.clone();

            let on_error_callback = Closure::wrap(Box::new(move |e: DomException| {
                get_req_tx_ref.lock().map(|mut tx| {
                    tx.take()
                        .and_then(|tx| tx.send(Err(ClientError::Storage(e.message()))).ok());
                });
            }) as Box<dyn FnMut(DomException)>);

            get_req.set_onerror(Some(on_error_callback.as_ref().unchecked_ref()));
            on_error_callback.forget();

            let on_success_callback = Closure::wrap(Box::new(move |e: web_sys::Event| {
                get_req_tx.lock().map(|mut tx| {
                    if let Some(tx) = tx.take() {
                        let r = e
                            .target()
                            .ok_or_else(|| {
                                ClientError::Storage("target is not IdbRequest".to_string())
                            })
                            .and_then(|target| {
                                target.dyn_into::<IdbRequest>().map_err(|_| {
                                    ClientError::Storage("target is not IdbRequest".to_string())
                                })
                            })
                            .and_then(|get_req| {
                                get_req
                                    .result()
                                    .map(|result| result.as_string().unwrap_or_default())
                                    .map_err(Into::into)
                                    .map(|r| serde_json::from_str::<ValueItem>(&r))
                            });
                        tx.send(r);
                    }
                });
            })
                as Box<dyn FnMut(web_sys::Event)>);
            get_req.set_onsuccess(Some(on_success_callback.as_ref().unchecked_ref()));
            on_success_callback.forget();
        }
        match get_req_rx.await.ok() {
            Some(v) => match v.ok()? {
                Ok(v) => match T::from_str(&v.value) {
                    Ok(v) => Some(v),
                    Err(e) => None,
                },
                Err(_) => None,
            },
            None => None,
        }
    }

    async fn set(&self, partition: &str, key: &str, value: Option<&T>) {
        match value {
            None => self.remove(partition, key).await,
            Some(value) => {
                let tx = self
                    .db
                    .lock()
                    .unwrap()
                    .transaction_with_str_and_mode(&self.table_name, IdbTransactionMode::Readwrite)
                    .unwrap();
                let store = tx.object_store(&self.table_name).unwrap();
                let value = ValueItem {
                    sortkey: value.sort_key(),
                    partition: partition.to_string(),
                    key: key.to_string(),
                    value: value.to_string(),
                };

                serde_wasm_bindgen::to_value(&value)
                    .map(|item| store.put(&item))
                    .ok();
            }
        }
    }

    async fn remove(&self, partition: &str, key: &str) {
        if !key.is_empty() {
            let tx = self
                .db
                .lock()
                .unwrap()
                .transaction_with_str_and_mode(&self.table_name, IdbTransactionMode::Readwrite)
                .unwrap();
            let store = tx.object_store(&self.table_name).unwrap();
            let query_keys = js_sys::Array::new();
            query_keys.push(&partition.into());
            query_keys.push(&key.into());
            store.delete(&key.into()).ok();
            return;
        }

        let (cursor_req_tx, cursor_req_rx) = tokio::sync::oneshot::channel::<crate::Result<()>>();
        {
            let tx = self
                .db
                .lock()
                .unwrap()
                .transaction_with_str_and_mode(&self.table_name, IdbTransactionMode::Readwrite)
                .unwrap();
            let store = tx.object_store(&self.table_name).unwrap();
            let index = match store.index("partition+sortkey").ok() {
                Some(v) => v,
                None => return,
            };

            let query_range = match web_sys::IdbKeyRange::bound(
                &js_sys::Array::of2(&partition.into(), &"-Infinity".into()).into(),
                &js_sys::Array::of2(&partition.into(), &"Infinity".into()).into(),
            ) {
                Ok(v) => v,
                Err(e) => return,
            };

            let mut cursor_req = index.open_cursor_with_range(&query_range).unwrap();
            let cursor_req_tx = Arc::new(Mutex::new(Some(cursor_req_tx)));
            let cursor_req_tx_ref = cursor_req_tx.clone();
            let on_sucess_callback = Closure::wrap(Box::new(move |e: web_sys::Event| {
                let cursor = e
                    .target()
                    .ok_or_else(|| ClientError::Storage("target is not IdbRequest".to_string()))
                    .and_then(|target| {
                        target.dyn_into::<IdbRequest>().map_err(|_| {
                            ClientError::Storage("target is not IdbRequest".to_string())
                        })
                    })
                    .and_then(|cursor_req| {
                        cursor_req
                            .result()
                            .map_err(|_| ClientError::Storage("cursor done".to_string()))
                            .and_then(|result| {
                                result
                                    .dyn_into::<web_sys::IdbCursorWithValue>()
                                    .map_err(|_| {
                                        ClientError::Storage(
                                            "cursor is not IdbCursorWithValue".to_string(),
                                        )
                                    })
                            })
                    });
                match cursor {
                    Ok(cursor) => {
                        cursor.delete().ok();
                        cursor.continue_().ok();
                    }
                    Err(e) => {
                        cursor_req_tx_ref.lock().map(|mut tx| {
                            tx.take().and_then(|tx| {
                                tx.send(Err(ClientError::Storage(e.to_string()))).ok()
                            });
                        });
                    }
                }
            })
                as Box<dyn FnMut(web_sys::Event)>);
            cursor_req.set_onsuccess(Some(on_sucess_callback.as_ref().unchecked_ref()));
            on_sucess_callback.forget();

            let cursor_req_tx = cursor_req_tx.clone();
            let onerror_callback = Closure::wrap(Box::new(move |e: DomException| {
                cursor_req_tx.lock().map(|mut tx| {
                    tx.take()
                        .and_then(|tx| tx.send(Err(ClientError::Storage(e.message()))).ok());
                });
            }) as Box<dyn FnMut(DomException)>);
            cursor_req.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
            onerror_callback.forget();
        }
        match cursor_req_rx.await {
            Ok(v) => match v {
                Ok(_) => {}
                Err(e) => {}
            },
            Err(e) => {}
        }
    }

    async fn last(&self, partition: &str) -> Option<T> {
        let (cursor_req_tx, cursor_req_rx) = tokio::sync::oneshot::channel();
        let cursor_req_tx = Arc::new(Mutex::new(Some(cursor_req_tx)));
        {
            let tx = self
                .db
                .lock()
                .unwrap()
                .transaction_with_str_and_mode(&self.table_name, IdbTransactionMode::Readwrite)
                .unwrap();
            let store = tx.object_store(&self.table_name).unwrap();
            let index = match store.index("partition+sortkey").ok() {
                Some(v) => v,
                None => return None,
            };
            let key_range =
                IdbKeyRange::upper_bound_with_open(&JsValue::from_str(partition), true).unwrap();

            let cursor_request = match index
                .open_cursor_with_range_and_direction(&key_range, web_sys::IdbCursorDirection::Prev)
            {
                Ok(v) => v,
                Err(e) => return None,
            };

            let cursor_req_tx_ref = cursor_req_tx.clone();
            let on_success_callback = Closure::wrap(Box::new(move |e: web_sys::Event| {
                if let Some(tx) = cursor_req_tx_ref.lock().unwrap().take() {
                    if let Some(cursor) = e
                        .target()
                        .and_then(|target| target.dyn_into::<IdbRequest>().ok())
                        .and_then(|cursor_req| cursor_req.result().ok())
                        .and_then(|result| result.dyn_into::<web_sys::IdbCursorWithValue>().ok())
                    {
                        let r = cursor
                            .value()
                            .map(|v| v.as_string().unwrap_or_default())
                            .map(|v| {
                                serde_json::from_str::<ValueItem>(&v)
                                    .map_err(|e| ClientError::Storage(e.to_string()))
                            })
                            .map_err(Into::into);
                        tx.send(r).ok();
                    } else {
                        tx.send(Err(ClientError::Storage("cursor error".to_string())))
                            .ok();
                    }
                }
            })
                as Box<dyn FnMut(web_sys::Event)>);
            cursor_request.set_onsuccess(Some(on_success_callback.as_ref().unchecked_ref()));
            on_success_callback.forget();

            let cursor_req_tx_ref = cursor_req_tx.clone();
            let on_error_callback = Closure::wrap(Box::new(move |e: DomException| {
                cursor_req_tx_ref.lock().map(|mut tx| {
                    tx.take()
                        .and_then(|tx| tx.send(Err(ClientError::Storage(e.message()))).ok());
                });
            }) as Box<dyn FnMut(DomException)>);
            cursor_request.set_onerror(Some(on_error_callback.as_ref().unchecked_ref()));
            on_error_callback.forget();
        }

        match cursor_req_rx.await {
            Ok(Ok(Ok(v))) => T::from_str(&v.value).ok(),
            _ => None,
        }
    }

    async fn clear(&self) {
        let tx = self
            .db
            .lock()
            .unwrap()
            .transaction_with_str_and_mode(&self.table_name, IdbTransactionMode::Readwrite)
            .unwrap();
        let store = tx.object_store(&self.table_name).unwrap();
        store.clear().ok();
    }
}

async fn iter_with_range(
    store: &IdbObjectStore,
    index: IdbIndex,
    range: IdbKeyRange,
    predicate: Box<dyn Fn(IdbCursorWithValue) -> Option<ValueItem> + Send>,
) -> crate::Result<()> {
    let (cursor_req_tx, cursor_req_rx) = tokio::sync::oneshot::channel::<crate::Result<()>>();
    {
        let mut cursor_req = index.open_cursor_with_range(&index).unwrap();
        let cursor_req_tx = Arc::new(Mutex::new(Some(cursor_req_tx)));
        let cursor_req_tx_ref = cursor_req_tx.clone();
        let on_sucess_callback = Closure::wrap(Box::new(move |e: web_sys::Event| {
            let cursor = e
                .target()
                .ok_or_else(|| ClientError::Storage("target is not IdbRequest".to_string()))
                .and_then(|target| {
                    target
                        .dyn_into::<IdbRequest>()
                        .map_err(|_| ClientError::Storage("target is not IdbRequest".to_string()))
                })
                .and_then(|cursor_req| {
                    cursor_req
                        .result()
                        .map_err(|_| ClientError::Storage("cursor done".to_string()))
                        .and_then(|result| {
                            result
                                .dyn_into::<web_sys::IdbCursorWithValue>()
                                .map_err(|_| {
                                    ClientError::Storage(
                                        "cursor is not IdbCursorWithValue".to_string(),
                                    )
                                })
                        })
                });
            match cursor {
                Ok(cursor) => {
                    predicate(cursor);
                }
                Err(e) => {
                    cursor_req_tx_ref.lock().map(|mut tx| {
                        tx.take()
                            .and_then(|tx| tx.send(Err(ClientError::Storage(e.to_string()))).ok());
                    });
                }
            }
        }) as Box<dyn FnMut(web_sys::Event)>);
        cursor_req.set_onsuccess(Some(on_sucess_callback.as_ref().unchecked_ref()));
        on_sucess_callback.forget();

        let cursor_req_tx = cursor_req_tx.clone();
        let onerror_callback = Closure::wrap(Box::new(move |e: DomException| {
            cursor_req_tx.lock().map(|mut tx| {
                tx.take()
                    .and_then(|tx| tx.send(Err(ClientError::Storage(e.message()))).ok());
            });
        }) as Box<dyn FnMut(DomException)>);
        cursor_req.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
        onerror_callback.forget();
    }
    cursor_req_rx.await.map(|_| ())
}
