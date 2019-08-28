use std::convert::TryFrom;
use std::path::Path;
use std::rc::Rc;

use chrono::prelude::*;
use rusqlite::{params, Connection, Result as SQLITEResult, NO_PARAMS};

use core::*;
use orm::errors::*;
use orm::filter::*;
use orm::*;

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
             deleted integer default 0
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

    fn select(&self, where_str: &str) -> SQLITEResult<Vec<Node>> {
        let sql = format!(
            "select
        id,
        parent_id,
        label,
        closed,
        created,
        deleted
            from nodes {}",
            where_str
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
                parent_id = ?3
                where id = ?4",
                    params![node.label, node.closed, parent_id, id],
                )
                .map_err(|s| DBError::wrap(Box::new(s)))?;
            return Ok(node.id);
        };
        self.con
            .execute(
                "insert into nodes (
                        label,
                        parent_id,
                        created) values (?1, ?2, ?3)",
                params![node.label, parent_id, Utc::now()],
            )
            .map_err(|s| DBError::wrap(Box::new(s)))?;
        Ok(usize::try_from(self.con.last_insert_rowid()).unwrap())
    }
    fn all(&self) -> DBResult<Vec<Self::Item>> {
        let res = self
            .select("where deleted = 0")
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
    fn filter(&self, filter: Filter) -> DBResult<Vec<Self::Item>> {
        let res = self
            .select(&format!("where {}", filter.build_where()))
            .map_err(|s| DBError::wrap(Box::new(s)))?;

        Ok(res)
    }
    fn with_max(&self, field: &str) -> DBResult<Option<Self::Item>> {
        let sql = format!(
            "select
        id,
        parent_id,
        label,
        closed,
        created,
        deleted
            from nodes where deleted = 0 order by {} desc limit 1",
            field
        );

        let mut stmt = self
            .con
            .prepare(&sql)
            .map_err(|s| DBError::wrap(Box::new(s)))?;

        let mut rows = stmt
            .query(NO_PARAMS)
            .map_err(|s| DBError::wrap(Box::new(s)))?;

        if let Some(r) = rows.next().map_err(|s| DBError::wrap(Box::new(s)))? {
            let id: isize = r.get(0).map_err(|s| DBError::wrap(Box::new(s)))?;
            let id = usize::try_from(id).unwrap();
            let parent_id: Option<isize> = r.get(1).map_err(|s| DBError::wrap(Box::new(s)))?;
            let parent_id = match parent_id {
                Some(v) => Some(usize::try_from(v).unwrap()),
                None => None,
            };
            return Ok(Some(Node {
                id,
                parent_id,
                label: r.get(2).map_err(|s| DBError::wrap(Box::new(s)))?,
                closed: r.get(3).map_err(|s| DBError::wrap(Box::new(s)))?,
                created: r.get(4).map_err(|s| DBError::wrap(Box::new(s)))?,
                deleted: r.get(5).map_err(|s| DBError::wrap(Box::new(s)))?,
            }));
        };
        Ok(None)
    }
}

pub struct Intervals {
    con: Rc<Connection>,
}

impl Intervals {
    pub fn new(con: Rc<Connection>) -> Intervals {
        Intervals { con }
    }
    fn select(&self, where_str: &str) -> SQLITEResult<Vec<Interval>> {
        let sql = format!(
            "select
        id,
        node_id,
        begin,
        end,
        deleted
            from intervals {}",
            where_str
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
                end = ?3
                where id = ?4",
                    params![node_id, interval.begin, interval.end, id],
                )
                .map_err(|s| DBError::wrap(Box::new(s)))?;
            return Ok(interval.id);
        };
        self.con
            .execute(
                "insert into intervals (node_id, begin, end) 
                values (?1, ?2, ?3)",
                params![node_id, interval.begin, interval.end],
            )
            .map_err(|s| DBError::wrap(Box::new(s)))?;
        Ok(usize::try_from(self.con.last_insert_rowid()).unwrap())
    }
    fn all(&self) -> DBResult<Vec<Self::Item>> {
        let res = self
            .select("where deleted = 0")
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
    fn filter(&self, filter: Filter) -> DBResult<Vec<Self::Item>> {
        let res = self
            .select(&format!("where {}", filter.build_where()))
            .map_err(|s| DBError::wrap(Box::new(s)))?;

        Ok(res)
    }
    fn with_max(&self, field: &str) -> DBResult<Option<Self::Item>> {
        let sql = format!(
            "select
        id,
        node_id,
        begin,
        end,
        deleted
            from intervals where
            deleted = 0 order by {} desc limit 1",
            field
        );

        let mut stmt = self
            .con
            .prepare(&sql)
            .map_err(|s| DBError::wrap(Box::new(s)))?;

        let mut rows = stmt
            .query(NO_PARAMS)
            .map_err(|s| DBError::wrap(Box::new(s)))?;

        if let Some(r) = rows.next().map_err(|s| DBError::wrap(Box::new(s)))? {
            let id: isize = r.get(0).map_err(|s| DBError::wrap(Box::new(s)))?;
            let id = usize::try_from(id).unwrap();
            let node_id: Option<isize> = r.get(1).map_err(|s| DBError::wrap(Box::new(s)))?;
            let node_id = match node_id {
                Some(v) => Some(usize::try_from(v).unwrap()),
                None => None,
            };
            return Ok(Some(Interval {
                id,
                node_id,
                begin: r.get(2).map_err(|s| DBError::wrap(Box::new(s)))?,
                end: r.get(3).map_err(|s| DBError::wrap(Box::new(s)))?,
                deleted: r.get(4).map_err(|s| DBError::wrap(Box::new(s)))?,
            }));
        };
        Ok(None)
    }
}

