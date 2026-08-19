use uuid::Uuid;

use crate::{
    entity::{Chapter, Pagination, SortOrder},
    error::ServiceError,
    service::Service,
};

impl Service {
    pub async fn store_chapter(&self, item: Chapter) -> Result<(), ServiceError> {
        self.database.store_chapter(&item).await.map_err(Into::into)
    }

    pub async fn get_chapter(&self, book_id: Uuid, id: Uuid) -> Result<Chapter, ServiceError> {
        self.database
            .get_chapter(book_id, id)
            .await
            .map_err(Into::into)
    }

    pub async fn get_chapters(
        &self,
        book_id: Uuid,
        pagination: Pagination,
        order: SortOrder,
    ) -> Result<(Vec<Chapter>, Option<Uuid>), ServiceError> {
        self.database.ensure_book_exists(book_id).await?;

        self.database
            .get_chapters(book_id, &pagination, order)
            .await
            .map_err(Into::into)
    }

    pub async fn update_chapter(&self, item: Chapter) -> Result<(), ServiceError> {
        self.database
            .update_chapter(&item)
            .await
            .map_err(Into::into)
    }

    pub async fn delete_chapter(&self, book_id: Uuid, id: Uuid) -> Result<(), ServiceError> {
        self.database
            .delete_chapter(book_id, id)
            .await
            .map_err(Into::into)
    }
}
