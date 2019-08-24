use chrono::prelude::*;
use orm::{Storage, Identifiers, BoxStorage};

pub trait StorageRoot {
    fn nodes(&self) -> BoxStorage<Node>;
    fn intervals(&self) -> BoxStorage<Interval>;
}

#[derive(Debug, Identifiers)]
pub struct Node {
    pub id: usize,
    pub label: String,
}

#[derive(Debug, Identifiers)]
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
        assert_eq!(Node::id_n(), String::from("id"));
        assert_eq!(Node::label_n(), String::from("label"));
    }
}
