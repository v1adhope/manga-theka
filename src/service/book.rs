use uuid::Uuid;

use crate::{
    entity::{
        Book, BookCover, BookCoverQuery, BookFilter, BookQuery, BookVisibilityUpdate, UserClaims,
        VisibilityTransition,
    },
    error::ServiceError,
    service::Service,
};

impl Service {
    pub async fn store_book(&self, item: Book) -> Result<(), ServiceError> {
        self.database.store_book(&item).await.map_err(Into::into)
    }

    pub async fn get_book(&self, id: Uuid) -> Result<BookQuery, ServiceError> {
        self.database.get_book(id).await.map_err(Into::into)
    }

    pub async fn get_books(
        &self,
        filter: BookFilter,
    ) -> Result<(Vec<BookQuery>, Option<String>), ServiceError> {
        self.database.get_books(&filter).await.map_err(Into::into)
    }

    pub async fn update_book(&self, item: Book) -> Result<(), ServiceError> {
        self.ensure_book_writable(item.id).await?;

        self.database.update_book(&item).await.map_err(Into::into)
    }

    pub async fn set_book_visibility(
        &self,
        item: VisibilityTransition,
    ) -> Result<(), ServiceError> {
        let from = self.database.get_book_visibility(item.id).await?;
        let item = BookVisibilityUpdate::try_from((from, item))?;

        self.database
            .set_book_visibility(&item)
            .await
            .map_err(Into::into)
    }

    pub async fn delete_book(&self, id: Uuid) -> Result<(), ServiceError> {
        let cover_ids = self.database.get_book_cover_ids(id).await?;

        self.database.delete_book(id).await?;

        let _ = self.storage.delete_book_covers(&cover_ids).await;

        Ok(())
    }

    pub async fn store_book_cover(&self, item: BookCover) -> Result<(), ServiceError> {
        self.ensure_book_writable(item.book_id).await?;

        self.storage.upload_book_cover(&item).await?;

        self.database
            .store_book_cover(&item)
            .await
            .map_err(Into::into)
    }

    pub async fn get_book_covers(
        &self,
        book_id: Uuid,
    ) -> Result<Vec<BookCoverQuery>, ServiceError> {
        self.ensure_book_exists(book_id).await?;

        self.database
            .get_book_covers(book_id)
            .await
            .map_err(Into::into)
    }

    // deferred: allow the submitter through once `books` records one
    pub async fn presign_book_cover(
        &self,
        id: Uuid,
        claims: Option<&UserClaims>,
    ) -> Result<String, ServiceError> {
        self.ensure_cover_readable(id, claims).await?;

        self.storage
            .presign_book_cover(id)
            .await
            .map_err(Into::into)
    }

    pub async fn promote_book_cover(&self, book_id: Uuid, id: Uuid) -> Result<(), ServiceError> {
        self.ensure_book_writable(book_id).await?;

        self.database
            .promote_book_cover(book_id, id)
            .await
            .map_err(Into::into)
    }

    pub async fn delete_book_cover(&self, id: Uuid) -> Result<(), ServiceError> {
        self.ensure_book_writable_by_cover(id).await?;

        self.database.delete_book_cover(id).await?;

        let _ = self.storage.delete_book_covers(&[id]).await;

        Ok(())
    }
}
