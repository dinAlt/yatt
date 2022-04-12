pub mod errors;
pub mod sqlite;
pub mod statement;

use chrono::prelude::*;
use chrono::{DateTime, Utc};
use core::convert::TryFrom;
use std::convert::TryInto;
use uuid::Uuid;

pub use errors::*;
pub use yatt_orm_derive::*;

use statement::*;

pub trait Storage {
  fn save(&self, item: &impl StoreObject) -> DBResult<usize>
  where
    Self: Sized;
  fn get_all<T: StoreObject>(&self) -> DBResult<Vec<T>>
  where
    Self: Sized;
  fn remove_by_filter<T: StoreObject>(
    &self,
    filter: Filter,
  ) -> DBResult<usize>
  where
    Self: Sized;
  fn get_by_statement<T: StoreObject>(
    &self,
    s: Statement,
  ) -> DBResult<Vec<T>>
  where
    Self: Sized;

  fn get_by_id<T: StoreObject>(&self, id: usize) -> DBResult<T>
  where
    Self: Sized,
  {
    let res = self.get_by_statement::<T>(filter(eq("id", id)))?;
    if res.is_empty() {
      return Err(DBError::IsEmpty {
        message: format!("no row with id {}", id),
      });
    }

    Ok(res.first().unwrap().to_owned())
  }

  fn get_by_filter<T: StoreObject>(
    &self,
    f: Filter,
  ) -> DBResult<Vec<T>>
  where
    Self: Sized,
  {
    let res = self.get_by_statement(filter(f))?;
    Ok(res)
  }

  fn get_with_max<T: StoreObject>(
    &self,
    f: &str,
  ) -> DBResult<Option<T>>
  where
    Self: Sized,
  {
    let res: Vec<T> =
      self.get_by_statement(sort(f, SortDir::Descend).limit(1))?;
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
  FieldName(String),
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
    val.clone()
  }
}
impl From<bool> for FieldVal {
  fn from(val: bool) -> FieldVal {
    FieldVal::Bool(val)
  }
}
impl From<&bool> for FieldVal {
  fn from(val: &bool) -> FieldVal {
    FieldVal::Bool(*val)
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

impl TryFrom<FieldVal> for usize {
  type Error = DBError;

  fn try_from(val: FieldVal) -> Result<Self, Self::Error> {
    match val {
      FieldVal::Usize(v) => Ok(v),
      FieldVal::I64(v) => {
        Ok(v.try_into().map_err(|e| DBError::wrap(Box::new(e)))?)
      }
      _ => Err(DBError::Convert {
        message: "wrong enum value usize".into(),
      }),
    }
  }
}
impl TryFrom<FieldVal> for DateTime<Local> {
  type Error = DBError;

  fn try_from(val: FieldVal) -> Result<Self, Self::Error> {
    match val {
      FieldVal::DateTime(v) => Ok(v.into()),
      FieldVal::I64(v) => Ok(Utc.timestamp_millis(v).into()),
      FieldVal::U8Vec(v) => {
        let strd = String::from_utf8(v)
          .map_err(|e| DBError::wrap(Box::new(e)))?;
        let dt = DateTime::parse_from_rfc3339(&strd)
          .map_err(|e| DBError::wrap(Box::new(e)))?;
        Ok(dt.with_timezone(&Local))
      }
      _ => Err(DBError::Convert {
        message: "wrong enum value for DateTime<Local>".into(),
      }),
    }
  }
}
impl TryFrom<FieldVal> for DateTime<Utc> {
  type Error = DBError;

  fn try_from(val: FieldVal) -> Result<Self, Self::Error> {
    match val {
      FieldVal::DateTime(v) => Ok(v),
      FieldVal::I64(v) => Ok(Utc.timestamp_millis(v)),
      FieldVal::U8Vec(v) => {
        let strd = String::from_utf8(v)
          .map_err(|e| DBError::wrap(Box::new(e)))?;
        let dt = DateTime::parse_from_rfc3339(&strd).unwrap();
        Ok(dt.with_timezone(&Utc))
      }
      _ => Err(DBError::Convert {
        message: "wrong enum value for DateTime<Utc>".into(),
      }),
    }
  }
}
impl TryFrom<FieldVal> for String {
  type Error = DBError;

  fn try_from(val: FieldVal) -> Result<Self, Self::Error> {
    match val {
      FieldVal::String(v) => Ok(v),
      FieldVal::U8Vec(v) => Ok(
        String::from_utf8(v)
          .map_err(|e| DBError::wrap(Box::new(e)))?,
      ),
      _ => Err(DBError::Convert {
        message: "wrong enum value for String".into(),
      }),
    }
  }
}
impl TryFrom<FieldVal> for bool {
  type Error = DBError;

  fn try_from(val: FieldVal) -> Result<Self, Self::Error> {
    match val {
      FieldVal::Bool(v) => Ok(v),
      FieldVal::I64(v) => Ok(v > 0),
      _ => Err(DBError::Convert {
        message: "wrong enum value for bool".into(),
      }),
    }
  }
}

impl TryFrom<FieldVal> for Option<usize> {
  type Error = DBError;

  fn try_from(val: FieldVal) -> Result<Self, Self::Error> {
    if let FieldVal::Null = val {
      Ok(None)
    } else {
      Ok(Some(
        val.try_into().map_err(|e| DBError::wrap(Box::new(e)))?,
      ))
    }
  }
}
impl TryFrom<FieldVal> for Option<String> {
  type Error = DBError;

  fn try_from(val: FieldVal) -> Result<Self, Self::Error> {
    if let FieldVal::Null = val {
      Ok(None)
    } else {
      Ok(Some(
        val.try_into().map_err(|e| DBError::wrap(Box::new(e)))?,
      ))
    }
  }
}
impl TryFrom<FieldVal> for Option<DateTime<Local>> {
  type Error = DBError;

  fn try_from(val: FieldVal) -> Result<Self, Self::Error> {
    if let FieldVal::Null = val {
      Ok(None)
    } else {
      Ok(Some(
        val.try_into().map_err(|e| DBError::wrap(Box::new(e)))?,
      ))
    }
  }
}
impl TryFrom<FieldVal> for Option<DateTime<Utc>> {
  type Error = DBError;

  fn try_from(val: FieldVal) -> Result<Self, Self::Error> {
    if let FieldVal::Null = val {
      Ok(None)
    } else {
      Ok(Some(
        val.try_into().map_err(|e| DBError::wrap(Box::new(e)))?,
      ))
    }
  }
}

pub trait HistoryStorage {
  fn push_record(&self, r: HistoryRecord) -> DBResult<()>;
  fn get_entity_guid(
    &self,
    id: usize,
    entity_type: &str,
  ) -> DBResult<Uuid>;
}

pub trait StoreObject: Clone + Default {
  fn get_field_val(&self, field_name: &str) -> FieldVal;
  fn get_type_name(&self) -> &'static str;
  fn get_fields_list(&self) -> &'static [&'static str];
  fn set_field_val(
    &mut self,
    field_name: &str,
    val: impl Into<FieldVal>,
  ) -> DBResult<()>;
}
