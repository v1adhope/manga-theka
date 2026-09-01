use uuid::Uuid;

use crate::{
    entity::{Chapter, Filter},
    error::ServiceError,
    service::{Service, visibility::ensure_content_writable},
};

impl Service {
    pub async fn store_chapter(&self, item: Chapter) -> Result<(), ServiceError> {
        let visibility = self.database.ensure_book_exists(item.book_id).await?;
        ensure_content_writable(visibility)?;

        self.database.store_chapter(&item).await.map_err(Into::into)
    }

    pub async fn get_chapter(&self, id: Uuid) -> Result<Chapter, ServiceError> {
        self.database.get_chapter(id).await.map_err(Into::into)
    }

    pub async fn get_chapters(
        &self,
        book_id: Uuid,
        filter: Filter,
    ) -> Result<(Vec<Chapter>, Option<Uuid>), ServiceError> {
        self.database.ensure_book_exists(book_id).await?;

        self.database
            .get_chapters(book_id, &filter)
            .await
            .map_err(Into::into)
    }

    pub async fn update_chapter(&self, item: Chapter) -> Result<(), ServiceError> {
        let visibility = self.database.ensure_chapter_exists(item.id).await?;
        ensure_content_writable(visibility)?;

        self.database
            .update_chapter(&item)
            .await
            .map_err(Into::into)
    }

    pub async fn delete_chapter(&self, id: Uuid) -> Result<(), ServiceError> {
        let visibility = self.database.ensure_chapter_exists(id).await?;
        ensure_content_writable(visibility)?;

        self.database.delete_chapter(id).await.map_err(Into::into)
    }
}
