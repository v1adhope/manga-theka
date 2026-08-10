use crate::{
    entity::{Label, LabelType},
    error::ServiceError,
    service::Service,
};

impl Service {
    pub async fn get_labels(
        &self,
        label_type: Option<LabelType>,
    ) -> Result<Vec<Label>, ServiceError> {
        self.database
            .get_labels(label_type)
            .await
            .map_err(Into::into)
    }
}
