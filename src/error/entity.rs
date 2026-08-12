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

    #[error("'{0}' exceeds the 255-character limit")]
    NameExceedsCharLimit(String),

    #[error("'{0}' can only contain letters")]
    NameContainsNotLetters(String),

    #[error("'{0}' is not a valid creator role")]
    InvalidCreatorRole(String),

    #[error("'{0}' is not a valid label kind")]
    InvalidLabelKind(String),

    #[error("'{0}' is not a valid book status")]
    InvalidBookStatus(String),

    #[error("'{0}' is not a valid book kind")]
    InvalidBookKind(String),

    #[error("'{0}' is not a valid book link kind")]
    InvalidBookLinkKind(String),

    #[error("Description can't be empty or whitespace")]
    DescriptionIsEmptyOrWhitespace,

    #[error("Description exceeds the 2000-character limit")]
    DescriptionExceedsCharLimit,

    #[error("Link url '{0}...' exceeds the 2048-character limit")]
    LinkUrlExceedsCharLimit(String),

    #[error("'{1}' is not a well-formed url")]
    LinkUrlIsMalformed(#[source] url::ParseError, String),

    #[error("'{0}' must be an http or https url")]
    LinkUrlSchemeNotAllowed(String),

    #[error("Limit {0} is out of range [1, {1}]")]
    LimitOutOfRange(u32, u32),
}

impl IntoResponse for EntityError {
    fn into_response(self) -> Response {
        (StatusCode::UNPROCESSABLE_ENTITY, self.to_string()).into_response()
    }
}
