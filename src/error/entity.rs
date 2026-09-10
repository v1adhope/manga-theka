use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    entity::{BookVisibility, Entity},
    error::error_response,
};

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum EntityError {
    #[error("Name can't be empty or whitespace")]
    NameIsEmptyOrWhitespace,

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

    #[error("'{0}' is not a valid publication demographic")]
    InvalidPublicationDemographic(String),

    #[error("'{0}' is not a valid book visibility")]
    InvalidBookVisibility(String),

    #[error("'{0}' is not a valid role")]
    InvalidRole(String),

    #[error("A note is required to move a book to {0}")]
    BookNoteRequired(BookVisibility),

    #[error("A book can't move from {0} to {1}")]
    IllegalVisibilityTransition(BookVisibility, BookVisibility),

    #[error("A book and its contents can't be changed while {0}")]
    BookNotWritable(BookVisibility),

    #[error("{entity} not found")]
    NotReadable { entity: &'static str },

    #[error("'{0}' is not a valid book link kind")]
    InvalidBookLinkKind(String),

    #[error("'{0}' is not a valid image extension")]
    InvalidImageExtension(String),

    #[error("Image must be JPEG, PNG, or WebP")]
    UnsupportedImageFormat,

    #[error("Text can't be empty or whitespace")]
    TextIsEmptyOrWhitespace,

    #[error("Text exceeds the {0}-character limit")]
    TextExceedsCharLimit(usize),

    #[error("Email is not a well-formed address")]
    EmailIsMalformed,

    #[error("Email exceeds the 254-character limit")]
    EmailExceedsCharLimit,

    #[error("Username must be 3 to 32 characters of letters, digits, or underscores")]
    UsernameIsMalformed,

    #[error("Password must be between {0} and {1} characters")]
    PasswordLengthOutOfRange(usize, usize),

    #[error("Password hash exceeds the {0}-character limit")]
    PasswordHashExceedsCharLimit(usize),

    #[error("A session younger than 24 hours can't revoke other sessions")]
    SessionTooNewToRevoke,

    #[error("'{0}' is not a valid feedback kind")]
    InvalidFeedbackKind(String),

    #[error("'{0}' is not a valid feedback status")]
    InvalidFeedbackStatus(String),

    #[error("Link url '{0}...' exceeds the 2048-character limit")]
    LinkUrlExceedsCharLimit(String),

    #[error("'{1}' is not a well-formed url")]
    LinkUrlIsMalformed(#[source] url::ParseError, String),

    #[error("'{0}' must be an http or https url")]
    LinkUrlSchemeNotAllowed(String),

    #[error("Limit {0} is out of range [1, {1}]")]
    LimitOutOfRange(i64, i64),

    #[error("The {0} range starts after it ends")]
    RangeIsInverted(&'static str),

    #[error("Cursor was issued for a different selection")]
    CursorSelectionMismatch,

    #[error("Hash must be exactly {0} lowercase hex characters")]
    HexHashIsMalformed(usize),

    #[error("Chapter number {0} must be within [{1}, {2}] with at most {3} decimal places")]
    ChapterNumberOutOfRange(f32, f32, f32, i32),

    #[error("Chapter volume {0} must be within [{1}, {2}]")]
    ChapterVolumeOutOfRange(i16, i16, i16),

    #[error("Ordinal {0} must be {1} or greater")]
    OrdinalOutOfRange(i32, i32),

    #[error("Page order can't be empty")]
    PageOrderIsEmpty,

    #[error("Page '{0}' is declared more than once")]
    PageOrderHasDuplicates(Uuid),

    #[error("Upload of {0} parts exceeds the {1}-part limit")]
    UploadPartsExceedLimit(usize, usize),

    #[error("Upload must contain at least one image")]
    ImagesEmpty,

    #[error("Release already holds {0} of {1} allowed pages")]
    ReleaseRowsExceedLimit(usize, usize),

    #[error("{0}: {1} items exceed the {2}-item limit")]
    CollectionExceedsLimit(&'static str, usize, usize),

    #[error("Image exceeds the {0}-byte limit")]
    ImageExceedsByteLimit(usize),

    #[error("Multipart part is missing a file name")]
    FileNameMissing,
}

impl EntityError {
    pub fn not_readable<T: Entity>() -> Self {
        Self::NotReadable { entity: T::NAME }
    }
}

impl IntoResponse for EntityError {
    fn into_response(self) -> Response {
        let status = match self {
            Self::UnsupportedImageFormat => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            Self::ImageExceedsByteLimit(_) => StatusCode::PAYLOAD_TOO_LARGE,
            Self::IllegalVisibilityTransition(_, _)
            | Self::BookNotWritable(_)
            | Self::SessionTooNewToRevoke => StatusCode::CONFLICT,
            Self::NotReadable { .. } => StatusCode::NOT_FOUND,
            Self::CursorSelectionMismatch => StatusCode::BAD_REQUEST,
            _ => StatusCode::UNPROCESSABLE_ENTITY,
        };

        error_response(status, self.to_string())
    }
}
