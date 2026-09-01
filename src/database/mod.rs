mod book;
mod chapter;
mod content_rating;
mod creator;
mod feedback;
mod label;
mod language;
mod release;

use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    config,
    entity::{BookVisibility, Entity, SortOrder},
    error::DatabaseError,
};

trait Invariant<T> {
    fn or_corrupted(self, field: &'static str) -> Result<T, DatabaseError>;
}

impl<T, E: std::error::Error> Invariant<T> for Result<T, E> {
    fn or_corrupted(self, field: &'static str) -> Result<T, DatabaseError> {
        self.map_err(|e| DatabaseError::invariant_corrupted(field, e))
    }
}

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

fn cursor_op(order: SortOrder) -> (&'static str, &'static str) {
    match order {
        SortOrder::Asc => (">", "asc"),
        SortOrder::Desc => ("<", "desc"),
    }
}

fn fetch_limit(limit: u32) -> i64 {
    i64::from(limit) + 1
}

fn take_page<R>(rows: &mut Vec<R>, limit: u32, cursor_of: impl Fn(&R) -> Uuid) -> Option<Uuid> {
    if rows.len() <= limit as usize {
        return None;
    }

    rows.pop();
    rows.last().map(cursor_of)
}

pub async fn pool(cfg: &config::Database) -> PgPool {
    PgPool::connect_with(cfg.with_db())
        .await
        .expect("failed to connect to Postgres")
}

#[derive(Debug, Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn migrate(&self) {
        sqlx::migrate!()
            .run(&self.pool)
            .await
            .expect("failed to migrate the database");
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::{
        database::{cursor_op, fetch_limit, require_visibility, take_page},
        entity::{Book, BookVisibility, SortOrder},
        error::DatabaseError,
    };

    struct Row {
        id: Uuid,
    }

    fn rows(n: u128) -> Vec<Row> {
        (1..=n)
            .map(|i| Row {
                id: Uuid::from_u128(i),
            })
            .collect()
    }

    #[test]
    fn ascending_order_compares_forward() {
        assert_eq!(cursor_op(SortOrder::Asc), (">", "asc"));
    }

    #[test]
    fn descending_order_compares_backward() {
        assert_eq!(cursor_op(SortOrder::Desc), ("<", "desc"));
    }

    #[test]
    fn fetch_limit_asks_for_one_extra_row() {
        assert_eq!(fetch_limit(0), 1);
        assert_eq!(fetch_limit(1), 2);
        assert_eq!(fetch_limit(u32::MAX), i64::from(u32::MAX) + 1);
    }

    #[test]
    fn page_shorter_than_limit_has_no_next_cursor() {
        let mut rows = rows(2);

        let next_cursor = take_page(&mut rows, 3, |r| r.id);

        assert_eq!(next_cursor, None);
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn full_page_without_an_extra_row_has_no_next_cursor() {
        let mut rows = rows(3);

        let next_cursor = take_page(&mut rows, 3, |r| r.id);

        assert_eq!(next_cursor, None);
        assert_eq!(rows.len(), 3);
    }

    #[test]
    fn extra_row_is_dropped_and_the_last_kept_row_is_the_next_cursor() {
        let mut rows = rows(4);

        let next_cursor = take_page(&mut rows, 3, |r| r.id);

        assert_eq!(next_cursor, Some(Uuid::from_u128(3)));
        assert_eq!(rows.len(), 3);
        assert_eq!(rows.last().unwrap().id, Uuid::from_u128(3));
    }

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

    #[test]
    fn empty_page_has_no_next_cursor() {
        let mut rows = rows(1);

        let next_cursor = take_page(&mut rows, 0, |r| r.id);

        assert_eq!(next_cursor, None);
        assert!(rows.is_empty());
    }
}
