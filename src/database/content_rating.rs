use crate::{database::Database, entity::ContentRating, error::DatabaseError};

impl Database {
    pub async fn get_content_ratings(&self) -> Result<Vec<ContentRating>, DatabaseError> {
        sqlx::query_file_as!(ContentRating, "queries/get_content_ratings.sql")
            .fetch_all(&self.pool)
            .await
            .map_err(DatabaseError::from)
            .inspect_err(|e| {
                if DatabaseError::is_internal(e) {
                    tracing::error!("failed to get content ratings from database: {e:?}");
                }
            })
    }
}
