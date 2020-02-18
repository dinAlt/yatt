pub mod errors;
pub mod statement;

use chrono::prelude::*;
use chrono::{DateTime, Utc};
use uuid::Uuid;

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

pub trait St {
    fn save(&self, item: impl Sized) -> DBResult<usize>;
    fn all<T: Sized>(&self) -> DBResult<Vec<T>>;
    fn remove(&self, item: impl Sized) -> DBResult<()>;
    fn by_statement<T: Sized>(&self, s: Statement) -> DBResult<Vec<T>>;
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

#[derive(Debug, Clone)]
pub enum FieldVal {
    Usize(usize),
    DateTime(DateTime<Utc>),
    String(String),
    Bool(bool),
    Null,
}

impl From<usize> for FieldVal {
    fn from(u: usize) -> FieldVal {
        FieldVal::Usize(u)
    }
}
impl From<DateTime<Local>> for FieldVal {
    fn from(val: DateTime<Local>) -> FieldVal {
        FieldVal::DateTime(DateTime::from(val))
    }
}
impl From<DateTime<Utc>> for FieldVal {
    fn from(val: DateTime<Utc>) -> FieldVal {
        FieldVal::DateTime(val)
    }
}
impl From<&str> for FieldVal {
    fn from(val: &str) -> FieldVal {
        FieldVal::String(val.to_string())
    }
}
impl From<String> for FieldVal {
    fn from(val: String) -> FieldVal {
        FieldVal::String(val)
    }
}
impl From<&String> for FieldVal {
    fn from(val: &String) -> FieldVal {
        FieldVal::String(val.clone())
    }
}
impl From<&FieldVal> for FieldVal {
    fn from(val: &FieldVal) -> FieldVal {
        (*val).clone()
    }
}
impl From<bool> for FieldVal {
    fn from(val: bool) -> FieldVal {
        FieldVal::Bool(val.clone())
    }
}
impl From<&bool> for FieldVal {
    fn from(val: &bool) -> FieldVal {
        FieldVal::Bool(val.clone())
    }
}
impl From<Option<DateTime<Utc>>> for FieldVal {
    fn from(val: Option<DateTime<Utc>>) -> FieldVal {
        if let Some(d) = val {
            FieldVal::DateTime(d)
        } else {
            FieldVal::Null
        }
    }
}
impl From<Option<usize>> for FieldVal {
    fn from(val: Option<usize>) -> FieldVal {
        if let Some(v) = val {
            FieldVal::Usize(v)
        } else {
            FieldVal::Null
        }
    }
}

pub trait HistoryStorage {
    fn push_record(&self, r: HistoryRecord) -> DBResult<()>;
    fn get_entity_guid(&self, id: usize, entity_type: &str) -> DBResult<Uuid>;
}

pub trait StoreObject {
    fn get_field_val(&self, field_name: &str) -> FieldVal;
    fn get_type_name(&self) -> &'static str;
    fn get_fields_list(&self) -> &'static [&'static str];
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
