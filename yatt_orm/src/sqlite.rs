use super::FieldVal;
use std::convert::TryFrom;
use std::path::Path;

pub use rusqlite::{
  types::ValueRef, Connection, Result as SQLITEResult,
  Statement as SQLITEStatement, ToSql, Transaction, NO_PARAMS,
};

use crate::errors::*;
use crate::statement::*;
use crate::*;

#[derive(Debug)]
enum DBRunner<'a> {
  Connection(Connection),
  Transaction(Transaction<'a>),
}

impl DBRunner<'_> {
  fn prepare(&self, sql: &str) -> SQLITEResult<SQLITEStatement<'_>> {
    match self {
      DBRunner::Connection(c) => c.prepare(sql),
      DBRunner::Transaction(t) => t.prepare(sql),
    }
  }
  fn execute<P>(&self, sql: &str, params: P) -> SQLITEResult<usize>
  where
    P: IntoIterator,
    P::Item: ToSql,
  {
    match self {
      DBRunner::Connection(c) => c.execute(sql, params),
      DBRunner::Transaction(t) => t.execute(sql, params),
    }
  }
  fn last_insert_rowid(&self) -> i64 {
    match self {
      DBRunner::Connection(c) => c.last_insert_rowid(),
      DBRunner::Transaction(t) => t.last_insert_rowid(),
    }
  }
  fn transaction(&mut self) -> SQLITEResult<Transaction<'_>> {
    match self {
      DBRunner::Connection(c) => c.transaction(),
      DBRunner::Transaction(_) => {
        panic!("trasaction method called on transaction")
      }
    }
  }
  fn commit(self) -> SQLITEResult<()> {
    match self {
      DBRunner::Connection(_) => panic!("call commit on connection"),
      DBRunner::Transaction(t) => t.commit(),
    }
  }
}

#[derive(Debug)]
pub struct DB<'a> {
  con: DBRunner<'a>,
}

impl<'a> DB<'a> {
  pub fn new<P, F>(path: P, init: F) -> DBResult<DB<'a>>
  where
    P: AsRef<Path>,
    F: FnOnce(&Connection) -> SQLITEResult<()>,
  {
    let con = Connection::open(path)
      .map_err(|s| DBError::wrap(Box::new(s)))?;
    init(&con).map_err(|s| DBError::wrap(Box::new(s)))?;
    let res = DB {
      con: DBRunner::Connection(con),
    };
    Ok(res)
  }

  pub fn transaction(&mut self) -> DBResult<DB<'_>> {
    let tx = self
      .con
      .transaction()
      .map_err(|e| DBError::wrap(Box::new(e)))?;
    Ok(DB {
      con: DBRunner::Transaction(tx),
    })
  }

  pub fn commit(self) -> DBResult<()> {
    self.con.commit().map_err(|e| DBError::wrap(Box::new(e)))?;
    Ok(())
  }

  fn query_rows<T: StoreObject>(&self, q: &str) -> DBResult<Vec<T>> {
    let mut q = self
      .con
      .prepare(q)
      .map_err(|e| DBError::wrap(Box::new(e)))?;

    let mut rows =
      q.query(NO_PARAMS).map_err(|e| DBError::wrap(Box::new(e)))?;
    let mut res = Vec::new();
    while let Some(r) =
      rows.next().map_err(|e| DBError::wrap(Box::new(e)))?
    {
      let mut strct = T::default();
      for (n, fld_name) in strct.get_fields_list().iter().enumerate()
      {
        let v = r.get_raw(n);
        let v: FieldVal = match v {
          ValueRef::Integer(vv) => vv.into(),
          ValueRef::Null => FieldVal::Null,
          ValueRef::Real(vv) => vv.into(),
          ValueRef::Text(vv) => vv.into(),
          _ => unreachable!(),
        };
        strct.set_field_val(fld_name, v)?;
      }
      res.push(strct);
    }

    Ok(res)
  }
}

