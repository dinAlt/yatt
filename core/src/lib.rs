use chrono::prelude::*;
use orm::{Storage, Identifiers};

pub trait StorageRoot {
    fn nodes(&self) -> Box<dyn Storage<Item = Node>>;
    fn intervals(&self) -> Box<dyn Storage<Item = Interval>>;
}

#[derive(Debug, Identifiers)]
pub struct Node {
    pub id: usize,
    pub label: String,
}

#[derive(Debug)]
pub struct Interval {
    pub id: usize,
    pub node_id: Option<usize>,
    pub begin: DateTime<Utc>,
    pub end: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_works() {
        assert_eq!(Node::idents().id(), String::from("id"));
        assert_eq!(Node::idents().label(), String::from("label"));
    }
}
