use uuid::Uuid;

use crate::{
    entity::{BookCover, Chapter, Entity, UserClaims},
    error::ServiceError,
    service::Service,
};

impl Service {
    // -- record-tier reads: the book row, its chapters, its covers -------------

    pub(crate) async fn ensure_book_readable<T: Entity>(
        &self,
        book_id: Uuid,
        claims: Option<&UserClaims>,
    ) -> Result<(), ServiceError> {
        self.database
            .get_book_access(book_id)
            .await?
            .ensure_readable::<T>(claims)
            .map_err(Into::into)
    }

    pub(crate) async fn ensure_chapter_readable(
        &self,
        chapter_id: Uuid,
        claims: Option<&UserClaims>,
    ) -> Result<(), ServiceError> {
        self.database
            .get_book_access_by_chapter(chapter_id)
            .await?
            .ensure_readable::<Chapter>(claims)
            .map_err(Into::into)
    }

    pub(crate) async fn ensure_cover_readable(
        &self,
        cover_id: Uuid,
        claims: Option<&UserClaims>,
    ) -> Result<(), ServiceError> {
        self.database
            .get_book_access_by_cover(cover_id)
            .await?
            .ensure_readable::<BookCover>(claims)
            .map_err(Into::into)
    }

    // -- book and cover writes ------------------------------------------------

    pub(crate) async fn ensure_book_writable(
        &self,
        book_id: Uuid,
        claims: &UserClaims,
    ) -> Result<(), ServiceError> {
        self.database
            .get_book_access(book_id)
            .await?
            .ensure_book_writable(claims)
            .map_err(Into::into)
    }

    pub(crate) async fn ensure_book_writable_by_cover(
        &self,
        cover_id: Uuid,
        claims: &UserClaims,
    ) -> Result<(), ServiceError> {
        self.database
            .get_book_access_by_cover(cover_id)
            .await?
            .ensure_book_writable(claims)
            .map_err(Into::into)
    }

    // -- content writes: chapters, releases, pages --------------------------

    pub(crate) async fn ensure_content_writable(&self, book_id: Uuid) -> Result<(), ServiceError> {
        self.database
            .get_book_access(book_id)
            .await?
            .visibility
            .ensure_content_writable()
            .map_err(Into::into)
    }

    pub(crate) async fn ensure_content_writable_by_chapter(
        &self,
        chapter_id: Uuid,
    ) -> Result<(), ServiceError> {
        self.database
            .get_book_access_by_chapter(chapter_id)
            .await?
            .visibility
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

    // -- release and page reads --------------------------------------------

    pub(crate) async fn ensure_release_record_readable(
        &self,
        release_id: Uuid,
        claims: Option<&UserClaims>,
    ) -> Result<(), ServiceError> {
        self.database
            .get_release_access(release_id)
            .await?
            .ensure_record_readable(claims)
            .map_err(Into::into)
    }

    pub(crate) async fn ensure_release_content_readable(
        &self,
        release_id: Uuid,
        claims: Option<&UserClaims>,
    ) -> Result<(), ServiceError> {
        self.database
            .get_release_access(release_id)
            .await?
            .ensure_content_readable(claims)
            .map_err(Into::into)
    }

    pub(crate) async fn ensure_release_staged_readable(
        &self,
        release_id: Uuid,
        claims: Option<&UserClaims>,
    ) -> Result<(), ServiceError> {
        self.database
            .get_release_access(release_id)
            .await?
            .ensure_staged_readable(claims)
            .map_err(Into::into)
    }

    pub(crate) async fn ensure_page_readable(
        &self,
        release_id: Uuid,
        page_id: Uuid,
        claims: Option<&UserClaims>,
    ) -> Result<(), ServiceError> {
        // Standing before sub-resource existence: a caller who fails the tier
        // gets `403` even when the page id in the path is also absent.
        self.database
            .get_release_access(release_id)
            .await?
            .ensure_content_readable(claims)?;

        self.database
            .ensure_chapter_page_exists(release_id, page_id)
            .await
            .map_err(Into::into)
    }
}
