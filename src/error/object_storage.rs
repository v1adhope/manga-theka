use aws_sdk_s3::{
    error::SdkError,
    operation::{
        delete_object::DeleteObjectError, get_object::GetObjectError, put_object::PutObjectError,
    },
    presigning::PresigningConfigError,
};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ObjectStorageError {
    #[error("Failed to upload the object")]
    Upload(#[source] Box<SdkError<PutObjectError>>),

    #[error("Failed to build the presigning config")]
    Presigning(#[source] PresigningConfigError),

    #[error("Failed to presign the object url")]
    Presign(#[source] Box<SdkError<GetObjectError>>),

    #[error("Failed to delete the object")]
    Delete(#[source] Box<SdkError<DeleteObjectError>>),
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
