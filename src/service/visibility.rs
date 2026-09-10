use uuid::Uuid;

use crate::{
    entity::{BookCover, ChapterPage, ChapterRelease, UserClaims},
    error::ServiceError,
    service::Service,
};

// deferred: once auth ships (issue #3) and the `created_by` carve-out lands in
// `ensure_readable`, consider folding these into one `guard(BookRef, Access)` fn
// or a fluent `book_by_*(id).content_writable()` guard.
impl Service {
    pub(crate) async fn ensure_book_exists(&self, book_id: Uuid) -> Result<(), ServiceError> {
        self.database
            .get_book_visibility(book_id)
            .await
            .map(|_| ())
            .map_err(Into::into)
    }

    pub(crate) async fn ensure_book_exists_by_chapter(
        &self,
        chapter_id: Uuid,
    ) -> Result<(), ServiceError> {
        self.database
            .get_book_visibility_by_chapter(chapter_id)
            .await
            .map(|_| ())
            .map_err(Into::into)
    }

    pub(crate) async fn ensure_book_writable(&self, book_id: Uuid) -> Result<(), ServiceError> {
        self.database
            .get_book_visibility(book_id)
            .await?
            .ensure_book_writable()
            .map_err(Into::into)
    }

    pub(crate) async fn ensure_book_writable_by_cover(
        &self,
        cover_id: Uuid,
    ) -> Result<(), ServiceError> {
        self.database
            .get_book_visibility_by_cover(cover_id)
            .await?
            .ensure_book_writable()
            .map_err(Into::into)
    }

    pub(crate) async fn ensure_content_writable(&self, book_id: Uuid) -> Result<(), ServiceError> {
        self.database
            .get_book_visibility(book_id)
            .await?
            .ensure_content_writable()
            .map_err(Into::into)
    }

    pub(crate) async fn ensure_content_writable_by_chapter(
        &self,
        chapter_id: Uuid,
    ) -> Result<(), ServiceError> {
        self.database
            .get_book_visibility_by_chapter(chapter_id)
            .await?
            .ensure_content_writable()
            .map_err(Into::into)
    }

    pub(crate) async fn ensure_release_mutable(
        &self,
        release_id: Uuid,
        claims: &UserClaims,
    ) -> Result<(), ServiceError> {
        self.database
            .get_release_access(release_id)
            .await?
            .ensure_mutable(claims)
            .map_err(Into::into)
    }

    pub(crate) async fn ensure_release_readable(
        &self,
        release_id: Uuid,
        claims: Option<&UserClaims>,
    ) -> Result<(), ServiceError> {
        self.database
            .get_release_access(release_id)
            .await?
            .visibility
            .ensure_readable::<ChapterRelease>(claims)
            .map_err(Into::into)
    }

    pub(crate) async fn ensure_page_readable(
        &self,
        release_id: Uuid,
        page_id: Uuid,
        claims: Option<&UserClaims>,
    ) -> Result<(), ServiceError> {
        self.database
            .get_book_visibility_by_page(release_id, page_id)
            .await?
            .ensure_readable::<ChapterPage>(claims)
            .map_err(Into::into)
    }

    pub(crate) async fn ensure_cover_readable(
        &self,
        cover_id: Uuid,
        claims: Option<&UserClaims>,
    ) -> Result<(), ServiceError> {
        self.database
            .get_book_visibility_by_cover(cover_id)
            .await?
            .ensure_readable::<BookCover>(claims)
            .map_err(Into::into)
    }
}
