use std::convert::TryFrom;
use std::path::Path;
use std::rc::Rc;

use rusqlite::{params, Connection, Result as SQLITEResult, NO_PARAMS};

use core::*;
use filter::*;
use orm::*;

#[derive(Debug)]
pub struct DB {
    con: Rc<Connection>,
}

impl DB {
    pub fn new<P: AsRef<Path>>(path: P) -> SQLITEResult<DB> {
        let exists = path.as_ref().exists();
        let con = Connection::open(path)?;
        let res = DB { con: Rc::new(con) };
        if !exists {
            res.init()?;
        }
        Ok(res)
    }

    fn init(&self) -> SQLITEResult<()> {
        self.con.execute(
            "create table nodes
            (id INTEGER PRIMARY KEY AUTOINCREMENT, label TEXT NOT NULL)",
            NO_PARAMS,
        )?;
        self.con.execute(
            "create table intervals 
            (id INTEGER PRIMARY KEY AUTOINCREMENT,
            node_id integer,
             begin integer NOT NULL,
             end integer)",
            NO_PARAMS,
        )?;

        Ok(())
    }
}

impl StorageRoot for DB {
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
}

impl Storage for Nodes {
    type Item = Node;
    fn save(&self, node: &Node) -> DynResult<usize> {
        if node.id > 0 {
            let id = isize::try_from(node.id).unwrap();
            let exec_res = self.con.execute(
                "update nodes
                set label = ?1
                where id = ?2",
                params![node.label, id],
            );
            return match exec_res {
                Ok(_) => Ok(node.id),
                Err(e) => Err(Box::new(e)),
            };
        }

        let exec_res = self
            .con
            .execute("insert into nodes (label) values (?1)", params![node.label]);
        match exec_res {
            Ok(_) => Ok(usize::try_from(self.con.last_insert_rowid()).unwrap()),
            Err(e) => Err(Box::new(e)),
        }
    }
    fn all(&self) -> RecSourceResult<Self::Item> {
        unimplemented!();
    }
    fn remove(&self, id: usize) -> DynResult<()> {
        unimplemented!();
    }
    fn filter(&self, filter: Filter) -> RecSourceResult<Self::Item> {
        unimplemented!();
    }
}

pub struct NodesIter {}

impl Iterator for NodesIter {
    type Item = Node;
    fn next(&mut self) -> Option<Self::Item> {
        unimplemented!()
    }
}

pub struct Intervals {
    con: Rc<Connection>,
}

impl Intervals {
    pub fn new(con: Rc<Connection>) -> Intervals {
        Intervals { con }
    }
}

impl Storage for Intervals {
    type Item = Interval;
    fn save(&self, interval: &Interval) -> DynResult<usize> {
        let node_id = match interval.node_id {
            Some(v) => Some(isize::try_from(v).unwrap()),
            None => None,
        };
        if interval.id > 0 {
            let id = isize::try_from(interval.id).unwrap();
            let exec_res = self.con.execute(
                "update intervals
                set node_id = ?1,
                begin = ?2,
                end = ?3
                where id = ?4",
                params![node_id, interval.begin, interval.end, id],
            );
            return match exec_res {
                Ok(_) => Ok(interval.id),
                Err(e) => Err(Box::new(e)),
            };
        }
        let exec_res = self.con.execute(
            "insert into intervals (node_id, begin, end) 
                values (?1, ?2, ?3)",
            params![node_id, interval.begin, interval.end],
        );
        match exec_res {
            Ok(_) => Ok(usize::try_from(self.con.last_insert_rowid()).unwrap()),
            Err(e) => Err(Box::new(e)),
        }
    }
    fn all(&self) -> RecSourceResult<Self::Item> {
        unimplemented!()
    }
    fn remove(&self, id: usize) -> DynResult<()> {
        unimplemented!();
    }
    fn filter(&self, filter: Filter) -> RecSourceResult<Self::Item> {
        unimplemented!();
    }
}

trait BuildWhere {
    fn build_where(&self) -> String;
}

impl BuildWhere for CmpVal {
    fn build_where(&self) -> String {
        unimplemented!()
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
                }
            }
            None => panic!("no rows returned"),
        }
    }
}
