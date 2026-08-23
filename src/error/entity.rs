use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;
use uuid::Uuid;

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

    #[error("'{0}' is not a valid book link kind")]
    InvalidBookLinkKind(String),

    #[error("'{0}' is not a valid cover extension")]
    InvalidCoverExtension(String),

    #[error("Image must be JPEG, PNG, or WebP")]
    UnsupportedImageFormat,

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

    #[error("Chapter number {0} must be within [{1}, {2}] with at most {3} decimal places")]
    ChapterNumberOutOfRange(f32, f32, f32, i32),

    #[error("Chapter volume {0} must be within [{1}, {2}]")]
    ChapterVolumeOutOfRange(i16, i16, i16),

    #[error("'{0}' is not a valid page extension")]
    InvalidPageExtension(String),

    #[error("Release version {0} can't be negative")]
    ReleaseVersionOutOfRange(i32),

    #[error("'{0}' is not a well-formed entity tag")]
    ReleaseVersionIsMalformed(String),

    #[error("If-Match is required to commit a release")]
    ReleaseVersionIsAbsent,

    #[error("Page order can't be empty")]
    PageOrderIsEmpty,

    #[error("Page order of {0} pages exceeds the {1}-page limit")]
    PageOrderExceedsLimit(usize, usize),

    #[error("Page '{0}' is declared more than once")]
    PageOrderHasDuplicates(Uuid),

    #[error("Upload of {0} parts exceeds the {1}-part limit")]
    UploadPartsExceedLimit(usize, usize),

    #[error("Release already holds {0} of {1} allowed pages")]
    ReleaseRowsExceedLimit(usize, usize),

    #[error("Page exceeds the {0}-byte limit")]
    PageExceedsByteLimit(usize),
}

impl IntoResponse for EntityError {
    fn into_response(self) -> Response {
        match self {
            Self::UnsupportedImageFormat => (StatusCode::UNSUPPORTED_MEDIA_TYPE, self.to_string()),
            Self::PageExceedsByteLimit(_) => (StatusCode::PAYLOAD_TOO_LARGE, self.to_string()),
            Self::ReleaseVersionIsAbsent => (StatusCode::PRECONDITION_REQUIRED, self.to_string()),
            Self::ReleaseVersionIsMalformed(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            _ => (StatusCode::UNPROCESSABLE_ENTITY, self.to_string()),
        }
        .into_response()
    }
}
