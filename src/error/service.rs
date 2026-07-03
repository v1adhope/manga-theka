use axum::response::{IntoResponse, Response};
use thiserror::Error;

use crate::error::DatabaseError;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ServiceError {
    #[error(transparent)]
    DatabaseError(#[from] DatabaseError),
}

impl IntoResponse for ServiceError {
    fn into_response(self) -> Response {
        match self {
            Self::DatabaseError(e) => e.into_response(),
        }
    }
}
