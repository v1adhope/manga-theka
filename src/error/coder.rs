use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

use crate::error::{EntityError, LogInternal, error_response};

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum CoderError {
    #[error("Failed to serialize the cursor")]
    Serialize(#[source] serde_json::Error),

    #[error("Cursor is not valid base64")]
    Base64(#[source] base64::DecodeError),

    #[error("Cursor payload is malformed")]
    Deserialize(#[source] serde_json::Error),

    #[error("Cursor payload is not a valid cursor")]
    Cursor(#[source] EntityError),
}

impl LogInternal for CoderError {
    const MODULE: &'static str = "coder";
}

impl IntoResponse for CoderError {
    fn into_response(self) -> Response {
        let status = match self {
            Self::Serialize(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Base64(_) | Self::Deserialize(_) | Self::Cursor(_) => StatusCode::BAD_REQUEST,
        };

        error_response(status, self.to_string())
    }
}
