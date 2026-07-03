use crate::{database::Database, entity::Creator, error::ServiceError};

#[derive(Debug, Clone)]
pub struct Service {
    pub database: Database,
}

impl Service {
    pub fn new(database: Database) -> Self {
        Self { database }
    }

    pub async fn store_creator(&self, item: Creator) -> Result<(), ServiceError> {
        self.database.store_creator(item).await.map_err(Into::into)
    }
}
