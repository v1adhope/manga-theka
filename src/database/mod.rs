mod book;
mod content_rating;
mod creator;
mod label;
mod language;

#[derive(Debug, Clone)]
pub struct Database {
    pool: sqlx::PgPool,
}

// TODO: tune tracing (internal errors handling)
impl Database {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
}
