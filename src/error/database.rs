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

    #[error("Creator not found")]
    CreatorNotFound,

    #[error("Book name exceeds the 255-character limit")]
    BookNameTooLong(#[source] sqlx::Error),

    #[error("Book description exceeds the 2000-character limit")]
    BookDescriptionTooLong(#[source] sqlx::Error),

    #[error("Book status doesn't exist")]
    BookStatusDoesNotExist(#[source] sqlx::Error),

    #[error("Book type doesn't exist")]
    BookKindDoesNotExist(#[source] sqlx::Error),

    #[error("Book content rating doesn't exist")]
    BookContentRatingDoesNotExist(#[source] sqlx::Error),

    #[error("Book author doesn't exist")]
    BookAuthorDoesNotExist(#[source] sqlx::Error),

    #[error("Book artist doesn't exist")]
    BookArtistDoesNotExist(#[source] sqlx::Error),

    #[error("Book publication language doesn't exist")]
    BookPublicationLanguageDoesNotExist(#[source] sqlx::Error),

    #[error("Book label doesn't exist")]
    BookLabelDoesNotExist(#[source] sqlx::Error),

    #[error("Book label is attached more than once")]
    BookLabelDuplication(#[source] sqlx::Error),

    #[error("Book link type doesn't exist")]
    BookLinkKindDoesNotExist(#[source] sqlx::Error),

    #[error("Book link url exceeds the 2048-character limit")]
    BookLinkUrlTooLong(#[source] sqlx::Error),

    #[error("Alternative title language doesn't exist")]
    BookTitleLanguageDoesNotExist(#[source] sqlx::Error),

    #[error("Alternative title exceeds the 255-character limit")]
    BookTitleNameTooLong(#[source] sqlx::Error),

    #[error("Alternative titles exceed the limit of 12")]
    BookTitlesExceedLimit(#[source] sqlx::Error),

    #[error("Book not found")]
    BookNotFound,

    #[error("Database invariant corrupted on field '{field}': {message}")]
    InvariantCorrupted { field: String, message: String },

    #[error("Unknown database error")]
    Unknown(#[source] sqlx::Error),
}

impl DatabaseError {
    pub fn is_internal(&self) -> bool {
        matches!(self, Self::Unknown(_) | Self::InvariantCorrupted { .. })
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
                Some("fk_books_creators_author") => {
                    return Self::BookAuthorDoesNotExist(err);
                }
                Some("fk_books_creators_artist") => {
                    return Self::BookArtistDoesNotExist(err);
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
                Some("fk_book_titles_languages_language_id") => {
                    return Self::BookTitleLanguageDoesNotExist(err);
                }
                Some("check_length_book_titles_name") => {
                    return Self::BookTitleNameTooLong(err);
                }
                Some("check_count_book_titles_book_id") => {
                    return Self::BookTitlesExceedLimit(err);
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
            Self::CreatorNotFound | Self::BookNotFound => (StatusCode::NOT_FOUND, self.to_string()),
            e => (StatusCode::UNPROCESSABLE_ENTITY, e.to_string()),
        }
        .into_response()
    }
}
