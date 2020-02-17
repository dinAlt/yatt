use std::convert::TryFrom;
use std::path::Path;
use std::rc::Rc;

use chrono::prelude::*;
use rusqlite::{params, Connection, Result as SQLITEResult, NO_PARAMS};

use crate::core::*;
use yatt_orm::errors::*;
use yatt_orm::statement::*;
use yatt_orm::*;

#[derive(Debug)]
pub struct DB {
    con: Rc<Connection>,
}

impl DB {
    pub fn new<P: AsRef<Path>>(path: P) -> DBResult<DB> {
        let exists = path.as_ref().exists();
        let con = Connection::open(path).map_err(|s| DBError::wrap(Box::new(s)))?;
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
}

impl DBRoot for DB {
    fn nodes(&self) -> BoxStorage<Node> {
        Box::new(Nodes::new(Rc::clone(&self.con)))
    }
    fn intervals(&self) -> BoxStorage<Interval> {
        Box::new(Intervals::new(Rc::clone(&self.con)))
    }
}

pub struct Nodes {
    con: Rc<Connection>,
}

impl Nodes {
    pub fn new(con: Rc<Connection>) -> Nodes {
        Nodes { con }
    }

    fn select(&self, select_str: &str, where_str: &str) -> SQLITEResult<Vec<Node>> {
        let sql = format!(
            "{}
        id,
        parent_id,
        label,
        closed,
        created,
        deleted
            from nodes {}",
            select_str, where_str
        );

        let mut stmt = self.con.prepare(&sql)?;
        let mut rows = stmt.query(NO_PARAMS)?;
        let mut res = Vec::new();
        while let Some(row) = rows.next()? {
            let id: isize = row.get(0)?;
            let id = usize::try_from(id).unwrap();
            let parent_id: Option<isize> = row.get(1)?;
            let parent_id = match parent_id {
                Some(v) => Some(usize::try_from(v).unwrap()),
                None => None,
            };
            res.push(Node {
                id,
                parent_id,
                label: row.get(2)?,
                closed: row.get(3)?,
                created: row.get(4)?,
                deleted: row.get(5)?,
            })
        }
        SQLITEResult::Ok(res)
    }
}

impl Storage for Nodes {
    type Item = Node;
    fn save(&self, node: &Node) -> DBResult<usize> {
        let parent_id = node.parent_id.map(|r| isize::try_from(r).unwrap());

        if node.id > 0 {
            let id = isize::try_from(node.id).unwrap();

            self.con
                .execute(
                    "update nodes
                set label = ?1,
                closed = ?2,
                parent_id = ?3,
                deleted = ?4
                where id = ?5",
                    params![node.label, node.closed, parent_id, id, node.deleted],
                )
                .map_err(|s| DBError::wrap(Box::new(s)))?;
            return Ok(node.id);
        };
        self.con
            .execute(
                "insert into nodes (
                        label,
                        parent_id,
                        created, 
                        deleted) values (?1, ?2, ?3, ?4)",
                params![node.label, parent_id, Utc::now(), node.deleted],
            )
            .map_err(|s| DBError::wrap(Box::new(s)))?;
        Ok(usize::try_from(self.con.last_insert_rowid()).unwrap())
    }
    fn all(&self) -> DBResult<Vec<Self::Item>> {
        let res = self
            .select("select", "where deleted = 0")
            .map_err(|s| DBError::wrap(Box::new(s)))?;

        Ok(res)
    }
    fn remove(&self, id: usize) -> DBResult<()> {
        let id = isize::try_from(id).unwrap();
        self.con
            .execute("update nodes set deleted = 1 where id = ?1", params![id])
            .map_err(|s| DBError::wrap(Box::new(s)))?;

        Ok(())
    }
    fn by_statement(&self, statement: Statement) -> DBResult<Vec<Self::Item>> {
        let res = self
            .select(&statement.build_select(), &statement.build_where())
            .map_err(|s| DBError::wrap(Box::new(s)))?;

        Ok(res)
    }
}

pub struct Intervals {
    con: Rc<Connection>,
}

