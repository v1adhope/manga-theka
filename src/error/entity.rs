use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum EntityError {
    #[error("'{0}' can't be empty or whitespace")]
    NameIsEmptyOrWhitespace(String),

    #[error("'{0}' exceeds character limit")]
    NameExceedsCharLimit(String),

    #[error("'{0}' can only contain letters")]
    NameContainsNotLetters(String),

    #[error("'{0}' invalid creator role")]
    InvalidCreatorRole(String),

    #[error("'{0}' invalid label kind")]
    InvalidLabelKind(String),

    #[error("'{0}' invalid book status")]
    InvalidBookStatus(String),

    #[error("'{0}' invalid book type")]
    InvalidBookKind(String),

    #[error("'{0}' invalid book link type")]
    InvalidBookLinkKind(String),

    #[error("Description exceeds the 2000-character limit")]
    DescriptionExceedsCharLimit,

    #[error("Link url exceeds the 2048-character limit")]
    LinkUrlExceedsCharLimit,

    #[error("'{0}' must be an http or https url")]
    LinkUrlSchemeNotAllowed(String),

    #[error("{0} alternative titles exceed the limit of {1}")]
    TitlesExceedLimit(usize, usize),

    #[error("Limit {0} is out of range [1, {1}]")]
    LimitOutOfRange(u32, u32),
}

impl IntoResponse for EntityError {
    fn into_response(self) -> Response {
        (StatusCode::UNPROCESSABLE_ENTITY, self.to_string()).into_response()
    }
}
