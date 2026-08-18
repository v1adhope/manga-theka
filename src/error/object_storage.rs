use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ObjectStorageError {
    #[error("Failed to upload the object")]
    Upload(#[source] anyhow::Error),

    #[error("Failed to build the presigning config")]
    Presigning(#[source] anyhow::Error),

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
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Something went wrong".to_string(),
        )
            .into_response()
    }
}
