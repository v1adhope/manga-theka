mod book;
mod chapter;
mod content_rating;
mod creator;
mod feedback;
mod label;
mod language;
mod release;

use sqlx::PgPool;

use crate::config;

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
