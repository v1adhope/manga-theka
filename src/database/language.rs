use crate::{database::Database, entity::Language, error::DatabaseError};

impl Database {
    #[tracing::instrument(name = "db.language.list", skip_all)]
    pub async fn get_languages(&self) -> Result<Vec<Language>, DatabaseError> {
        sqlx::query_file_as!(Language, "queries/get_languages.sql")
            .fetch_all(&self.pool)
            .await
            .map_err(DatabaseError::from)
            .inspect_err(DatabaseError::log_internal)
    }
}
