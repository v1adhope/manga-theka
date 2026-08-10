use crate::{
    entity::{Label, LabelKind},
    error::ServiceError,
    service::Service,
};

impl Service {
    pub async fn get_labels(
        &self,
        label_kind: Option<LabelKind>,
    ) -> Result<Vec<Label>, ServiceError> {
        self.database
            .get_labels(label_kind)
            .await
            .map_err(Into::into)
    }
}
