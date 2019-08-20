use core;
use rusqlite::{Connection, Error, Result as SQLITEResult, NO_PARAMS};
use std::path::Path;

pub struct Factory {
    con: Connection,
}

impl Factory {
    pub fn new<P: AsRef<Path>>(path: P) -> SQLITEResult<Factory> {
        let con = Connection::open(path)?;
        Ok(Factory { con: con })
    }
    pub fn nodes(&self) -> Nodes {
        Nodes::new(&self.con)
    }
    pub fn intervals(&self) -> Intervals {
        Intervals::new(&self.con)
    }
}

pub struct Nodes<'a> {
    con: &'a Connection,
}

impl<'a> Nodes<'a> {
    pub fn new(con: &'a Connection) -> Nodes<'a> {
        Nodes { con: con }
    }
}

impl<'a> core::NodesStorage for Nodes<'a> {
    fn save(&self, node: &core::Node) -> Result<usize, core::StorageError> {
        unimplemented!()
    }
}

pub struct Intervals<'a> {
    con: &'a Connection,
}

impl<'a> Intervals<'a> {
    pub fn new(con: &'a Connection) -> Intervals<'a> {
        Intervals { con: con }
    }
}

impl<'a> core::IntervalsStorage for Intervals<'a> {
    fn save(&self, interval: &core::Interval) -> Result<usize, core::StorageError> {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::{Node, NodesStorage};
    #[test]
    fn it_works() {
        let con = Connection::open("test.db").unwrap();
        let nodes = Nodes::new(&con);
        let node = Node {
            id: 1,
            label: String::from("test"),
        };
        nodes.save(&node).unwrap();
        assert_eq!(2 + 2, 4);
    }
}
