use crate::{entity::ContentRating, error::ServiceError, service::Service};

impl Service {
    pub async fn get_content_ratings(&self) -> Result<Vec<ContentRating>, ServiceError> {
        self.database
            .get_content_ratings()
            .await
            .map_err(Into::into)
    }
}
