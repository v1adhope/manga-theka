use uuid::Uuid;

use crate::{
    entity::{Creator, Filter},
    error::ServiceError,
    service::Service,
};

impl Service {
    pub async fn store_creator(&self, item: Creator) -> Result<(), ServiceError> {
        self.database.store_creator(&item).await.map_err(Into::into)
    }

    pub async fn get_creator(&self, id: Uuid) -> Result<Creator, ServiceError> {
        self.database.get_creator(id).await.map_err(Into::into)
    }

    pub async fn get_creators(
        &self,
        filter: Filter,
    ) -> Result<(Vec<Creator>, Option<Uuid>), ServiceError> {
        self.database
            .get_creators(&filter)
            .await
            .map_err(Into::into)
    }

    pub async fn update_creator(&self, item: Creator) -> Result<(), ServiceError> {
        self.database
            .update_creator(&item)
            .await
            .map_err(Into::into)
    }

    pub async fn delete_creator(&self, id: Uuid) -> Result<(), ServiceError> {
        self.database.delete_creator(id).await.map_err(Into::into)
    }
}