impl Storage for DB<'_> {
  fn save(&self, item: &impl StoreObject) -> DBResult<usize> {
    let id = if let FieldVal::Usize(id) = item.get_field_val("id") {
      id
    } else {
      return Err(DBError::Unexpected {
        message: "field id has unexpected type".into(),
      });
    };

    let field_list = item.get_fields_list();

    let id_idx = field_list.iter().position(|&v| v == "id").unwrap();
    let non_id_fields = field_list
      .iter()
      .enumerate()
      .filter_map(|(n, &v)| if n == id_idx { None } else { Some(v) })
      .enumerate();
    let mut params: Vec<Box<dyn ToSql>> = field_list
      .iter()
      .enumerate()
      .filter_map(|(n, &v)| {
        if n == id_idx {
          None
        } else {
          let fld = item.get_field_val(v);
          let res: Box<dyn ToSql> = match fld {
            FieldVal::Usize(v) => {
              Box::new(isize::try_from(v).unwrap())
            }
            FieldVal::Bool(v) => Box::new(v),
            FieldVal::String(v) => Box::new(v),
            FieldVal::DateTime(v) => Box::new(v),
            FieldVal::F64(v) => Box::new(v),
            FieldVal::I64(v) => Box::new(v),
            FieldVal::U8Vec(v) => Box::new(v),
            FieldVal::Null => {
              let res: Box<Option<isize>> = Box::new(None);
              res
            }
            FieldVal::FieldName(f) => Box::new(f),
          };

          Some(res)
        }
      })
      .collect();

    let sql = if id > 0 {
      let sql = format!(
        "update {}s set {} where id = ?{} ",
        item.get_type_name(),
        non_id_fields
          .map(|(n, v)| format!("{} = ?{}", v, n + 1))
          .collect::<Vec<String>>()
          .join(", "),
        field_list.len()
      );

      params.push(Box::new(isize::try_from(id).unwrap()));

      sql
    } else {
      format!(
        "insert into {}s ({}) values ({})",
        item.get_type_name(),
        non_id_fields
          .map(|(_, v)| v.to_string())
          .collect::<Vec<String>>()
          .join(", "),
        (1..field_list.len())
          .map(|v| format!("?{}", v))
          .collect::<Vec<String>>()
          .join(", ")
      )
    };
    self
      .con
      .execute(&sql, params)
      .map_err(|e| DBError::wrap(Box::new(e)))?;

    if id > 0 {
      Ok(id)
    } else {
      Ok(usize::try_from(self.con.last_insert_rowid()).unwrap())
    }
  }
  fn get_all<T: StoreObject>(&self) -> DBResult<Vec<T>> {
    let strct = T::default();
    let q = format!(
      "select {} from {}s",
      strct.get_fields_list().join(", "),
      strct.get_type_name()
    );

    self.query_rows(&q)
  }
  fn remove_by_filter<T: StoreObject>(
    &self,
    filter: Filter,
  ) -> DBResult<usize> {
    let strct = T::default();
    let q = format!(
      "update {}s set deleted = 1 where {}",
      strct.get_type_name(),
      filter.build_where()
    );
    self
      .con
      .execute(&q, NO_PARAMS)
      .map_err(|e| DBError::wrap(Box::new(e)))
  }
  fn get_by_statement<T: StoreObject>(
    &self,
    s: Statement,
  ) -> DBResult<Vec<T>> {
    let strct = T::default();
    let from = format!("{}s", strct.get_type_name());
    let s = s.from(&from).alias("t");
    let q = s.build_select_statement(strct.get_fields_list());
    self.query_rows(&q)
  }
}

trait BuildSelectStatement {
  fn build_select_statement(&self, fields: &[&str]) -> String;
}

impl BuildSelectStatement for Statement<'_> {
  fn build_select_statement(&self, fields: &[&str]) -> String {
    let table = self.from.unwrap();
    let alias = if let Some(alias) = self.alias {
      alias
    } else {
      table
    };
    let aliased_flds: String = fields
      .iter()
      .map(|f| format!("{}.{}", &alias, &f))
      .collect::<Vec<String>>()
      .join(", ");
    let unaliased_flds: String = fields
      .iter()
      .map(|f| format!("{}.{}", &table, &f))
      .collect::<Vec<String>>()
      .join(", ");
    let select = format!(
      "{} {} {} {}",
      self.build_select(),
      &aliased_flds,
      self.build_from(),
      self.build_where(),
    );

    let select = if let Some(recursive_on) = self.recursive_on {
      format!(
        "with recursive rec({}) as ({}
      	 union select {} from {}, rec where {}.id = rec.{})
      	 select * from rec {} {}",
        fields.join(", "),
        select,
        &unaliased_flds,
        &table,
        &table,
        recursive_on,
        self.build_order(),
        self.build_limit_offset(),
      )
    } else {
      format!(
        "{} {} {}",
        select,
        self.build_order(),
        self.build_limit_offset(),
      )
    };

    select
  }
}

trait BuildSelect {
  fn build_select(&self) -> String;
}