impl Intervals {
    pub fn new(con: Rc<Connection>) -> Intervals {
        Intervals { con }
    }
    fn select(&self, select_str: &str, where_str: &str) -> SQLITEResult<Vec<Interval>> {
        let sql = format!(
            "{}
        id,
        node_id,
        begin,
        end,
        deleted,
        closed
            from intervals {}",
            select_str, where_str
        );

        let mut stmt = self.con.prepare(&sql)?;
        let mut rows = stmt.query(NO_PARAMS)?;
        let mut res = Vec::new();
        while let Some(row) = rows.next()? {
            let id: isize = row.get(0)?;
            let id = usize::try_from(id).unwrap();
            let node_id: Option<isize> = row.get(1)?;
            let node_id = match node_id {
                Some(v) => Some(usize::try_from(v).unwrap()),
                None => None,
            };
            res.push(Interval {
                id,
                node_id,
                begin: row.get(2)?,
                end: row.get(3)?,
                deleted: row.get(4)?,
                closed: row.get(5)?,
            })
        }
        SQLITEResult::Ok(res)
    }
}

impl Storage for Intervals {
    type Item = Interval;
    fn save(&self, interval: &Interval) -> DBResult<usize> {
        let node_id = interval.node_id.map(|r| isize::try_from(r).unwrap());

        if interval.id > 0 {
            let id = isize::try_from(interval.id).unwrap();
            self.con
                .execute(
                    "update intervals
                set node_id = ?1,
                begin = ?2,
                end = ?3,
                deleted = ?4,
                closed = ?5
                where id = ?6",
                    params![
                        node_id,
                        interval.begin,
                        interval.end,
                        interval.deleted,
                        interval.closed,
                        id
                    ],
                )
                .map_err(|s| DBError::wrap(Box::new(s)))?;
            return Ok(interval.id);
        };
        self.con
            .execute(
                "insert into intervals (node_id, begin, end, deleted, closed) 
                values (?1, ?2, ?3, ?4, ?5)",
                params![
                    node_id,
                    interval.begin,
                    interval.end,
                    interval.deleted,
                    interval.closed
                ],
            )
            .map_err(|s| DBError::wrap(Box::new(s)))?;
        Ok(usize::try_from(self.con.last_insert_rowid()).unwrap())
    }
    fn all(&self) -> DBResult<Vec<Self::Item>> {
        let res = self
            .select("select", "where deleted = 0")
            .map_err(|s| DBError::wrap(Box::new(s)))?;

        Ok(res)
    }
    fn remove(&self, id: usize) -> DBResult<()> {
        let id = isize::try_from(id).unwrap();
        self.con
            .execute(
                "update intervals set deleted = 1 where id = ?1",
                params![id],
            )
            .map_err(|s| DBError::wrap(Box::new(s)))?;

        Ok(())
    }
    fn by_statement(&self, statement: Statement) -> DBResult<Vec<Self::Item>> {
        let res = self
            .select(&statement.build_select(), &statement.build_where())
            .map_err(|s| DBError::wrap(Box::new(s)))?;

        Ok(res)
    }
}

trait BuildSelect {
    fn build_select(&self) -> String;
}

impl BuildSelect for Statement {
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

impl BuildWhere for Statement {
    fn build_where(&self) -> String {
        let mut res = String::new();
        if self.filter.is_some() {
            res += &format!("where {}", self.filter.as_ref().unwrap().build_where());
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
impl BuildWhere for CmpVal {
    fn build_where(&self) -> String {
        match self {
            CmpVal::Usize(u) => u.to_string(),
            CmpVal::DateTime(d) => format!("\"{}\"", d.to_rfc3339()),
            CmpVal::String(s) => format!("\"{}\"", s.to_string()),
            CmpVal::Null => String::from("null"),
        }
    }
}
impl BuildWhere for CmpOp {
    fn build_where(&self) -> String {
        match self {
            CmpOp::Eq(s, v) => {
                let sign = if let CmpVal::Null = v { "is" } else { "=" };
                format!("{} {} {}", s, sign, v.build_where())
            }
            CmpOp::Ne(s, v) => {
                let sign = if let CmpVal::Null = v { "is not" } else { "<>" };
                format!("{} {} {}", s, sign, v.build_where())
            }
            CmpOp::Gt(s, v) => format!("{} > {}", s, v.build_where()),
            CmpOp::Lt(s, v) => format!("{} < {}", s, v.build_where()),
        }
    }
}
impl BuildWhere for Filter {
    fn build_where(&self) -> String {
        match self {
            Filter::LogOp(lo) => lo.build_where(),
            Filter::CmpOp(co) => co.build_where(),
        }
    }
}
impl BuildWhere for LogOp {
    fn build_where(&self) -> String {
        match self {
            LogOp::Or(f1, f2) => format!("({} or {})", f1.build_where(), f2.build_where()),
            LogOp::And(f1, f2) => format!("({} and {})", f1.build_where(), f2.build_where()),
            LogOp::Not(f) => format!("(not {})", f.build_where()),
        }
    }
}

#[cfg(test)]
mod tests {}
