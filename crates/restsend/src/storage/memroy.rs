use lrumap::{LruHashMap, Removed};
struct MemoryTable {
    name: String,
    max_items: usize,
    data: LruHashMap<String, String>,
}
pub(crate) struct MemoryStorage {}

impl super::Storage for MemoryStorage {
    fn prepare() -> Result<()> {
        Ok(())
    }
    fn table(&self, name: &str) -> Box<dyn super::Table> {
        Box::new(MemoryTable::new(name))
    }
}
