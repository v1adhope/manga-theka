use axum::{
    extract::FromRequestParts,
    http::{HeaderMap, request::Parts},
};
use std::convert::Infallible;
use uuid::Uuid;

use crate::entity::{Role, UserClaims};

const USER_ID_HEADER: &str = "x-user-id";
const USER_ROLE_HEADER: &str = "x-user-role";

// deferred: replace with the access-JWT middleware from ADR-0003; until then the caller
// asserts its own identity, so this is a development stand-in and not a security boundary.
fn claims_from_headers(headers: &HeaderMap) -> UserClaims {
    let id = headers
        .get(USER_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| Uuid::parse_str(value.trim()).ok())
        .unwrap_or_default();

    let roles = headers
        .get_all(USER_ROLE_HEADER)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .filter_map(|role| role.trim().parse::<Role>().ok())
        .collect();

    UserClaims { id, roles }
}

impl<S: Send + Sync> FromRequestParts<S> for UserClaims {
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(claims_from_headers(&parts.headers))
    }
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue};
    use uuid::Uuid;

    use crate::{entity::Role, route::user::claims_from_headers};

    fn headers(pairs: &[(&'static str, &str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (name, value) in pairs {
            headers.append(*name, HeaderValue::from_str(value).unwrap());
        }

        headers
    }

    #[test]
    fn no_headers_is_a_guest() {
        let claims = claims_from_headers(&HeaderMap::new());

        assert!(claims.id.is_nil());
        assert!(claims.roles.is_empty());
    }

    #[test]
    fn an_identity_is_read_from_its_headers() {
        let id = Uuid::now_v7();
        let claims = claims_from_headers(&headers(&[
            ("x-user-id", &id.to_string()),
            ("x-user-role", "Moderator"),
        ]));

        assert_eq!(claims.id, id);
        assert!(claims.can_moderate());
    }

    #[test]
    fn roles_arrive_comma_separated_or_repeated() {
        let comma = claims_from_headers(&headers(&[("x-user-role", "Reader,Uploader")]));
        let repeated = claims_from_headers(&headers(&[
            ("x-user-role", "Reader"),
            ("x-user-role", "Uploader"),
        ]));

        assert_eq!(comma.roles, vec![Role::Reader, Role::Uploader]);
        assert_eq!(repeated.roles, comma.roles);
    }

    #[test]
    fn unreadable_values_drop_to_no_privilege() {
        let claims = claims_from_headers(&headers(&[
            ("x-user-id", "not-a-uuid"),
            ("x-user-role", "Owner"),
        ]));

        assert!(claims.id.is_nil());
        assert!(claims.roles.is_empty());
    }
}
