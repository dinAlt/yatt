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
    fn save(&self, item: impl StoreObject) -> DBResult<usize>;
    fn get_all<T: StoreObject>(&self) -> DBResult<Vec<T>>;
    fn remove_by_filter(&self, object_type_name: &str, filter: Filter) -> DBResult<()>;
    fn get_by_statement<T: StoreObject>(&self, s: Statement) -> DBResult<Vec<T>>;

    fn get_by_id<T: StoreObject>(&self, id: usize) -> DBResult<T> {
        let res = self.get_by_statement::<T>(filter(eq(&"id", id)))?;
        if res.is_empty() {
            return Err(DBError::IsEmpty {
                message: format!("no row with id {}", id),
            });
        }

        Ok(res.first().unwrap().to_owned())
    }

    fn get_by_filter<T: StoreObject>(&self, f: Filter) -> DBResult<Vec<T>> {
        let res = self.get_by_statement(filter(f))?;
        Ok(res)
    }

    fn get_with_max<T: StoreObject>(&self, f: &str) -> DBResult<Option<T>> {
        let res: Vec<T> = self.get_by_statement(sort(f, SortDir::Descend).limit(1))?;
        if res.is_empty() {
            return Ok(None);
        }
        Ok(Some(res.first().unwrap().to_owned()))
    }
}

// pub trait StFuncs {
//     fn get_by_id<T: StoreObject + Clone>(&self, id: usize) -> DBResult<T>;
//     fn get_by_filter<T: StoreObject + Clone>(&self, f: Filter) -> DBResult<Vec<T>>;
//     fn get_with_max<T: StoreObject + Clone>(&self, f: &str) -> DBResult<Option<T>>;
// }

// impl<T: St> StFuncs for T {
//     fn get_by_id<U: StoreObject + Clone>(&self, id: usize) -> DBResult<U> {
//         let res = self.get_by_statement::<U>(filter(eq(&"id", id)))?;
//         if res.is_empty() {
//             return Err(DBError::IsEmpty {
//                 message: format!("no row with id {}", id),
//             });
//         }

//         Ok(res.first().unwrap().to_owned())
//     }

//     fn get_by_filter<U: StoreObject + Clone>(&self, f: Filter) -> DBResult<Vec<U>> {
//         let res = self.get_by_statement(filter(f))?;
//         Ok(res)
//     }

//     fn get_with_max<U: StoreObject + Clone>(&self, f: &str) -> DBResult<Option<U>> {
//         let res: Vec<U> = self.get_by_statement(sort(f, SortDir::Descend).limit(1))?;
//         if res.is_empty() {
//             return Ok(None);
//         }
//         Ok(Some(res.first().unwrap().to_owned()))
//     }
// }

impl<T: Clone> dyn Storage<Item = T> {
    pub fn by_id(&self, id: usize) -> DBResult<T> {
        let res = self.by_statement(filter(eq(&"id", id)))?;
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
    I64(i64),
    F64(f64),
    U8Vec(Vec<u8>),
    Usize(usize),
    DateTime(DateTime<Utc>),
    String(String),
    Bool(bool),
    Null,
}

impl From<i32> for FieldVal {
    fn from(u: i32) -> FieldVal {
        FieldVal::I64(u as i64)
    }
}
impl From<i64> for FieldVal {
    fn from(u: i64) -> FieldVal {
        FieldVal::I64(u)
    }
}
impl From<f64> for FieldVal {
    fn from(u: f64) -> FieldVal {
        FieldVal::F64(u)
    }
}
impl From<&[u8]> for FieldVal {
    fn from(u: &[u8]) -> FieldVal {
        FieldVal::U8Vec(u.into())
    }
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

impl From<FieldVal> for usize {
    fn from(val: FieldVal) -> usize {
        if let FieldVal::Usize(v) = val {
            return v;
        }

        panic!("wrong enum value")
    }
}
impl From<FieldVal> for DateTime<Local> {
    fn from(val: FieldVal) -> DateTime<Local> {
        if let FieldVal::DateTime(v) = val {
            return v.into();
        } else if let FieldVal::I64(v) = val {
            return Utc.timestamp_millis(v).into();
        }

        panic!("wrong enum value")
    }
}
impl From<FieldVal> for DateTime<Utc> {
    fn from(val: FieldVal) -> DateTime<Utc> {
        if let FieldVal::DateTime(v) = val {
            return v;
        } else if let FieldVal::I64(v) = val {
            return Utc.timestamp_millis(v);
        }

        panic!("wrong enum value")
    }
}
impl From<FieldVal> for String {
    fn from(val: FieldVal) -> String {
        if let FieldVal::String(v) = val {
            return v;
        } else if let FieldVal::U8Vec(v) = val {
            return String::from_utf8(v).unwrap();
        }

        panic!("wrong enum value")
    }
}
impl From<FieldVal> for bool {
    fn from(val: FieldVal) -> bool {
        if let FieldVal::Bool(v) = val {
            return v;
        }

        panic!("wrong enum value")
    }
}

impl From<FieldVal> for Option<usize> {
    fn from(val: FieldVal) -> Option<usize> {
        if let FieldVal::Null = val {
            None
        } else {
            Some(val.into())
        }
    }
}
impl From<FieldVal> for Option<String> {
    fn from(val: FieldVal) -> Option<String> {
        if let FieldVal::Null = val {
            None
        } else {
            Some(val.into())
        }
    }
}
impl From<FieldVal> for Option<DateTime<Local>> {
    fn from(val: FieldVal) -> Option<DateTime<Local>> {
        if let FieldVal::Null = val {
            None
        } else {
            Some(val.into())
        }
    }
}
impl From<FieldVal> for Option<DateTime<Utc>> {
    fn from(val: FieldVal) -> Option<DateTime<Utc>> {
        if let FieldVal::Null = val {
            None
        } else {
            Some(val.into())
        }
    }
}

pub trait HistoryStorage {
    fn push_record(&self, r: HistoryRecord) -> DBResult<()>;
    fn get_entity_guid(&self, id: usize, entity_type: &str) -> DBResult<Uuid>;
}

pub trait StoreObject: Clone + Default {
    fn get_field_val(&self, field_name: &str) -> FieldVal;
    fn get_type_name(&self) -> &'static str;
    fn get_fields_list(&self) -> &'static [&'static str];
    fn set_field_val(&mut self, field_name: &str, val: impl Into<FieldVal>);
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
