mod content_rating;
mod creator;
mod label;
mod language;

#[derive(Debug, Clone)]
pub struct Database {
    pub pool: sqlx::PgPool,
}

impl Database {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
}
