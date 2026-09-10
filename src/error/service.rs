use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

use crate::error::{
    DatabaseError, EntityError, HasherError, JwtError, MemoryStoreError, ObjectStorageError,
    error_response,
};

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ServiceError {
    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error(transparent)]
    DatabaseError(#[from] DatabaseError),

    #[error(transparent)]
    ObjectStorageError(#[from] ObjectStorageError),

    #[error(transparent)]
    EntityError(#[from] EntityError),

    #[error(transparent)]
    HasherError(#[from] HasherError),

    #[error(transparent)]
    JwtError(#[from] JwtError),

    #[error(transparent)]
    MemoryStoreError(#[from] MemoryStoreError),
}

impl IntoResponse for ServiceError {
    fn into_response(self) -> Response {
        match self {
            Self::InvalidCredentials => error_response(StatusCode::UNAUTHORIZED, self.to_string()),
            Self::DatabaseError(e) => e.into_response(),
            Self::ObjectStorageError(e) => e.into_response(),
            Self::EntityError(e) => e.into_response(),
            Self::HasherError(e) => e.into_response(),
            Self::JwtError(e) => e.into_response(),
            Self::MemoryStoreError(e) => e.into_response(),
        }
    }
}
