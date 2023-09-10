use mongodb::{Collection, Database, GridFsBucket};

use super::data::{LingeringTask, RegStageUser, User};

#[derive(Debug, Clone)]
pub struct MongoDatabase {
    pub inner: Database,
    pub reg_stage: Collection<RegStageUser>,
    pub users: Collection<User>,
    pub tasks: Collection<LingeringTask>,
    pub gridfs: GridFsBucket,
}

impl MongoDatabase {
    pub fn new(inner: Database) -> Self {
        let reg_stage: Collection<RegStageUser> = inner.collection("reg_stage_users");
        let users: Collection<User> = inner.collection("users");
        let tasks: Collection<LingeringTask> = inner.collection("lingering_tasks");
        let gridfs = inner.gridfs_bucket(None);
        Self {
            inner,
            reg_stage,
            users,
            tasks,
            gridfs,
        }
    }
}
