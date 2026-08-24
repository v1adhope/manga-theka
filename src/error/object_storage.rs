use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

use crate::error::error_response;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ObjectStorageError {
    #[error("Failed to upload the object")]
    Upload(#[source] anyhow::Error),

    #[error("Failed to presign the object url")]
    Presign(#[source] anyhow::Error),

    #[error("Failed to delete the object")]
    Delete(#[source] anyhow::Error),
}

impl ObjectStorageError {
    pub fn log_internal(&self) {
        tracing::error!(error = ?self, "internal object storage error");
    }
}

impl IntoResponse for ObjectStorageError {
    fn into_response(self) -> Response {
        error_response(StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
    }
}
