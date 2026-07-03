use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum DatabaseError {
    #[error("Creator first name exceeds the 255 character limit")]
    CreatorFirstNameTooLong(#[source] sqlx::Error),

    #[error("Creator last name exceeds the 255 character limit")]
    CreatorLastNameTooLong(#[source] sqlx::Error),

    #[error("Create role doesn't exist")]
    CreatorRoleDoesNotExist(#[source] sqlx::Error),

    #[error("Creator full name has already exist")]
    CreatorFullNameDuplication(#[source] sqlx::Error),

    #[error("Unknown database error")]
    Unknown(#[source] sqlx::Error),
}

impl DatabaseError {
    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown(_))
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
                _ => {}
            }
        }
        Self::Unknown(err)
    }
}

impl IntoResponse for DatabaseError {
    fn into_response(self) -> Response {
        match self {
            Self::Unknown(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Something went wrong".to_string(),
            ),
            e => (StatusCode::UNPROCESSABLE_ENTITY, e.to_string()),
        }
        .into_response()
    }
}
