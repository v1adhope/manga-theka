use crate::{entity::Language, error::ServiceError, service::Service};

impl Service {
    pub async fn get_languages(&self) -> Result<Vec<Language>, ServiceError> {
        self.database.get_languages().await.map_err(Into::into)
    }
}
