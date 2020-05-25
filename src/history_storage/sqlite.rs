use std::convert::TryFrom;
use std::path::Path;
use std::rc::Rc;

use rusqlite::{
  params, Connection, Result as SQLITEResult, NO_PARAMS,
};
use uuid::Uuid;

use yatt_orm::{DBError, DBResult, HistoryRecord, HistoryStorage};

#[derive(Debug)]
pub(crate) struct DB {
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
      "create table history_records (
            date INTEGER NOT NULL,
            uuid TEXT NOT NULL,
            record_type TEXT NOT NULL,
            entyty_type INTEGER NOT NULL,
            entity_id INTEGER NOT NULL
            )",
      NO_PARAMS,
    )?;

    Ok(())
  }
}

impl HistoryStorage for DB {
  fn push_record(&self, r: HistoryRecord) -> DBResult<()> {
    self
      .con
      .execute(
        "insert into history_records (
                date,
                uuid,
                record_type,
                entity_type,
                entity_id
        ) values (?1, ?2, ?3, ?4, ?5)",
        params![
          r.date,
          r.uuid.to_string(),
          isize::from(r.record_type),
          r.entity_type,
          isize::try_from(r.entity_id).unwrap(),
        ],
      )
      .map_err(|s| DBError::wrap(Box::new(s)))?;

    Ok(())
  }
  fn get_entity_guid(
    &self,
    id: usize,
    entity_type: &str,
  ) -> DBResult<Uuid> {
    match self
      .con
      .prepare(
        "select uuid from history_records
                where entity_id = ?1 and entity_type = ?2 limit 1",
      )
      .map_err(|s| DBError::wrap(Box::new(s)))?
      .query(params![isize::try_from(id).unwrap(), entity_type])
      .map_err(|s| DBError::wrap(Box::new(s)))?
      .next()
      .map_err(|s| DBError::wrap(Box::new(s)))?
    {
      Some(row) => {
        let str_row: String =
          row.get(0).map_err(|s| DBError::wrap(Box::new(s)))?;
        Ok(
          Uuid::parse_str(str_row.as_str())
            .map_err(|s| DBError::wrap(Box::new(s)))?,
        )
      }
      None => Err(DBError::IsEmpty {
        message: format!(
          "no entity found for id={} and entity_type={}",
          id, entity_type
        ),
      }),
    }
  }
}
