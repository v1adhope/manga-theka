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

use crate::{config, entity::SortOrder};

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
