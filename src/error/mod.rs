mod database;
mod entity;
mod service;

pub use database::*;
pub use entity::*;
pub use service::*;

use axum::response::{IntoResponse, Response};
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AppError {
    #[error(transparent)]
    EntityError(#[from] EntityError),

    #[error(transparent)]
    ServiceError(#[from] ServiceError),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            Self::EntityError(e) => e.into_response(),
            Self::ServiceError(e) => e.into_response(),
        }
    }
}
