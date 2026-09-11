use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

use crate::error::{LogInternal, error_response};

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

impl LogInternal for ObjectStorageError {
    const MODULE: &'static str = "object storage";
}

impl IntoResponse for ObjectStorageError {
    fn into_response(self) -> Response {
        error_response(StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
    }
}
