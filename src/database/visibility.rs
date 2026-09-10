use tracing::instrument;
use uuid::Uuid;

use crate::{
    database::{Database, Invariant},
    entity::{
        Book, BookAccess, BookCover, BookVisibilityUpdate, Chapter, ChapterPage, ChapterRelease,
        Entity, ReleaseAccess,
    },
    error::{DatabaseError, LogInternal},
};

fn require_access<T: Entity>(row: Option<(String, Uuid)>) -> Result<BookAccess, DatabaseError> {
    let Some((visibility, created_by)) = row else {
        return Err(DatabaseError::not_found::<T>());
    };

    let visibility = visibility
        .parse()
        .or_corrupted("visibility")
        .inspect_err(DatabaseError::log_internal)?;

    Ok(BookAccess {
        visibility,
        created_by,
    })
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
        sqlx::query_file!("queries/book_visibility.sql", id)
            .fetch_optional(&self.pool)
            .await
            .map(|row| row.map(|r| (r.visibility, r.created_by)))
            .map_err(DatabaseError::from)
            .inspect_err(DatabaseError::log_internal)
            .and_then(require_access::<Book>)
    }

    #[instrument(name = "db.book_cover.access", skip_all, fields(cover.id = %id))]
    pub async fn get_book_access_by_cover(&self, id: Uuid) -> Result<BookAccess, DatabaseError> {
        sqlx::query_file!("queries/book_visibility_by_cover.sql", id)
            .fetch_optional(&self.pool)
            .await
            .map(|row| row.map(|r| (r.visibility, r.created_by)))
            .map_err(DatabaseError::from)
            .inspect_err(DatabaseError::log_internal)
            .and_then(require_access::<BookCover>)
    }

    #[instrument(name = "db.chapter.access", skip_all, fields(chapter.id = %id))]
    pub async fn get_book_access_by_chapter(&self, id: Uuid) -> Result<BookAccess, DatabaseError> {
        sqlx::query_file!("queries/book_visibility_by_chapter.sql", id)
            .fetch_optional(&self.pool)
            .await
            .map(|row| row.map(|r| (r.visibility, r.created_by)))
            .map_err(DatabaseError::from)
            .inspect_err(DatabaseError::log_internal)
            .and_then(require_access::<Chapter>)
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

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::{
        database::visibility::require_access,
        entity::{Book, BookVisibility},
        error::DatabaseError,
    };

    #[test]
    fn a_missing_row_is_reported_as_not_found() {
        let access = require_access::<Book>(None);

        assert!(matches!(access, Err(DatabaseError::NotFound { .. })));
    }

    #[test]
    fn an_unknown_visibility_is_reported_as_a_corrupted_invariant() {
        let access = require_access::<Book>(Some(("Unlisted".to_owned(), Uuid::now_v7())));

        assert!(matches!(
            access,
            Err(DatabaseError::InvariantCorrupted {
                field: "visibility",
                ..
            })
        ));
    }

    #[test]
    fn a_stored_row_is_parsed_into_access() {
        let owner = Uuid::now_v7();
        let access = require_access::<Book>(Some(("Listed".to_owned(), owner)));

        let access = access.expect("a well-formed row must parse");
        assert_eq!(access.visibility, BookVisibility::Listed);
        assert_eq!(access.created_by, owner);
    }
}
