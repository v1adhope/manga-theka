use std::time::Duration;

use bytes::Bytes;
use uuid::Uuid;

use crate::{
    entity::{Book, BookCover, Pagination},
    error::ServiceError,
    service::Service,
};

const COVER_PRESIGN_TTL: Duration = Duration::from_secs(300);

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
        let cover_ids = self.database.get_book_cover_ids(id).await?;

        self.database.delete_book(id).await?;

        for cover_id in cover_ids {
            let _ = self.storage.delete_cover(&cover_id.to_string()).await;
        }

        Ok(())
    }

    pub async fn store_book_cover(
        &self,
        book_id: Uuid,
        item: &BookCover,
        body: Bytes,
    ) -> Result<(), ServiceError> {
        self.database.book_exists(book_id).await?;

        self.storage
            .upload_cover(&item.id.to_string(), body, item.extension.content_type())
            .await?;

        self.database
            .store_book_cover(book_id, item)
            .await
            .map_err(Into::into)
    }

    pub async fn get_book_covers(&self, book_id: Uuid) -> Result<Vec<BookCover>, ServiceError> {
        self.database
            .get_book_covers(book_id)
            .await
            .map_err(Into::into)
    }

    pub async fn presign_book_cover(
        &self,
        book_id: Uuid,
        id: Uuid,
    ) -> Result<String, ServiceError> {
        let extension = self.database.get_book_cover(book_id, id).await?;
        let disposition = format!("inline; filename=\"{id}.{}\"", extension.as_ref());

        self.storage
            .presign_cover(&id.to_string(), COVER_PRESIGN_TTL, &disposition)
            .await
            .map_err(Into::into)
    }

    pub async fn promote_book_cover(&self, book_id: Uuid, id: Uuid) -> Result<(), ServiceError> {
        self.database
            .promote_book_cover(book_id, id)
            .await
            .map_err(Into::into)
    }

    pub async fn delete_book_cover(&self, book_id: Uuid, id: Uuid) -> Result<(), ServiceError> {
        self.database.delete_book_cover(book_id, id).await?;

        let _ = self.storage.delete_cover(&id.to_string()).await;

        Ok(())
    }
}
