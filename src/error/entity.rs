use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum EntityError {
    #[error("'{0}' can't be empty or whitespaces")]
    NameIsEmptyOrWhitespaces(String),

    #[error("'{0}' exceeds character limit")]
    NameExceedsCharLimit(String),

    #[error("{0} can only contains letters")]
    NameContainsNotLetters(String),
}

impl IntoResponse for EntityError {
    fn into_response(self) -> Response {
        (StatusCode::UNPROCESSABLE_ENTITY, self.to_string()).into_response()
    }
}
