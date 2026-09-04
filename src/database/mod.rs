mod book;
mod chapter;
mod content_rating;
mod creator;
mod feedback;
mod label;
mod language;
mod query;
mod release;
mod visibility;

use sqlx::PgPool;

use crate::{config, error::DatabaseError};

use query::{cursor_op, fetch_limit, take_page, upper_bound_op};

trait Invariant<T> {
    fn or_corrupted(self, field: &'static str) -> Result<T, DatabaseError>;
}

impl<T, E: std::error::Error> Invariant<T> for Result<T, E> {
    fn or_corrupted(self, field: &'static str) -> Result<T, DatabaseError> {
        self.map_err(|e| DatabaseError::invariant_corrupted(field, e))
    }
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
