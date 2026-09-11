use tracing::instrument;
use uuid::Uuid;

use crate::{
    database::{Database, Invariant},
    entity::{
        Book, BookAccess, BookCover, BookVisibilityUpdate, Chapter, ChapterPage, ChapterRelease,
        ReleaseAccess,
    },
    error::{DatabaseError, LogInternal},
};

impl TryFrom<(String, Uuid)> for BookAccess {
    type Error = DatabaseError;

    fn try_from((visibility, created_by): (String, Uuid)) -> Result<Self, Self::Error> {
        let visibility = visibility
            .parse()
            .or_corrupted("visibility")
            .inspect_err(DatabaseError::log_internal)?;

        Ok(BookAccess {
            visibility,
            created_by,
        })
    }
}

impl Database {
    #[instrument(name = "db.book.set_visibility", skip_all, fields(book.id = %item.id))]
    pub async fn set_book_visibility(
        &self,
        item: &BookVisibilityUpdate,
    ) -> Result<(), DatabaseError> {
        let row = sqlx::query_file!(
            "queries/set_book_visibility.sql",
            item.id,
            item.to.as_ref(),
            item.note.as_ref().map(AsRef::as_ref),
            item.submitted_at,
            item.updated_at,
            item.from.as_ref(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(DatabaseError::from)
        .inspect_err(DatabaseError::log_internal)?;

        if row.is_none() {
            return Err(DatabaseError::not_found::<Book>());
        }

        Ok(())
    }

    #[instrument(name = "db.book.access", skip_all, fields(book.id = %id))]
    pub async fn get_book_access(&self, id: Uuid) -> Result<BookAccess, DatabaseError> {
        let row = sqlx::query_file!("queries/book_visibility.sql", id)
            .fetch_optional(&self.pool)
            .await
            .map_err(DatabaseError::from)
            .inspect_err(DatabaseError::log_internal)?;

        let Some(row) = row else {
            return Err(DatabaseError::not_found::<Book>());
        };

        (row.visibility, row.created_by).try_into()
    }

    #[instrument(name = "db.book_cover.access", skip_all, fields(cover.id = %id))]
    pub async fn get_book_access_by_cover(&self, id: Uuid) -> Result<BookAccess, DatabaseError> {
        let row = sqlx::query_file!("queries/book_visibility_by_cover.sql", id)
            .fetch_optional(&self.pool)
            .await
            .map_err(DatabaseError::from)
            .inspect_err(DatabaseError::log_internal)?;

        let Some(row) = row else {
            return Err(DatabaseError::not_found::<BookCover>());
        };

        (row.visibility, row.created_by).try_into()
    }

    #[instrument(name = "db.chapter.access", skip_all, fields(chapter.id = %id))]
    pub async fn get_book_access_by_chapter(&self, id: Uuid) -> Result<BookAccess, DatabaseError> {
        let row = sqlx::query_file!("queries/book_visibility_by_chapter.sql", id)
            .fetch_optional(&self.pool)
            .await
            .map_err(DatabaseError::from)
            .inspect_err(DatabaseError::log_internal)?;

        let Some(row) = row else {
            return Err(DatabaseError::not_found::<Chapter>());
        };

        (row.visibility, row.created_by).try_into()
    }

    #[instrument(name = "db.chapter_release.access", skip_all, fields(release.id = %id))]
    pub async fn get_release_access(&self, id: Uuid) -> Result<ReleaseAccess, DatabaseError> {
        let row = sqlx::query_file!("queries/release_access.sql", id)
            .fetch_optional(&self.pool)
            .await
            .map_err(DatabaseError::from)
            .inspect_err(DatabaseError::log_internal)?;

        let Some(row) = row else {
            return Err(DatabaseError::not_found::<ChapterRelease>());
        };

        let visibility = row
            .visibility
            .parse()
            .or_corrupted("visibility")
            .inspect_err(DatabaseError::log_internal)?;

        Ok(ReleaseAccess {
            visibility,
            created_by: row.created_by,
        })
    }

    #[instrument(name = "db.chapter_page.exists", skip_all, fields(release.id = %release_id, page.id = %id))]
    pub async fn ensure_chapter_page_exists(
        &self,
        release_id: Uuid,
        id: Uuid,
    ) -> Result<(), DatabaseError> {
        let row = sqlx::query_file_scalar!("queries/book_visibility_by_page.sql", id, release_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(DatabaseError::from)
            .inspect_err(DatabaseError::log_internal)?;

        if row.is_none() {
            return Err(DatabaseError::not_found::<ChapterPage>());
        }

        Ok(())
    }
}
