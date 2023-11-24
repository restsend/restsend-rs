use serde::Serializer;

mod memroy;
mod sqlite;

pub trait SortKey<T>
where
    T: Sized,
{
    fn sorted_value(&self) -> &T;
}

pub struct QureyResult<T>
where
    T: Serializer + SortKey<T>,
{
    pub items: Vec<T>,
    pub has_more: bool,
}

pub trait Table: Sync + Send {
    fn get<T>(&self, key: &str) -> Result<T>
    where
        T: Serializer;
    fn set<T>(&self, key: &str, value: &T)
    where
        T: Serializer;
    fn query<T>(&self, key: &str, pos: u32, limit: u32) -> Result<QureyResult<T>>;
    fn remove(&self, key: &str);
    fn clear(&self);
}

pub trait Storage {
    fn prepare() -> Result<()>;
    fn table(&self, name: &str) -> Box<dyn Table>;
}
