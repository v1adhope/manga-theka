use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

use crate::entity::{Book, Entity};

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum DatabaseError {
    #[error("{field} is out of range")]
    OutOfRange {
        field: &'static str,
        #[source]
        source: sqlx::Error,
    },

    #[error("{field} doesn't exist")]
    DoesNotExist {
        field: &'static str,
        #[source]
        source: sqlx::Error,
    },

    #[error("{field} is attached more than once")]
    Duplication {
        field: &'static str,
        #[source]
        source: sqlx::Error,
    },

    #[error("{field} already exists")]
    AlreadyExists {
        field: &'static str,
        #[source]
        source: sqlx::Error,
    },

    #[error("{entity} not found")]
    NotFound { entity: &'static str },

    #[error("Creator is attached to one or more books")]
    CreatorInUse(#[source] sqlx::Error),

    #[error("Another cover was promoted concurrently")]
    BookCoverMainConflict(#[source] sqlx::Error),

    #[error("Database invariant corrupted on field '{field}': {msg}")]
    InvariantCorrupted { field: &'static str, msg: String },

    #[error("Unknown database error")]
    Unknown(#[source] sqlx::Error),
}

impl DatabaseError {
    pub fn log_internal(&self) {
        if matches!(self, Self::Unknown(_) | Self::InvariantCorrupted { .. }) {
            tracing::error!(error = ?self, "internal database error");
        }
    }

    pub fn not_found<T: Entity>() -> Self {
        Self::NotFound { entity: T::NAME }
    }

    pub fn invariant_corrupted(field: &'static str, msg: impl std::error::Error) -> Self {
        Self::InvariantCorrupted {
            field,
            msg: msg.to_string(),
        }
    }
}

impl From<sqlx::Error> for DatabaseError {
    fn from(err: sqlx::Error) -> Self {
        if let Some(db_err) = err.as_database_error() {
            match db_err.constraint() {
                Some("check_length_creators_first_name") => {
                    return Self::OutOfRange {
                        field: "Creator first name",
                        source: err,
                    };
                }
                Some("check_length_creators_last_name") => {
                    return Self::OutOfRange {
                        field: "Creator last name",
                        source: err,
                    };
                }
                Some("enum_creators_role") => {
                    return Self::DoesNotExist {
                        field: "Creator role",
                        source: err,
                    };
                }
                Some("unique_creators_first_name_last_name") => {
                    return Self::AlreadyExists {
                        field: "Creator full name",
                        source: err,
                    };
                }
                Some("fk_book_creators_creators_creator_id") => {
                    return Self::CreatorInUse(err);
                }
                Some("check_length_books_name") => {
                    return Self::OutOfRange {
                        field: "Book name",
                        source: err,
                    };
                }
                Some("check_length_books_description") => {
                    return Self::OutOfRange {
                        field: "Book description",
                        source: err,
                    };
                }
                Some("enum_books_status") => {
                    return Self::DoesNotExist {
                        field: "Book status",
                        source: err,
                    };
                }
                Some("enum_books_kind") => {
                    return Self::DoesNotExist {
                        field: "Book kind",
                        source: err,
                    };
                }
                Some("fk_books_content_ratings_content_rating") => {
                    return Self::DoesNotExist {
                        field: "Book content rating",
                        source: err,
                    };
                }
                Some("fk_books_languages_publication_language") => {
                    return Self::DoesNotExist {
                        field: "Book publication language",
                        source: err,
                    };
                }
                Some("fk_book_labels_labels_label_id") => {
                    return Self::DoesNotExist {
                        field: "Book label",
                        source: err,
                    };
                }
                Some("pk_book_labels_book_id_label_id") => {
                    return Self::Duplication {
                        field: "Book label",
                        source: err,
                    };
                }
                Some("enum_book_links_kind") => {
                    return Self::DoesNotExist {
                        field: "Book link kind",
                        source: err,
                    };
                }
                Some("check_length_book_links_url") => {
                    return Self::OutOfRange {
                        field: "Book link url",
                        source: err,
                    };
                }
                Some("pk_book_links_book_id_link_hash") => {
                    return Self::Duplication {
                        field: "Book link url",
                        source: err,
                    };
                }
                Some("fk_book_titles_languages_language_id") => {
                    return Self::DoesNotExist {
                        field: "Alternative title language",
                        source: err,
                    };
                }
                Some("check_length_book_titles_name") => {
                    return Self::OutOfRange {
                        field: "Alternative title name",
                        source: err,
                    };
                }
                Some("pk_book_titles_book_id_language_id_name") => {
                    return Self::Duplication {
                        field: "Alternative title name",
                        source: err,
                    };
                }
                Some("enum_book_covers_extension") => {
                    return Self::DoesNotExist {
                        field: "Book cover extension",
                        source: err,
                    };
                }
                Some("unique_book_covers_book_id_is_main") => {
                    return Self::BookCoverMainConflict(err);
                }
                Some("fk_book_covers_books_book_id") | Some("fk_chapters_books_book_id") => {
                    return Self::not_found::<Book>();
                }
                Some("check_length_chapters_name") => {
                    return Self::OutOfRange {
                        field: "Chapter name",
                        source: err,
                    };
                }
                Some("check_range_chapters_number") | Some("check_scale_chapters_number") => {
                    return Self::OutOfRange {
                        field: "Chapter number",
                        source: err,
                    };
                }
                Some("check_range_chapters_volume") => {
                    return Self::OutOfRange {
                        field: "Chapter volume",
                        source: err,
                    };
                }
                Some("unique_chapters_book_id_number") => {
                    return Self::AlreadyExists {
                        field: "Chapter number",
                        source: err,
                    };
                }
                Some("fk_chapter_localizations_languages_language_id") => {
                    return Self::DoesNotExist {
                        field: "Chapter localization language",
                        source: err,
                    };
                }
                Some("check_length_chapter_localizations_name") => {
                    return Self::OutOfRange {
                        field: "Chapter localization name",
                        source: err,
                    };
                }
                Some("pk_chapter_localizations_chapter_id_language_id") => {
                    return Self::Duplication {
                        field: "Chapter localization language",
                        source: err,
                    };
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
            Self::NotFound { .. } => (StatusCode::NOT_FOUND, self.to_string()),
            Self::AlreadyExists { .. } | Self::CreatorInUse(_) | Self::BookCoverMainConflict(_) => {
                (StatusCode::CONFLICT, self.to_string())
            }
            Self::OutOfRange { .. } | Self::DoesNotExist { .. } | Self::Duplication { .. } => {
                (StatusCode::UNPROCESSABLE_ENTITY, self.to_string())
            }
        }
        .into_response()
    }
}
