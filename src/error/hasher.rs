use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

use crate::error::{EntityError, LogInternal, error_response};

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum HasherError {
    #[error("Failed to serialize the hash target")]
    Serialize(#[source] serde_json::Error),

    #[error("Digest is not a well-formed hash")]
    Digest(#[source] EntityError),

    #[error("Failed to build the Argon2 parameters")]
    Params(#[source] argon2::Error),

    #[error("Failed to hash the password")]
    HashPassword(#[source] argon2::password_hash::Error),

    #[error("Invalid credentials")]
    PasswordMismatch(#[source] anyhow::Error),

    #[error("Failed to verify the password")]
    VerifyPassword(#[source] argon2::password_hash::Error),

    #[error("Argon2 produced a PHC string outside the storage bound")]
    Phc(#[source] EntityError),
}

impl LogInternal for HasherError {
    const MODULE: &'static str = "hasher";
}

impl IntoResponse for HasherError {
    fn into_response(self) -> Response {
        let status = match self {
            Self::PasswordMismatch(_) => StatusCode::UNAUTHORIZED,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        error_response(status, self.to_string())
    }
}
