mod database;
mod entity;
mod object_storage;
mod route;
mod service;

pub use database::*;
pub use entity::*;
pub use object_storage::*;
pub use route::*;
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
    RouteError(#[from] RouteError),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            Self::EntityError(e) => e.into_response(),
            Self::ServiceError(e) => e.into_response(),
            Self::RouteError(e) => e.into_response(),
        }
    }
}
