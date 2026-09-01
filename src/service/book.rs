use uuid::Uuid;

use crate::{
    entity::{
        Book, BookCover, BookCoverQuery, BookFilter, BookQuery, BookVisibilityUpdate, UserClaims,
        VisibilityTransition,
    },
    error::{EntityError, ServiceError},
    service::{
        Service,
        visibility::{ensure_book_writable, ensure_readable},
    },
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
    ) -> Result<(Vec<BookQuery>, Option<Uuid>), ServiceError> {
        self.database.get_books(&filter).await.map_err(Into::into)
    }

    pub async fn update_book(&self, item: Book) -> Result<(), ServiceError> {
        let visibility = self.database.get_book_visibility(item.id).await?;
        ensure_book_writable(visibility)?;

        self.database.update_book(&item).await.map_err(Into::into)
    }

    pub async fn set_book_visibility(
        &self,
        item: VisibilityTransition,
    ) -> Result<(), ServiceError> {
        let from = self.database.get_book_visibility(item.id).await?;
        let update = BookVisibilityUpdate::try_from((from, item))?;

        if !self.database.set_book_visibility(&update).await? {
            return Err(EntityError::IllegalVisibilityTransition(update.from, update.to).into());
        }

        Ok(())
    }

    pub async fn delete_book(&self, id: Uuid) -> Result<(), ServiceError> {
        let cover_ids = self.database.get_book_cover_ids(id).await?;

        self.database.delete_book(id).await?;

        let _ = self.storage.delete_book_covers(&cover_ids).await;

        Ok(())
    }

    pub async fn store_book_cover(&self, item: BookCover) -> Result<(), ServiceError> {
        let visibility = self.database.get_book_visibility(item.book_id).await?;
        ensure_book_writable(visibility)?;

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
        self.database.get_book_visibility(book_id).await?;

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
        let visibility = self.database.get_book_visibility_by_cover(id).await?;
        ensure_readable::<BookCover>(visibility, claims)?;

        self.storage
            .presign_book_cover(id)
            .await
            .map_err(Into::into)
    }

    pub async fn promote_book_cover(&self, book_id: Uuid, id: Uuid) -> Result<(), ServiceError> {
        let visibility = self.database.get_book_visibility(book_id).await?;
        ensure_book_writable(visibility)?;

        self.database
            .promote_book_cover(book_id, id)
            .await
            .map_err(Into::into)
    }

    pub async fn delete_book_cover(&self, id: Uuid) -> Result<(), ServiceError> {
        let visibility = self.database.get_book_visibility_by_cover(id).await?;
        ensure_book_writable(visibility)?;

        self.database.delete_book_cover(id).await?;

        let _ = self.storage.delete_book_covers(&[id]).await;

        Ok(())
    }
}
