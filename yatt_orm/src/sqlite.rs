use std::convert::TryFrom;
use std::path::Path;
use std::rc::Rc;

use rusqlite::{
  types::ValueRef, Connection, Result as SQLITEResult, ToSql,
  NO_PARAMS,
};

use crate::errors::*;
use crate::statement::*;
use crate::*;

#[derive(Debug)]
pub struct DB {
  con: Rc<Connection>,
}

impl DB {
  pub fn new<P: AsRef<Path>>(path: P) -> DBResult<DB> {
    let exists = path.as_ref().exists();
    let con = Connection::open(path)
      .map_err(|s| DBError::wrap(Box::new(s)))?;
    let res = DB { con: Rc::new(con) };
    if !exists {
      res.init().map_err(|s| DBError::wrap(Box::new(s)))?;
    }
    Ok(res)
  }

  fn init(&self) -> SQLITEResult<()> {
    self.con.execute(
      "create table nodes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            label TEXT NOT NULL,
            parent_id INTEGER,
            created INTEGER NOT NULL,
            closed INTEGER DEFAULT 0,
            deleted integer default 0
            )",
      NO_PARAMS,
    )?;
    self.con.execute(
      "create table intervals (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            node_id integer,
             begin integer NOT NULL,
             end integer,
             deleted integer default 0,
             closed integer default 0
             )",
      NO_PARAMS,
    )?;

    Ok(())
  }

  fn query_rows<T: StoreObject>(&self, q: &str) -> DBResult<Vec<T>> {
    let mut q = self
      .con
      .prepare(&q)
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

impl Storage for DB {
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
            FieldVal::U8Vec(v) => Box::new(v.clone()),
            FieldVal::Null => {
              let res: Box<Option<isize>> = Box::new(None);
              res
            }
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
          .map(|(_, v)| format!("{}", v))
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
    Ok(
      self
        .con
        .execute(&q, NO_PARAMS)
        .map_err(|e| DBError::wrap(Box::new(e)))?,
    )
  }
  fn get_by_statement<T: StoreObject>(
    &self,
    s: Statement,
  ) -> DBResult<Vec<T>> {
    let strct = T::default();
    let q = format!(
      "{} {} from {}s {}",
      s.build_select(),
      strct.get_fields_list().join(", "),
      strct.get_type_name(),
      s.build_where()
    );

    self.query_rows(&q)
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
    if self.sorts.is_some() {
      res += " order by ";
      res += &self
        .sorts
        .as_ref()
        .unwrap()
        .iter()
        .map(|s| s.build_where())
        .collect::<Vec<String>>()
        .join(", ")
    }
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
      FieldVal::String(s) => format!("\"{}\"", s.to_string()),
      FieldVal::Bool(b) => (if *b { 1 } else { 0 }).to_string(),
      FieldVal::Null => String::from("null"),
      FieldVal::I64(u) => u.to_string(),
      FieldVal::F64(u) => u.to_string(),
      FieldVal::U8Vec(u) => String::from_utf8(u.clone()).unwrap(),
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
