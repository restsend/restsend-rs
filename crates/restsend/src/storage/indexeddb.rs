use crate::error::ClientError;

use super::QueryOption;
use super::QueryResult;
use super::StoreModel;
use indexed_db_futures::prelude::*;
use std::sync::{Arc, Mutex};
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::Event;
use web_sys::IdbDatabase;
use web_sys::IdbObjectStore;
use web_sys::IdbOpenDbRequest;
pub struct IndexeddbStorage {
    db: Arc<Mutex<IdbDatabase>>,
}

impl IndexeddbStorage {
    fn open_db(db_name: &str) -> crate::Result<IdbDatabase> {
        let idb = web_sys::window()
            .ok_or(ClientError::Storage("window is none".to_string()))?
            .indexed_db()?
            .ok_or(ClientError::Storage("indexed_db is none".to_string()))?;

        let db = idb.open(&db_name)?;
        db.result()?.dyn_into::<IdbDatabase>().map_err(|e| e.into())
    }

    pub fn new(db_name: &str) -> Self {
        let db = Self::open_db(db_name).unwrap();
        IndexeddbStorage {
            db: Arc::new(Mutex::new(db)),
        }
    }

    pub fn make_table(&self, name: &str) -> crate::Result<()> {
        let db_store = self.db.lock().unwrap().create_object_store(name)?;
        db_store.create_index_with_str("sort_key", "__sort_key")?;
        Ok(())
    }

    pub fn table<T>(&self, name: &str) -> Box<dyn super::Table<T>>
    where
        T: StoreModel + 'static,
    {
        let t = self.db.lock().unwrap().create_object_store(name).unwrap();
        let table = IndexeddbTable::from(t);
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

impl<T: StoreModel> super::Table<T> for IndexeddbTable<T> {
    fn filter(&self, partition: &str, predicate: Box<dyn Fn(T) -> Option<T>>) -> Vec<T> {
        todo!()
    }
    fn query(&self, partition: &str, option: &QueryOption) -> QueryResult<T> {
        todo!()
    }
    fn get(&self, partition: &str, key: &str) -> Option<T> {
        todo!()
    }

    fn set(&self, partition: &str, key: &str, value: Option<&T>) {
        todo!()
    }

    fn remove(&self, partition: &str, key: &str) {
        todo!()
    }

    fn last(&self, partition: &str) -> Option<T> {
        todo!()
    }

    fn clear(&self) {
        self.data.clear().ok();
    }
}
