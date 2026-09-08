use axum::{
    extract::{FromRequestParts, OptionalFromRequestParts},
    http::request::Parts,
};

use crate::{entity::UserClaims, error::RouteError};

/// Both extractors read the `Extension<Option<UserClaims>>` the global auth
/// middleware populates from the access JWT -- absent header -> `None`, present
/// but invalid -> the middleware already short-circuited 401.
impl<S: Send + Sync> FromRequestParts<S> for UserClaims {
    type Rejection = RouteError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Option<UserClaims>>()
            .cloned()
            .flatten()
            .ok_or(RouteError::InvalidCredentials)
    }
}

impl<S: Send + Sync> OptionalFromRequestParts<S> for UserClaims {
    type Rejection = RouteError;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Option<Self>, Self::Rejection> {
        Ok(parts
            .extensions
            .get::<Option<UserClaims>>()
            .cloned()
            .flatten())
    }
}
