use std::rc::Rc;

use chrono::Utc;
use uuid::Uuid;

use crate::core::{DBRoot, Interval, Node};
use yatt_orm::statement::Statement;
use yatt_orm::{
    BoxStorage, DBError, DBResult, HistoryRecord, HistoryRecordType, HistoryStorage, Storage,
};

pub(crate) struct DBWatcher {
    db: Box<dyn DBRoot>,
    history_storage: Rc<dyn HistoryStorage>,
}

pub(crate) trait LocalUnique {
    fn get_local_id(&self) -> usize;
}

impl DBWatcher {
    pub fn new(db: Box<dyn DBRoot>, history_storage: Rc<dyn HistoryStorage>) -> Self {
        DBWatcher {
            db,
            history_storage,
        }
    }
}

impl DBRoot for DBWatcher {
    fn nodes(&self) -> BoxStorage<Node> {
        Box::new(StorageWatcher::new(
            "nodes",
            self.db.nodes(),
            Rc::clone(&self.history_storage),
        ))
    }
    fn intervals(&self) -> BoxStorage<Interval> {
        Box::new(StorageWatcher::new(
            "intervals",
            self.db.intervals(),
            Rc::clone(&self.history_storage),
        ))
    }
}

struct StorageWatcher<T: LocalUnique> {
    entity_type: &'static str,
    storage: BoxStorage<T>,
    history_storage: Rc<dyn HistoryStorage>,
}

impl<T: LocalUnique> StorageWatcher<T> {
    fn new(
        entity_type: &'static str,
        storage: BoxStorage<T>,
        history_storage: Rc<dyn HistoryStorage>,
    ) -> Self {
        StorageWatcher {
            entity_type,
            storage,
            history_storage,
        }
    }
}

impl<T: LocalUnique> Storage for StorageWatcher<T> {
    type Item = T;
    fn save(&self, item: &Self::Item) -> DBResult<usize> {
        let entity_id = self.storage.save(item)?;

        let uid = self
            .history_storage
            .get_entity_guid(item.get_local_id(), self.entity_type);

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
            entity_type: self.entity_type.to_string(),
            entity_id,
        })?;

        Ok(entity_id)
    }
    fn all(&self) -> DBResult<Vec<Self::Item>> {
        self.storage.all()
    }
    fn remove(&self, id: usize) -> DBResult<()> {
        self.storage.remove(id)?;

        let uid = self.history_storage.get_entity_guid(id, self.entity_type)?;
        self.history_storage.push_record(HistoryRecord {
            date: Utc::now(),
            uuid: uid,
            record_type: HistoryRecordType::Delete,
            entity_type: self.entity_type.to_string(),
            entity_id: id,
        })
    }
    fn by_statement(&self, s: Statement) -> DBResult<Vec<Self::Item>> {
        self.storage.by_statement(s)
    }
}
