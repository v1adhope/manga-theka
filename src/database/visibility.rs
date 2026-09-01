use tracing::instrument;
use uuid::Uuid;

use crate::{
    database::{Database, Invariant},
    entity::{
        Book, BookCover, BookVisibility, BookVisibilityUpdate, Chapter, ChapterPage,
        ChapterRelease, Entity,
    },
    error::DatabaseError,
};

fn require_visibility<T: Entity>(
    visibility: Option<String>,
) -> Result<BookVisibility, DatabaseError> {
    let Some(visibility) = visibility else {
        return Err(DatabaseError::not_found::<T>());
    };

    visibility
        .parse()
        .or_corrupted("visibility")
        .inspect_err(DatabaseError::log_internal)
}

impl Database {
    #[instrument(name = "db.book.set_visibility", skip_all, fields(book.id = %item.id))]
    pub async fn set_book_visibility(
        &self,
        item: &BookVisibilityUpdate,
    ) -> Result<bool, DatabaseError> {
        let row = sqlx::query_file!(
            "queries/set_book_visibility.sql",
            item.id,
            item.to.as_ref(),
            item.note.as_ref().map(AsRef::as_ref),
            item.submitted_at,
            item.from.as_ref(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(DatabaseError::from)
        .inspect_err(DatabaseError::log_internal)?;

        Ok(row.is_some())
    }

    #[instrument(name = "db.book.visibility", skip_all, fields(book.id = %id))]
    pub async fn get_book_visibility(&self, id: Uuid) -> Result<BookVisibility, DatabaseError> {
        sqlx::query_file_scalar!("queries/book_visibility.sql", id)
            .fetch_optional(&self.pool)
            .await
            .map_err(DatabaseError::from)
            .inspect_err(DatabaseError::log_internal)
            .and_then(require_visibility::<Book>)
    }

    #[instrument(name = "db.book_cover.visibility", skip_all, fields(cover.id = %id))]
    pub async fn get_book_visibility_by_cover(
        &self,
        id: Uuid,
    ) -> Result<BookVisibility, DatabaseError> {
        sqlx::query_file_scalar!("queries/book_visibility_by_cover.sql", id)
            .fetch_optional(&self.pool)
            .await
            .map_err(DatabaseError::from)
            .inspect_err(DatabaseError::log_internal)
            .and_then(require_visibility::<BookCover>)
    }

    #[instrument(name = "db.chapter.visibility", skip_all, fields(chapter.id = %id))]
    pub async fn get_book_visibility_by_chapter(
        &self,
        id: Uuid,
    ) -> Result<BookVisibility, DatabaseError> {
        sqlx::query_file_scalar!("queries/book_visibility_by_chapter.sql", id)
            .fetch_optional(&self.pool)
            .await
            .map_err(DatabaseError::from)
            .inspect_err(DatabaseError::log_internal)
            .and_then(require_visibility::<Chapter>)
    }

    #[instrument(name = "db.chapter_release.visibility", skip_all, fields(release.id = %id))]
    pub async fn get_book_visibility_by_release(
        &self,
        id: Uuid,
    ) -> Result<BookVisibility, DatabaseError> {
        sqlx::query_file_scalar!("queries/book_visibility_by_release.sql", id)
            .fetch_optional(&self.pool)
            .await
            .map_err(DatabaseError::from)
            .inspect_err(DatabaseError::log_internal)
            .and_then(require_visibility::<ChapterRelease>)
    }

    #[instrument(name = "db.chapter_page.visibility", skip_all, fields(release.id = %release_id, page.id = %id))]
    pub async fn get_book_visibility_by_page(
        &self,
        release_id: Uuid,
        id: Uuid,
    ) -> Result<BookVisibility, DatabaseError> {
        sqlx::query_file_scalar!("queries/book_visibility_by_page.sql", id, release_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(DatabaseError::from)
            .inspect_err(DatabaseError::log_internal)
            .and_then(require_visibility::<ChapterPage>)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        database::visibility::require_visibility,
        entity::{Book, BookVisibility},
        error::DatabaseError,
    };

    #[test]
    fn a_missing_row_is_reported_as_not_found() {
        let visibility = require_visibility::<Book>(None);

        assert!(matches!(visibility, Err(DatabaseError::NotFound { .. })));
    }

    #[test]
    fn an_unknown_visibility_is_reported_as_a_corrupted_invariant() {
        let visibility = require_visibility::<Book>(Some("Unlisted".to_owned()));

        assert!(matches!(
            visibility,
            Err(DatabaseError::InvariantCorrupted {
                field: "visibility",
                ..
            })
        ));
    }

    #[test]
    fn a_stored_visibility_is_parsed() {
        let visibility = require_visibility::<Book>(Some("Listed".to_owned()));

        assert!(matches!(visibility, Ok(BookVisibility::Listed)));
    }
}
