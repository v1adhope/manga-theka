use uuid::Uuid;

use crate::{
    entity::{Chapter, Filter, UserClaims},
    error::ServiceError,
    service::Service,
};

impl Service {
    pub async fn store_chapter(&self, item: Chapter) -> Result<(), ServiceError> {
        self.ensure_book_content_writable(item.book_id).await?;

        self.database.store_chapter(&item).await.map_err(Into::into)
    }

    pub async fn get_chapter(
        &self,
        id: Uuid,
        claims: Option<&UserClaims>,
    ) -> Result<Chapter, ServiceError> {
        self.ensure_book_record_readable_by_chapter(id, claims)
            .await?;

        self.database.get_chapter(id).await.map_err(Into::into)
    }

    pub async fn get_chapters(
        &self,
        book_id: Uuid,
        filter: Filter,
        claims: Option<&UserClaims>,
    ) -> Result<(Vec<Chapter>, Option<Uuid>), ServiceError> {
        self.ensure_book_record_readable::<Chapter>(book_id, claims)
            .await?;

        self.database
            .get_chapters(book_id, &filter)
            .await
            .map_err(Into::into)
    }

    pub async fn update_chapter(&self, item: Chapter) -> Result<(), ServiceError> {
        self.ensure_book_content_writable_by_chapter(item.id)
            .await?;

        self.database
            .update_chapter(&item)
            .await
            .map_err(Into::into)
    }

    pub async fn delete_chapter(&self, id: Uuid) -> Result<(), ServiceError> {
        self.ensure_book_content_writable_by_chapter(id).await?;

        self.database.delete_chapter(id).await.map_err(Into::into)
    }
}
