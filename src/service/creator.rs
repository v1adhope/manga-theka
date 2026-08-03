use uuid::Uuid;

use crate::{
    database::Database,
    entity::{Creator, Pagination},
    error::ServiceError,
};

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

    pub async fn get_creator(&self, id: Uuid) -> Result<Creator, ServiceError> {
        self.database.get_creator(id).await.map_err(Into::into)
    }

    pub async fn get_creators(
        &self,
        pagination: Pagination,
    ) -> Result<(Vec<Creator>, Option<Uuid>), ServiceError> {
        self.database
            .get_creators(&pagination)
            .await
            .map_err(Into::into)
    }

    pub async fn update_creator(&self, item: Creator) -> Result<(), ServiceError> {
        self.database.update_creator(item).await.map_err(Into::into)
    }

    pub async fn delete_creator(&self, id: Uuid) -> Result<(), ServiceError> {
        self.database.delete_creator(id).await.map_err(Into::into)
    }
}
