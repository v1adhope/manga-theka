use uuid::Uuid;

use crate::{
    entity::{Book, Pagination},
    error::ServiceError,
    service::Service,
};

impl Service {
    pub async fn store_book(&self, item: Book, label_ids: Vec<Uuid>) -> Result<(), ServiceError> {
        self.database
            .store_book(&item, &label_ids)
            .await
            .map_err(Into::into)
    }

    pub async fn get_book(&self, id: Uuid) -> Result<Book, ServiceError> {
        self.database.get_book(id).await.map_err(Into::into)
    }

    pub async fn get_books(
        &self,
        pagination: Pagination,
    ) -> Result<(Vec<Book>, Option<Uuid>), ServiceError> {
        self.database
            .get_books(&pagination)
            .await
            .map_err(Into::into)
    }

    pub async fn update_book(&self, item: Book, label_ids: Vec<Uuid>) -> Result<(), ServiceError> {
        self.database
            .update_book(&item, &label_ids)
            .await
            .map_err(Into::into)
    }

    pub async fn delete_book(&self, id: Uuid) -> Result<(), ServiceError> {
        self.database.delete_book(id).await.map_err(Into::into)
    }
}
