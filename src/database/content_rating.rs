use crate::{
    database::Database,
    entity::ContentRating,
    error::{DatabaseError, LogInternal},
};

impl Database {
    #[tracing::instrument(name = "db.content_rating.list", skip_all)]
    pub async fn get_content_ratings(&self) -> Result<Vec<ContentRating>, DatabaseError> {
        sqlx::query_file_as!(ContentRating, "queries/get_content_ratings.sql")
            .fetch_all(&self.pool)
            .await
            .map_err(DatabaseError::from)
            .inspect_err(DatabaseError::log_internal)
    }
}
