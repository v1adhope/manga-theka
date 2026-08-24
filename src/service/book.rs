use uuid::Uuid;

use crate::{
    entity::{Book, BookCover, BookCoverQuery, Filter},
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
        filter: Filter,
    ) -> Result<(Vec<Book>, Option<Uuid>), ServiceError> {
        self.database.get_books(&filter).await.map_err(Into::into)
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

        self.storage
            .delete_book_covers(&cover_ids)
            .await
            .map_err(Into::into)
    }

    pub async fn store_book_cover(&self, item: &BookCover) -> Result<(), ServiceError> {
        self.database.ensure_book_exists(item.book_id).await?;

        self.storage.upload_book_cover(item).await?;

        self.database
            .store_book_cover(item)
            .await
            .map_err(Into::into)
    }

    pub async fn get_book_covers(
        &self,
        book_id: Uuid,
    ) -> Result<Vec<BookCoverQuery>, ServiceError> {
        self.database.ensure_book_exists(book_id).await?;

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
        self.database.ensure_book_cover_exists(book_id, id).await?;

        self.storage
            .presign_book_cover(id)
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

        self.storage
            .delete_book_covers(&[id])
            .await
            .map_err(Into::into)
    }
}
