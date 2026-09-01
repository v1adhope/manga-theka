use axum::{
    extract::{FromRequestParts, OptionalFromRequestParts},
    http::{HeaderMap, request::Parts},
};
use uuid::Uuid;

use crate::{
    entity::{Role, UserClaims},
    error::RouteError,
};

const USER_ID_HEADER: &str = "x-user-id";
const USER_ROLE_HEADER: &str = "x-user-role";

// deferred: replace with the access-JWT middleware from ADR-0003; until then the caller
// asserts its own identity, so this is a development stand-in and not a security boundary.
fn claims_from_headers(headers: &HeaderMap) -> Result<UserClaims, RouteError> {
    let id = headers
        .get(USER_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| Uuid::parse_str(value.trim()).ok())
        .ok_or(RouteError::InvalidCredentials)?;

    let mut roles = Vec::new();

    for value in headers.get_all(USER_ROLE_HEADER) {
        let value = value.to_str().map_err(|_| RouteError::InvalidCredentials)?;

        for name in value.split(',') {
            let role = name
                .trim()
                .parse::<Role>()
                .map_err(|_| RouteError::InvalidCredentials)?;

            roles.push(role);
        }
    }

    if roles.is_empty() {
        return Err(RouteError::InvalidCredentials);
    }

    Ok(UserClaims { id, roles })
}

fn claims_from_optional_headers(headers: &HeaderMap) -> Result<Option<UserClaims>, RouteError> {
    if !headers.contains_key(USER_ID_HEADER) && !headers.contains_key(USER_ROLE_HEADER) {
        return Ok(None);
    }

    claims_from_headers(headers).map(Some)
}

impl<S: Send + Sync> FromRequestParts<S> for UserClaims {
    type Rejection = RouteError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        claims_from_headers(&parts.headers)
    }
}

impl<S: Send + Sync> OptionalFromRequestParts<S> for UserClaims {
    type Rejection = RouteError;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Option<Self>, Self::Rejection> {
        claims_from_optional_headers(&parts.headers)
    }
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue};
    use uuid::Uuid;

    use crate::{
        entity::Role,
        route::authz::{claims_from_headers, claims_from_optional_headers},
    };

    fn headers(pairs: &[(&'static str, &str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (name, value) in pairs {
            headers.append(*name, HeaderValue::from_str(value).unwrap());
        }

        headers
    }

    #[test]
    fn an_identity_is_read_from_its_headers() {
        let id = Uuid::now_v7();
        let claims = claims_from_headers(&headers(&[
            ("x-user-id", &id.to_string()),
            ("x-user-role", "Moderator"),
        ]))
        .unwrap();

        assert_eq!(claims.id, id);
        assert!(claims.can_moderate());
    }

    #[test]
    fn roles_arrive_comma_separated_or_repeated() {
        let comma = claims_from_headers(&headers(&[
            ("x-user-id", &Uuid::now_v7().to_string()),
            ("x-user-role", "Reader,Uploader"),
        ]))
        .unwrap();
        let repeated = claims_from_headers(&headers(&[
            ("x-user-id", &Uuid::now_v7().to_string()),
            ("x-user-role", "Reader"),
            ("x-user-role", "Uploader"),
        ]))
        .unwrap();

        assert_eq!(comma.roles, vec![Role::Reader, Role::Uploader]);
        assert_eq!(repeated.roles, comma.roles);
    }

    #[test]
    fn a_half_stated_identity_is_rejected() {
        let id = Uuid::now_v7().to_string();

        assert!(claims_from_headers(&HeaderMap::new()).is_err());
        assert!(claims_from_headers(&headers(&[("x-user-id", &id)])).is_err());
        assert!(claims_from_headers(&headers(&[("x-user-role", "Reader")])).is_err());
    }

    #[test]
    fn unreadable_values_are_rejected() {
        let id = Uuid::now_v7().to_string();

        assert!(
            claims_from_headers(&headers(&[
                ("x-user-id", "not-a-uuid"),
                ("x-user-role", "Reader"),
            ]))
            .is_err()
        );
        assert!(
            claims_from_headers(&headers(&[("x-user-id", &id), ("x-user-role", "Owner")])).is_err()
        );
    }

    #[test]
    fn no_identity_headers_are_anonymous() {
        let claims = claims_from_optional_headers(&HeaderMap::new()).unwrap();

        assert!(claims.is_none());
    }

    #[test]
    fn an_asserted_identity_is_still_read_when_it_is_optional() {
        let id = Uuid::now_v7();
        let claims = claims_from_optional_headers(&headers(&[
            ("x-user-id", &id.to_string()),
            ("x-user-role", "Moderator"),
        ]))
        .unwrap()
        .unwrap();

        assert_eq!(claims.id, id);
        assert!(claims.can_moderate());
    }

    #[test]
    fn a_broken_identity_is_rejected_rather_than_read_as_anonymous() {
        let id = Uuid::now_v7().to_string();

        assert!(claims_from_optional_headers(&headers(&[("x-user-id", "not-a-uuid")])).is_err());
        assert!(claims_from_optional_headers(&headers(&[("x-user-role", "Reader")])).is_err());
        assert!(
            claims_from_optional_headers(&headers(
                &[("x-user-id", &id), ("x-user-role", "Owner"),]
            ))
            .is_err()
        );
    }
}
