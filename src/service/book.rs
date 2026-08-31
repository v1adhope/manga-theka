use uuid::Uuid;

use crate::{
    entity::{
        Book, BookCover, BookCoverQuery, BookFilter, BookQuery, BookVisibility,
        BookVisibilityUpdate, Entity, UserClaims, VisibilityTransition,
    },
    error::{DatabaseError, EntityError, ServiceError},
    service::Service,
};

pub(super) fn ensure_book_writable(visibility: BookVisibility) -> Result<(), ServiceError> {
    match visibility {
        BookVisibility::Draft | BookVisibility::Listed => Ok(()),
        blocked => Err(EntityError::BookNotWritable(blocked).into()),
    }
}

pub(super) fn ensure_content_writable(visibility: BookVisibility) -> Result<(), ServiceError> {
    match visibility {
        BookVisibility::Listed => Ok(()),
        blocked => Err(EntityError::BookContentNotWritable(blocked).into()),
    }
}

// `T` is the resource the caller asked for, not the book: refusing to serve unlisted content
// must be indistinguishable from that resource never having existed, or the 404 becomes an
// oracle telling anyone holding an id that a book was moderated away rather than deleted.
pub(super) fn ensure_readable<T: Entity>(
    visibility: BookVisibility,
    claims: &UserClaims,
) -> Result<(), ServiceError> {
    // deferred: also admit the book's submitter once `books` records one (issue #3)
    if visibility == BookVisibility::Listed || claims.can_moderate() {
        return Ok(());
    }

    Err(DatabaseError::not_found::<T>().into())
}

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
        let visibility = self.database.ensure_book_exists(item.id).await?;
        ensure_book_writable(visibility)?;

        self.database.update_book(&item).await.map_err(Into::into)
    }

    pub async fn set_book_visibility(
        &self,
        item: VisibilityTransition,
    ) -> Result<(), ServiceError> {
        let from = self.database.ensure_book_exists(item.id).await?;
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
        let visibility = self.database.ensure_book_exists(item.book_id).await?;
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
        self.database.ensure_book_exists(book_id).await?;

        self.database
            .get_book_covers(book_id)
            .await
            .map_err(Into::into)
    }

    // deferred: allow the submitter through once `books` records one
    pub async fn presign_book_cover(
        &self,
        id: Uuid,
        claims: &UserClaims,
    ) -> Result<String, ServiceError> {
        let visibility = self.database.ensure_book_cover_exists(id).await?;
        ensure_readable::<BookCover>(visibility, claims)?;

        self.storage
            .presign_book_cover(id)
            .await
            .map_err(Into::into)
    }

    pub async fn promote_book_cover(&self, book_id: Uuid, id: Uuid) -> Result<(), ServiceError> {
        let visibility = self.database.ensure_book_exists(book_id).await?;
        ensure_book_writable(visibility)?;

        self.database
            .promote_book_cover(book_id, id)
            .await
            .map_err(Into::into)
    }

    pub async fn delete_book_cover(&self, id: Uuid) -> Result<(), ServiceError> {
        let visibility = self.database.ensure_book_cover_exists(id).await?;
        ensure_book_writable(visibility)?;

        self.database.delete_book_cover(id).await?;

        let _ = self.storage.delete_book_covers(&[id]).await;

        Ok(())
    }
}
