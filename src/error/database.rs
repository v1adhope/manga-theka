use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum DatabaseError {
    #[error("Creator first name exceeds the 255-character limit")]
    CreatorFirstNameTooLong(#[source] sqlx::Error),

    #[error("Creator last name exceeds the 255-character limit")]
    CreatorLastNameTooLong(#[source] sqlx::Error),

    #[error("Creator role doesn't exist")]
    CreatorRoleDoesNotExist(#[source] sqlx::Error),

    #[error("Creator full name already exists")]
    CreatorFullNameDuplication(#[source] sqlx::Error),

    #[error("Creator is attached to one or more books")]
    CreatorInUse(#[source] sqlx::Error),

    #[error("Creator not found")]
    CreatorNotFound,

    #[error("Book name exceeds the 255-character limit")]
    BookNameTooLong(#[source] sqlx::Error),

    #[error("Book description exceeds the 2000-character limit")]
    BookDescriptionTooLong(#[source] sqlx::Error),

    #[error("Book status doesn't exist")]
    BookStatusDoesNotExist(#[source] sqlx::Error),

    #[error("Book kind doesn't exist")]
    BookKindDoesNotExist(#[source] sqlx::Error),

    #[error("Book content rating doesn't exist")]
    BookContentRatingDoesNotExist(#[source] sqlx::Error),

    #[error("Book publication language doesn't exist")]
    BookPublicationLanguageDoesNotExist(#[source] sqlx::Error),

    #[error("Book label doesn't exist")]
    BookLabelDoesNotExist(#[source] sqlx::Error),

    #[error("Book label is attached more than once")]
    BookLabelDuplication(#[source] sqlx::Error),

    #[error("Book link kind doesn't exist")]
    BookLinkKindDoesNotExist(#[source] sqlx::Error),

    #[error("Book link url exceeds the 2048-character limit")]
    BookLinkUrlTooLong(#[source] sqlx::Error),

    #[error("Book link url is attached more than once")]
    BookLinkUrlDuplication(#[source] sqlx::Error),

    #[error("Alternative title language doesn't exist")]
    BookTitleLanguageDoesNotExist(#[source] sqlx::Error),

    #[error("Alternative title exceeds the 255-character limit")]
    BookTitleNameTooLong(#[source] sqlx::Error),

    #[error("Alternative title is attached more than once")]
    BookTitleNameDuplication(#[source] sqlx::Error),

    #[error("Book not found")]
    BookNotFound,

    #[error("Book cover extension doesn't exist")]
    BookCoverExtensionDoesNotExist(#[source] sqlx::Error),

    #[error("Another cover was promoted concurrently")]
    BookCoverMainConflict(#[source] sqlx::Error),

    #[error("Book cover not found")]
    BookCoverNotFound,

    #[error("Chapter name exceeds the 255-character limit")]
    ChapterNameTooLong(#[source] sqlx::Error),

    #[error("Chapter number is out of range")]
    ChapterNumberOutOfRange(#[source] sqlx::Error),

    #[error("Chapter volume is out of range")]
    ChapterVolumeOutOfRange(#[source] sqlx::Error),

    #[error("Chapter number already exists in this book")]
    ChapterNumberDuplication(#[source] sqlx::Error),

    #[error("Chapter localization language doesn't exist")]
    ChapterLocalizationLanguageDoesNotExist(#[source] sqlx::Error),

    #[error("Chapter localization exceeds the 255-character limit")]
    ChapterLocalizationNameTooLong(#[source] sqlx::Error),

    #[error("Chapter localization language is attached more than once")]
    ChapterLocalizationDuplication(#[source] sqlx::Error),

    #[error("Chapter not found")]
    ChapterNotFound,

    #[error("Database invariant corrupted on field '{field}': {message}")]
    InvariantCorrupted { field: String, message: String },

    #[error("Unknown database error")]
    Unknown(#[source] sqlx::Error),
}

impl DatabaseError {
    pub fn log_internal(&self) {
        if matches!(self, Self::Unknown(_) | Self::InvariantCorrupted { .. }) {
            tracing::error!(error = ?self, "internal database error");
        }
    }

    pub fn invariant_corrupted(field: &str, message: impl std::error::Error) -> Self {
        Self::InvariantCorrupted {
            field: field.to_string(),
            message: message.to_string(),
        }
    }
}

impl From<sqlx::Error> for DatabaseError {
    fn from(err: sqlx::Error) -> Self {
        if let Some(db_err) = err.as_database_error() {
            match db_err.constraint() {
                Some("check_length_creators_first_name") => {
                    return Self::CreatorFirstNameTooLong(err);
                }
                Some("check_length_creators_last_name") => {
                    return Self::CreatorLastNameTooLong(err);
                }
                Some("enum_creators_role") => {
                    return Self::CreatorRoleDoesNotExist(err);
                }
                Some("unique_creators_first_name_last_name") => {
                    return Self::CreatorFullNameDuplication(err);
                }
                Some("fk_book_creators_creators_creator_id") => {
                    return Self::CreatorInUse(err);
                }
                Some("check_length_books_name") => {
                    return Self::BookNameTooLong(err);
                }
                Some("check_length_books_description") => {
                    return Self::BookDescriptionTooLong(err);
                }
                Some("enum_books_status") => {
                    return Self::BookStatusDoesNotExist(err);
                }
                Some("enum_books_kind") => {
                    return Self::BookKindDoesNotExist(err);
                }
                Some("fk_books_content_ratings_content_rating") => {
                    return Self::BookContentRatingDoesNotExist(err);
                }
                Some("fk_books_languages_publication_language") => {
                    return Self::BookPublicationLanguageDoesNotExist(err);
                }
                Some("fk_book_labels_labels_label_id") => {
                    return Self::BookLabelDoesNotExist(err);
                }
                Some("pk_book_labels_book_id_label_id") => {
                    return Self::BookLabelDuplication(err);
                }
                Some("enum_book_links_kind") => {
                    return Self::BookLinkKindDoesNotExist(err);
                }
                Some("check_length_book_links_url") => {
                    return Self::BookLinkUrlTooLong(err);
                }
                Some("pk_book_links_book_id_link_hash") => {
                    return Self::BookLinkUrlDuplication(err);
                }
                Some("fk_book_titles_languages_language_id") => {
                    return Self::BookTitleLanguageDoesNotExist(err);
                }
                Some("check_length_book_titles_name") => {
                    return Self::BookTitleNameTooLong(err);
                }
                Some("pk_book_titles_book_id_language_id_name") => {
                    return Self::BookTitleNameDuplication(err);
                }
                Some("enum_book_covers_extension") => {
                    return Self::BookCoverExtensionDoesNotExist(err);
                }
                Some("unique_book_covers_book_id_is_main") => {
                    return Self::BookCoverMainConflict(err);
                }
                Some("fk_book_covers_books_book_id") | Some("fk_chapters_books_book_id") => {
                    return Self::BookNotFound;
                }
                Some("check_length_chapters_name") => {
                    return Self::ChapterNameTooLong(err);
                }
                Some("check_range_chapters_number") | Some("check_scale_chapters_number") => {
                    return Self::ChapterNumberOutOfRange(err);
                }
                Some("check_range_chapters_volume") => {
                    return Self::ChapterVolumeOutOfRange(err);
                }
                Some("unique_chapters_book_id_number") => {
                    return Self::ChapterNumberDuplication(err);
                }
                Some("fk_chapter_localizations_languages_language_id") => {
                    return Self::ChapterLocalizationLanguageDoesNotExist(err);
                }
                Some("check_length_chapter_localizations_name") => {
                    return Self::ChapterLocalizationNameTooLong(err);
                }
                Some("pk_chapter_localizations_chapter_id_language_id") => {
                    return Self::ChapterLocalizationDuplication(err);
                }
                _ => {}
            }
        }
        Self::Unknown(err)
    }
}

impl IntoResponse for DatabaseError {
    fn into_response(self) -> Response {
        match self {
            Self::Unknown(_) | Self::InvariantCorrupted { .. } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Something went wrong".to_string(),
            ),
            Self::CreatorNotFound
            | Self::BookNotFound
            | Self::BookCoverNotFound
            | Self::ChapterNotFound => (StatusCode::NOT_FOUND, self.to_string()),
            Self::CreatorInUse(_)
            | Self::CreatorFullNameDuplication(_)
            | Self::BookCoverMainConflict(_)
            | Self::ChapterNumberDuplication(_) => (StatusCode::CONFLICT, self.to_string()),
            Self::CreatorFirstNameTooLong(_)
            | Self::CreatorLastNameTooLong(_)
            | Self::CreatorRoleDoesNotExist(_)
            | Self::BookNameTooLong(_)
            | Self::BookDescriptionTooLong(_)
            | Self::BookStatusDoesNotExist(_)
            | Self::BookKindDoesNotExist(_)
            | Self::BookContentRatingDoesNotExist(_)
            | Self::BookPublicationLanguageDoesNotExist(_)
            | Self::BookLabelDoesNotExist(_)
            | Self::BookLabelDuplication(_)
            | Self::BookLinkKindDoesNotExist(_)
            | Self::BookLinkUrlTooLong(_)
            | Self::BookLinkUrlDuplication(_)
            | Self::BookTitleLanguageDoesNotExist(_)
            | Self::BookTitleNameTooLong(_)
            | Self::BookTitleNameDuplication(_)
            | Self::BookCoverExtensionDoesNotExist(_)
            | Self::ChapterNameTooLong(_)
            | Self::ChapterNumberOutOfRange(_)
            | Self::ChapterVolumeOutOfRange(_)
            | Self::ChapterLocalizationLanguageDoesNotExist(_)
            | Self::ChapterLocalizationNameTooLong(_)
            | Self::ChapterLocalizationDuplication(_) => {
                (StatusCode::UNPROCESSABLE_ENTITY, self.to_string())
            }
        }
        .into_response()
    }
}
