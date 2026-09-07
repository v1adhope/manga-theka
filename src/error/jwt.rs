use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

use crate::error::error_response;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum JwtError {
    #[error("Failed to sign a token")]
    Sign(#[source] jsonwebtoken::errors::Error),

    #[error("Invalid credentials")]
    Verify(#[source] jsonwebtoken::errors::Error),

    #[error("Invalid credentials")]
    WrongTokenClass,
}

impl JwtError {
    pub fn log_internal(&self) {
        tracing::error!(error = ?self, "internal jwt error");
    }
}

impl IntoResponse for JwtError {
    fn into_response(self) -> Response {
        let status = match self {
            Self::Sign(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Verify(_) | Self::WrongTokenClass => StatusCode::UNAUTHORIZED,
        };

        error_response(status, self.to_string())
    }
}
