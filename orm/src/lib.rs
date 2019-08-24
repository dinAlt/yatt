pub mod filter;
pub use orm_derive::*;

use std::error::Error;
use filter::*;

#[derive(Debug)]
pub enum NodeFieldNames {
    Id,
    NodeId,
    Begin,
    End,
}

impl ToString for NodeFieldNames {
    fn to_string(&self) -> String {
        match self {
            Self::Id => String::from("id"),
            Self::NodeId => String::from("node_id"),
            Self::Begin => String::from("begin"),
            Self::End => String::from("end"),
        }
    }
}


pub type DynErr = Box<dyn Error>;
pub type DynResult<T> = Result<T, DynErr>;

pub trait Storage {
    type Item;
    fn save(&self, item: &Self::Item) -> DynResult<usize>;
    fn all(&self) -> DynResult<Box<dyn RecordsSource<Item = Self::Item>>>;
    fn remove(&self, id: usize) -> DynResult<()>;
    fn filter(&self, filter: Filter) -> DynResult<Box<dyn RecordsSource<Item = Self::Item>>>; 
}

pub trait RecordsSource {
    type Item;
    fn fetch_next(&self) -> DynResult<Option<Self::Item>>;
    fn get_iter(&self) -> DynResult<Box<dyn Iterator<Item = Self::Item>>>;
}


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
