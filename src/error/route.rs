use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

use crate::error::error_response;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum RouteError {
    #[error(transparent)]
    MultipartError(#[from] axum::extract::multipart::MultipartError),

    #[error("Request is missing an image part")]
    ImagePartMissing,

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Forbidden")]
    Forbidden,
}

impl IntoResponse for RouteError {
    fn into_response(self) -> Response {
        match self {
            Self::MultipartError(e) => error_response(e.status(), e.to_string()),
            Self::ImagePartMissing => {
                error_response(StatusCode::UNPROCESSABLE_ENTITY, self.to_string())
            }
            Self::InvalidCredentials => error_response(StatusCode::UNAUTHORIZED, self.to_string()),
            Self::Forbidden => error_response(StatusCode::FORBIDDEN, self.to_string()),
        }
    }
}
