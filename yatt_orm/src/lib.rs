use chrono::{DateTime, Utc};
use uuid::Uuid;

pub mod errors;
pub mod statement;

pub use errors::*;
pub use yatt_orm_derive::*;

use statement::*;

pub type BoxStorage<T> = Box<dyn Storage<Item = T>>;

pub trait Storage {
    type Item;
    fn save(&self, item: &Self::Item) -> DBResult<usize>;
    fn all(&self) -> DBResult<Vec<Self::Item>>;
    fn remove(&self, id: usize) -> DBResult<()>;
    fn by_statement(&self, s: Statement) -> DBResult<Vec<Self::Item>>;
}

impl<T: Clone> dyn Storage<Item = T> {
    pub fn by_id(&self, id: usize) -> DBResult<T> {
        let res = self.by_statement(filter(eq("id".to_string(), id)))?;
        if res.is_empty() {
            return Err(DBError::IsEmpty {
                message: format!("no row with id {}", id),
            });
        }

        Ok(res.first().unwrap().to_owned())
    }
    pub fn filter(&self, f: Filter) -> DBResult<Vec<T>> {
        let res = self.by_statement(filter(f))?;
        Ok(res)
    }
    pub fn with_max(&self, field: &str) -> DBResult<Option<T>> {
        let res = self.by_statement(sort(field, SortDir::Descend).limit(1))?;
        if res.is_empty() {
            return Ok(None);
        }
        Ok(Some(res.first().unwrap().to_owned()))
    }
}

#[derive(Clone, Copy)]
pub enum HistoryRecordType {
    Create,
    Update,
    Delete,
}

impl From<usize> for HistoryRecordType {
    fn from(u: usize) -> HistoryRecordType {
        match u {
            0 => HistoryRecordType::Create,
            1 => HistoryRecordType::Update,
            2 => HistoryRecordType::Delete,
            _ => panic!("wrong argument value"),
        }
    }
}

impl From<HistoryRecordType> for isize {
    fn from(r: HistoryRecordType) -> isize {
        match r {
            HistoryRecordType::Create => 0,
            HistoryRecordType::Update => 1,
            HistoryRecordType::Delete => 2,
        }
    }
}

pub struct HistoryRecord {
    pub date: DateTime<Utc>,
    pub uuid: Uuid,
    pub record_type: HistoryRecordType,
    pub entity_type: String,
    pub entity_id: usize,
}

pub trait HistoryStorage {
    fn push_record(&self, r: HistoryRecord) -> DBResult<()>;
    fn get_entity_guid(&self, id: usize, entity_type: &str) -> DBResult<Uuid>;
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
