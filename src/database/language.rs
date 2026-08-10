use crate::{database::Database, entity::Language, error::DatabaseError};

impl Database {
    pub async fn get_languages(&self) -> Result<Vec<Language>, DatabaseError> {
        sqlx::query_file_as!(Language, "queries/get_languages.sql")
            .fetch_all(&self.pool)
            .await
            .map_err(DatabaseError::from)
            .inspect_err(|e| {
                if DatabaseError::is_internal(e) {
                    tracing::error!("failed to get languages from database: {e:?}");
                }
            })
    }
}
