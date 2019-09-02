pub mod errors;
pub mod statement;
pub use orm_derive::*;

pub use errors::*;
use statement::*;

pub type BoxStorage<T> = Box<dyn Storage<Item = T>>;

pub trait Storage {
    type Item;
    fn save(&self, item: &Self::Item) -> DBResult<usize>;
    fn all(&self) -> DBResult<Vec<Self::Item>>;
    fn remove(&self, id: usize) -> DBResult<()>;
    fn by_statement(&self, s: Statement) -> DBResult<Vec<Self::Item>>;
}

impl<T: Clone> dyn Storage<Item = T> {
    pub fn by_id(&self, id: usize) -> DBResult<T> {
        let res = self.by_statement(filter(eq("id".to_string(), id)))?;
        if res.is_empty() {
            return Err(DBError::IsEmpty {
                message: format!("no row with id {}", id),
            });
        }

        Ok(res.first().unwrap().to_owned())
    }
    pub fn filter(&self, f: Filter) -> DBResult<Vec<T>> {
        let res = self.by_statement(filter(f))?;
        Ok(res)
    }
    pub fn with_max(&self, field: &str) -> DBResult<Option<T>> {
        let res = self.by_statement(sort(field, SortDir::Descend).limit(1))?;
        if res.is_empty() {
            return Ok(None);
        }
        Ok(Some(res.first().unwrap().to_owned()))
    }
}

// pub trait StorageObject {
//     fn field_list() -> &'static [&'static str];
// }

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
