use chrono::prelude::*;
use custom_error::custom_error;

pub struct Node {
    pub id: usize,
    pub label: String,
}

pub struct Interval {
    pub node_id: usize,
    pub begin: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

custom_error!{
   /// This is the documentation for my error type
   pub StorageError{msg: String} = "message: '{msg}'"
}


pub trait IntervalsStorage {
    fn save(&self, interval: &Interval) -> Result<usize, StorageError>;
}

pub trait NodesStorage {
    fn save(&self, node: &Node) -> Result<usize, StorageError>;
}


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_works() {
        let n = Node {
            id: 0,
            label: String::from("azaza"),
        };
        assert_eq!(n.label.as_str(), "azaza");
        let i = Interval {
            node_id: 123,
            begin: Utc::now(),
            end: Utc::now(),
        };
        
        assert_eq!(i.begin.date(), Utc::now().date())
    }
}
