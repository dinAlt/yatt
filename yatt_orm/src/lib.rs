use chrono::Utc;
use uuid::Uuid;

pub mod errors;
pub mod statement;

pub use errors::*;
pub use yatt_orm_derive::*;

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

pub enum SyncActionType {
    Create,
    Update,
    Delete,
}

impl From<usize> for SyncActionType {
    fn from(u: usize) -> SyncActionType {
        match u {
            0 => SyncActionType::Create,
            1 => SyncActionType::Update,
            2 => SyncActionType::Delete,
            _ => panic!("wrong argument value"),
        }
    }
}

pub struct SyncAction {
    pub date: Utc,
    pub uuid: Uuid,
    pub action_type: SyncActionType,
}

pub struct SyncEntity {
    pub entity_type: String,
    pub id: usize,
    pub uuid: Uuid,
}

pub trait SyncStorage {
    fn push_entity(e: &SyncEntity) -> DBResult<()>;
    fn push_action(a: &SyncAction) -> DBResult<()>;
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
