mod database;
mod entity;
mod object_storage;
mod service;

pub use database::*;
pub use entity::*;
pub use object_storage::*;
pub use service::*;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

const INTERNAL_MESSAGE: &str = "Something went wrong";

pub fn error_response(status: StatusCode, message: String) -> Response {
    if status.is_server_error() {
        return (status, INTERNAL_MESSAGE.to_string()).into_response();
    }

    (status, message).into_response()
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AppError {
    #[error(transparent)]
    EntityError(#[from] EntityError),

    #[error(transparent)]
    ServiceError(#[from] ServiceError),

    #[error(transparent)]
    MultipartError(#[from] axum::extract::multipart::MultipartError),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            Self::EntityError(e) => e.into_response(),
            Self::ServiceError(e) => e.into_response(),
            Self::MultipartError(e) => error_response(e.status(), e.to_string()),
        }
    }
}
