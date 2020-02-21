use std::convert::TryInto;

use chrono::Utc;
use uuid::Uuid;

use crate::core::DBRoot;
use yatt_orm::statement::{Filter, Statement};
use yatt_orm::{
    DBError, DBResult, HistoryRecord, HistoryRecordType, HistoryStorage, Storage, StoreObject,
};

pub(crate) struct DBWatcher<T: DBRoot, S: HistoryStorage> {
    db: T,
    history_storage: S,
}

impl<T, S> DBWatcher<T, S>
where
    T: DBRoot,
    S: HistoryStorage,
{
    pub fn new(db: T, history_storage: S) -> Self {
        DBWatcher {
            db,
            history_storage,
        }
    }
}

impl<T, S> DBRoot for DBWatcher<T, S>
where
    T: DBRoot,
    S: HistoryStorage,
{
}

impl<T, S> Storage for DBWatcher<T, S>
where
    T: DBRoot,
    S: HistoryStorage,
{
    fn save(&self, item: &impl StoreObject) -> DBResult<usize>
    where
        Self: Sized,
    {
        let entity_id = self.db.save(item)?;

        let uid = self
            .history_storage
            .get_entity_guid(item.get_field_val("id").try_into()?, item.get_type_name());

        let (uid, is_new) = match uid {
            Ok(uid) => (uid, false),
            Err(e) => {
                if let DBError::IsEmpty { message: _ } = e {
                    (Uuid::new_v4(), true)
                } else {
                    return Err(e);
                }
            }
        };

        let record_type = if is_new {
            HistoryRecordType::Create
        } else {
            HistoryRecordType::Update
        };

        self.history_storage.push_record(HistoryRecord {
            date: Utc::now(),
            uuid: uid,
            record_type,
            entity_type: item.get_type_name().into(),
            entity_id,
        })?;

        Ok(entity_id)
    }
    fn get_all<U: StoreObject>(&self) -> DBResult<Vec<U>>
    where
        Self: Sized,
    {
        self.db.get_all()
    }
    fn remove_by_filter<U: StoreObject>(&self, filter: Filter) -> DBResult<()>
    where
        Self: Sized,
    {
        let rows: Vec<U> = self.db.get_by_filter(filter.clone())?;
        self.db.remove_by_filter::<U>(filter)?;

        for r in rows {
            let uid = self
                .history_storage
                .get_entity_guid(r.get_field_val("id").try_into()?, r.get_type_name())?;
            self.history_storage.push_record(HistoryRecord {
                date: Utc::now(),
                uuid: uid,
                record_type: HistoryRecordType::Delete,
                entity_type: r.get_type_name().into(),
                entity_id: r.get_field_val("id").try_into()?,
            })?;
        }
        Ok(())
    }
    fn get_by_statement<U: StoreObject>(&self, s: Statement) -> DBResult<Vec<U>>
    where
        Self: Sized,
    {
        self.db.get_by_statement(s)
    }
}
