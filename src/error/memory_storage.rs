use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

use crate::error::{EntityError, LogInternal, error_response};

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum MemoryStoreError {
    #[error("Redis command failed")]
    Redis(#[from] redis::RedisError),

    #[error("Failed to (de)serialize a session blob")]
    Serde(#[source] serde_json::Error),

    #[error("Stored session id is not a uuid")]
    CorruptSid(#[source] uuid::Error),

    #[error("Stored session jti is not a well-formed hash")]
    CorruptJti(#[source] EntityError),
}

impl LogInternal for MemoryStoreError {
    const MODULE: &'static str = "memory store";
}

impl IntoResponse for MemoryStoreError {
    fn into_response(self) -> Response {
        error_response(StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
    }
}