trait BuildWhere {
    fn build_where(&self) -> String;
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
            CmpOp::Ne(s, v) => format!("{} <> {}", s, v.build_where()),
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
mod tests {
    use super::*;
    use chrono::prelude::Utc;
    use std::convert::TryFrom;
    use std::fs;
    const DATA_DIR: &str = "test_data";

    #[test]
    fn it_creates_db() {
        let r = Path::new(DATA_DIR);
        let p = r.join("test_inits.db");
        if p.exists() {
            fs::remove_file(&p).unwrap();
        }
        let _ = DB::new(&p).unwrap();
        assert!(p.exists());
        fs::remove_file(p).unwrap();
    }
    #[test]
    fn it_saves_node() {
        let r = Path::new(DATA_DIR);
        let p = r.join("test_save_node.db");
        if p.exists() {
            fs::remove_file(&p).unwrap();
        }
        let s = DB::new(&p).unwrap();
        let mut n = Node {
            id: 0,
            label: String::from("test_node"),
            parent_id: None,
            created: Utc::now(),
            closed: false,
            deleted: false,
        };
        let id = s.nodes().save(&n).unwrap();
        let nn = select_node(&s);
        assert_eq!(id, 1);
        assert_eq!(nn.label, n.label);

        n.label = String::from("test_update_node");
        n.id = 1;
        s.nodes().save(&n).unwrap();
        let nn = select_node(&s);
        assert_eq!(nn.id, n.id);
        assert_eq!(nn.label, n.label);
        fs::remove_file(p).unwrap();
    }
    #[test]
    fn it_saves_interval() {
        let r = Path::new(DATA_DIR);
        let p = r.join("test_save_interval.db");
        if p.exists() {
            fs::remove_file(&p).unwrap();
        }
        let s = DB::new(&p).unwrap();
        let mut n = Interval {
            id: 0,
            node_id: Some(1),
            begin: Utc::now(),
            end: Some(Utc::now()),
            deleted: false,
        };
        let id = s.intervals().save(&n).unwrap();
        let nn = select_interval(&s);
        assert_eq!(id, nn.id);
        assert_eq!(n.node_id, nn.node_id);
        assert_eq!(n.begin, nn.begin);
        assert_eq!(n.end, nn.end);
        n.node_id = Some(3);
        n.id = 1;
        s.intervals().save(&n).unwrap();
        let nn = select_interval(&s);
        assert_eq!(n.id, nn.id);
        assert_eq!(n.node_id, nn.node_id);
        assert_eq!(n.begin, nn.begin);
        assert_eq!(n.end, nn.end);
        fs::remove_file(p).unwrap();
    }

    fn select_node(s: &DB) -> Node {
        let mut stmt = s.con.prepare("select id, label from nodes").unwrap();
        let mut q = stmt.query(NO_PARAMS).unwrap();
        let row = q.next().unwrap();
        match row {
            Some(r) => {
                let id: isize = r.get(0).unwrap();
                Node {
                    id: usize::try_from(id).unwrap(),
                    label: r.get(1).unwrap(),
                    parent_id: None,
                    created: Utc::now(),
                    deleted: false,
                    closed: false,
                }
            }
            None => panic!("no rows returned"),
        }
    }

    fn select_interval(s: &DB) -> Interval {
        let mut stmt = s
            .con
            .prepare(
                "select id, node_id,
            begin, end from intervals",
            )
            .unwrap();
        let mut q = stmt.query(NO_PARAMS).unwrap();
        let row = q.next().unwrap();
        match row {
            Some(r) => {
                let id: isize = r.get(0).unwrap();
                let node_id: Option<isize> = r.get(1).unwrap();
                let node_id = match node_id {
                    Some(v) => Some(usize::try_from(v).unwrap()),
                    None => None,
                };
                Interval {
                    id: usize::try_from(id).unwrap(),
                    node_id,
                    begin: r.get(2).unwrap(),
                    end: r.get(3).unwrap(),
                    deleted: false,
                }
            }
            None => panic!("no rows returned"),
        }
    }
}