impl BuildSelect for Statement<'_> {
  fn build_select(&self) -> String {
    if self.distinct {
      return "select distinct".to_string();
    }
    "select".to_string()
  }
}

trait BuildFrom {
  fn build_from(&self) -> String;
}

impl BuildFrom for Statement<'_> {
  fn build_from(&self) -> String {
    if let Some(alias) = self.alias {
      format!("from {} as {}", self.from.unwrap(), alias)
    } else {
      format!("from {}", self.from.unwrap())
    }
  }
}

trait BuildWhere {
  fn build_where(&self) -> String;
}

impl BuildWhere for Statement<'_> {
  fn build_where(&self) -> String {
    let mut res = String::new();
    if self.filter.is_some() {
      res += &format!(
        "where {}",
        self.filter.as_ref().unwrap().build_where()
      );
    }

    res
  }
}

trait BuildOrder {
  fn build_order(&self) -> String;
}

impl BuildOrder for Statement<'_> {
  fn build_order(&self) -> String {
    if self.sorts.is_some() {
      format!(
        "order by {}",
        self
          .sorts
          .as_ref()
          .unwrap()
          .iter()
          .map(|s| s.build_where())
          .collect::<Vec<String>>()
          .join(", ")
      )
    } else {
      "".to_string()
    }
  }
}

trait BuildLimitOffset {
  fn build_limit_offset(&self) -> String;
}

impl BuildLimitOffset for Statement<'_> {
  fn build_limit_offset(&self) -> String {
    let mut res = String::new();

    if self.limit.is_some() {
      res += &format!(" limit {}", self.limit.unwrap());
    }
    if self.offset.is_some() {
      res += &format!(" offset {}", self.offset.unwrap());
    }

    res
  }
}

impl BuildWhere for SortItem {
  fn build_where(&self) -> String {
    format!("{} {}", self.0, self.1.build_where())
  }
}
impl BuildWhere for SortDir {
  fn build_where(&self) -> String {
    match self {
      SortDir::Ascend => "asc".to_string(),
      SortDir::Descend => "desc".to_string(),
    }
  }
}
impl BuildWhere for FieldVal {
  fn build_where(&self) -> String {
    match self {
      FieldVal::Usize(u) => u.to_string(),
      FieldVal::DateTime(d) => format!("\"{}\"", d.to_rfc3339()),
      FieldVal::String(s) => format!("\"{}\"", s),
      FieldVal::Bool(b) => (if *b { 1 } else { 0 }).to_string(),
      FieldVal::Null => String::from("null"),
      FieldVal::I64(u) => u.to_string(),
      FieldVal::F64(u) => u.to_string(),
      FieldVal::U8Vec(u) => String::from_utf8(u.clone()).unwrap(),
      FieldVal::FieldName(s) => format!("t.{}", s),
    }
  }
}
impl<'a> BuildWhere for CmpOp<'a> {
  fn build_where(&self) -> String {
    match self {
      CmpOp::Eq(s, v) => {
        let sign = if let FieldVal::Null = v { "is" } else { "=" };
        format!("{} {} {}", s, sign, v.build_where())
      }
      CmpOp::Ne(s, v) => {
        let sign = if let FieldVal::Null = v {
          "is not"
        } else {
          "<>"
        };
        format!("{} {} {}", s, sign, v.build_where())
      }
      CmpOp::Gt(s, v) => format!("{} > {}", s, v.build_where()),
      CmpOp::Lt(s, v) => format!("{} < {}", s, v.build_where()),
    }
  }
}
impl BuildWhere for Filter<'_> {
  fn build_where(&self) -> String {
    match self {
      Filter::LogOp(lo) => lo.build_where(),
      Filter::CmpOp(co) => co.build_where(),
      Filter::Exists(ex) => {
        format!("exists ({})", ex.build_select_statement(&["id"]))
      }
      Filter::Includes(field, val) => {
        format!(
          "{} like '%{}%'",
          field,
          String::try_from(val.clone()).expect(
            "Filter::Includes expects value to be of type String"
          )
        )
      }
    }
  }
}
impl BuildWhere for LogOp<'_> {
  fn build_where(&self) -> String {
    match self {
      LogOp::Or(f1, f2) => {
        format!("({} or {})", f1.build_where(), f2.build_where())
      }
      LogOp::And(f1, f2) => {
        format!("({} and {})", f1.build_where(), f2.build_where())
      }
      LogOp::Not(f) => format!("(not {})", f.build_where()),
    }
  }
}
