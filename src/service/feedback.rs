use uuid::Uuid;

use crate::{
    entity::{Feedback, FeedbackFilter, FeedbackStatusUpdate},
    error::ServiceError,
    service::Service,
};

impl Service {
    pub async fn store_feedback(&self, item: Feedback) -> Result<(), ServiceError> {
        self.database
            .store_feedback(&item)
            .await
            .map_err(Into::into)
    }

    pub async fn get_feedback(&self, id: Uuid) -> Result<Feedback, ServiceError> {
        self.database.get_feedback(id).await.map_err(Into::into)
    }

    pub async fn get_feedbacks(
        &self,
        filter: FeedbackFilter,
    ) -> Result<(Vec<Feedback>, Option<Uuid>), ServiceError> {
        self.database
            .get_feedbacks(&filter)
            .await
            .map_err(Into::into)
    }

    pub async fn set_feedback_status(
        &self,
        item: FeedbackStatusUpdate,
    ) -> Result<(), ServiceError> {
        self.database
            .set_feedback_status(&item)
            .await
            .map_err(Into::into)
    }
}
