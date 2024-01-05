use super::QueryOption;
use super::QueryResult;
use super::StoreModel;
use crate::error::ClientError;
use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::DomException;
use web_sys::IdbDatabase;
use web_sys::IdbObjectStore;

pub struct IndexeddbStorage {
    db: Arc<Mutex<IdbDatabase>>,
}

impl IndexeddbStorage {
    pub async fn open_db(db_name: &str, version: Option<u32>) -> crate::Result<IdbDatabase> {
        let idb = web_sys::window()
            .ok_or(ClientError::Storage("window is none".to_string()))?
            .indexed_db()?
            .ok_or(ClientError::Storage("indexed_db is none".to_string()))?;

        let open_req = idb.open_with_u32(db_name, version.unwrap_or(1))?;
        loop {
            match open_req.ready_state() {
                web_sys::IdbRequestReadyState::Done => break,
                _ => {
                    crate::utils::sleep(Duration::from_millis(20)).await;
                    //let _ = JsFuture::from(js_sys::Promise::resolve(&JsValue::NULL)).await?;
                }
            }
        }
        let db = open_req.result()?;
        db.dyn_into::<IdbDatabase>().map_err(|e| e.into())
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
        let db = self.db.lock().unwrap();
        let db_store = db.create_object_store(name)?;
        db_store
            .create_index_with_str("sort_key", "__sort_key")
            .map(|_| ())
            .map_err(|e| e.into())
    }

    pub fn table<T>(&self, name: &str) -> Box<dyn super::Table<T>>
    where
        T: StoreModel + 'static,
    {
        let db = self.db.lock().unwrap();
        let db_store = db.create_object_store(name).unwrap();
        let table = IndexeddbTable::from(db_store);
        table
    }
}

#[derive(Debug)]
pub(super) struct IndexeddbTable<T>
where
    T: StoreModel,
{
    data: IdbObjectStore,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: StoreModel + 'static> IndexeddbTable<T> {
    pub fn from(t: IdbObjectStore) -> Box<dyn super::Table<T>> {
        Box::new(IndexeddbTable {
            data: t,
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
        todo!()
    }

    async fn set(&self, partition: &str, key: &str, value: Option<&T>) {
        todo!()
    }

    async fn remove(&self, partition: &str, key: &str) {
        todo!()
    }

    async fn last(&self, partition: &str) -> Option<T> {
        todo!()
    }

    async fn clear(&self) {
        self.data.clear().ok();
    }
}
