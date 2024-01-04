use super::QueryOption;
use super::QueryResult;
use super::StoreModel;
use crate::error::ClientError;
use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use wasm_bindgen::closure::Closure;
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
    pub async fn open_db(db_name: &str) -> crate::Result<IdbDatabase> {
        todo!()
    }

    pub fn new(db: IdbDatabase) -> Self {
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
