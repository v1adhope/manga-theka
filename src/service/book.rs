use uuid::Uuid;

use crate::{
    entity::{
        Book, BookAccess, BookCover, BookCoverQuery, BookCursor, BookFilter, BookQuery,
        BookVisibilityUpdate, UserClaims, VisibilityTransition,
    },
    error::ServiceError,
    service::Service,
};

impl Service {
    pub async fn store_book(&self, item: Book) -> Result<(), ServiceError> {
        self.database.store_book(&item).await.map_err(Into::into)
    }

    pub async fn get_book(
        &self,
        id: Uuid,
        claims: Option<&UserClaims>,
    ) -> Result<BookQuery, ServiceError> {
        let book = self.database.get_book(id).await?;
        BookAccess {
            visibility: book.visibility,
            created_by: book.created_by,
        }
        .ensure_readable::<Book>(claims)?;

        Ok(book)
    }

    pub async fn get_books(
        &self,
        filter: BookFilter,
    ) -> Result<(Vec<BookQuery>, Option<BookCursor>), ServiceError> {
        self.database.get_books(&filter).await.map_err(Into::into)
    }

    pub async fn update_book(&self, item: Book, claims: UserClaims) -> Result<(), ServiceError> {
        self.ensure_book_writable(item.id, &claims).await?;

        self.database.update_book(&item).await.map_err(Into::into)
    }

    pub async fn set_book_visibility(
        &self,
        item: VisibilityTransition,
        claims: UserClaims,
    ) -> Result<(), ServiceError> {
        // Fail in order (mirrors the release mutation gate, ADR-0003): `404`
        // unknown book, then `403` caller lacks standing, then `409`/`422` the
        // move itself is illegal.
        let access = self.database.get_book_access(item.id).await?;
        access.ensure_visibility_settable(item.visibility, &claims)?;
        let update = BookVisibilityUpdate::try_from((access.visibility, item))?;

        self.database
            .set_book_visibility(&update)
            .await
            .map_err(Into::into)
    }

    pub async fn delete_book(&self, id: Uuid) -> Result<(), ServiceError> {
        let cover_ids = self.database.get_book_cover_ids(id).await?;

        self.database.delete_book(id).await?;

        let _ = self.storage.delete_book_covers(&cover_ids).await;

        Ok(())
    }

    pub async fn store_book_cover(
        &self,
        item: BookCover,
        claims: UserClaims,
    ) -> Result<(), ServiceError> {
        self.ensure_book_writable(item.book_id, &claims).await?;

        self.storage.upload_book_cover(&item).await?;

        self.database
            .store_book_cover(&item)
            .await
            .map_err(Into::into)
    }

    pub async fn get_book_covers(
        &self,
        book_id: Uuid,
        claims: Option<&UserClaims>,
    ) -> Result<Vec<BookCoverQuery>, ServiceError> {
        self.ensure_book_readable::<BookCover>(book_id, claims)
            .await?;

        self.database
            .get_book_covers(book_id)
            .await
            .map_err(Into::into)
    }

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

    pub async fn promote_book_cover(
        &self,
        book_id: Uuid,
        id: Uuid,
        claims: UserClaims,
    ) -> Result<(), ServiceError> {
        self.ensure_book_writable(book_id, &claims).await?;

        self.database
            .promote_book_cover(book_id, id)
            .await
            .map_err(Into::into)
    }

    pub async fn delete_book_cover(
        &self,
        id: Uuid,
        claims: UserClaims,
    ) -> Result<(), ServiceError> {
        self.ensure_book_writable_by_cover(id, &claims).await?;

        self.database.delete_book_cover(id).await?;

        let _ = self.storage.delete_book_covers(&[id]).await;

        Ok(())
    }
}
