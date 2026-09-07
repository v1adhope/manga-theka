use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

use crate::error::error_response;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum MemoryStoreError {
    #[error("Redis command failed")]
    Redis(#[from] redis::RedisError),

    #[error("Failed to (de)serialize a session blob")]
    Serde(#[source] serde_json::Error),

    #[error("Stored session id is not a uuid")]
    CorruptSid(#[source] uuid::Error),
}

impl MemoryStoreError {
    pub fn log_internal(&self) {
        tracing::error!(error = ?self, "internal memory store error");
    }
}

impl IntoResponse for MemoryStoreError {
    fn into_response(self) -> Response {
        error_response(StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
    }
}
