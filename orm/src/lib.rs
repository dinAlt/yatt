pub mod errors;
pub mod filter;
pub use orm_derive::*;

pub use errors::*;
use filter::*;

pub type BoxStorage<T> = Box<dyn Storage<Item = T>>;

pub trait Storage {
    type Item;
    fn save(&self, item: &Self::Item) -> DBResult<usize>;
    fn all(&self) -> DBResult<Vec<Self::Item>>;
    fn remove(&self, id: usize) -> DBResult<()>;
    fn filter(&self, filter: Filter) -> DBResult<Vec<Self::Item>>;
    fn with_max(&self, field: &str) -> DBResult<Option<Self::Item>>;
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
