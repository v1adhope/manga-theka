use std::future::Future;
use std::pin::Pin;

use axum::{
    Extension,
    extract::{FromRequestParts, OptionalFromRequestParts, Request, State},
    http::{header, request::Parts},
    middleware::{FromFnLayer, Next, from_fn_with_state},
    response::{IntoResponse, Response},
};

use crate::{
    entity::{Role, UserClaims},
    error::RouteError,
    service::Service,
};

pub const CONTENT_WRITERS: &[Role] = &[Role::Uploader, Role::Moderator, Role::Admin];
pub const MODERATORS: &[Role] = &[Role::Moderator, Role::Admin];
pub const SIGNED_IN: &[Role] = &[Role::Reader, Role::Uploader, Role::Moderator, Role::Admin];

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

/// The one global layer. Reads `Authorization: Bearer <jwt>` (scheme match is
/// case-insensitive, RFC 9110 s11.1) and inserts `Extension<Option<UserClaims>>`.
/// It never short-circuits: an absent, non-Bearer, or unverifiable token all
/// yield `None`, which `require_roles` / the `UserClaims` extractor turn into 401
/// on protected routes. A stale token riding along on a refresh or a public read
/// therefore does not lock the caller out.
pub async fn authenticate(
    State(service): State<Service>,
    mut req: Request,
    next: Next,
) -> Response {
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok());

    let token = auth_header
        .and_then(|value| value.split_once(' '))
        .filter(|(scheme, _)| scheme.eq_ignore_ascii_case("bearer"))
        .map(|(_, token)| token.trim())
        .filter(|token| !token.is_empty());

    let access = token.and_then(|token| service.jwt.verify_access(token).ok());

    let claims = access.map(|access| UserClaims {
        id: access.sub,
        sid: access.sid,
        roles: access.roles,
    });

    req.extensions_mut().insert::<Option<UserClaims>>(claims);
    next.run(req).await
}

type BoxFuture = Pin<Box<dyn Future<Output = Response> + Send>>;

type GateFn = fn(State<&'static [Role]>, Extension<Option<UserClaims>>, Request, Next) -> BoxFuture;

pub type RoleGate = FromFnLayer<
    GateFn,
    &'static [Role],
    (
        State<&'static [Role]>,
        Extension<Option<UserClaims>>,
        Request,
    ),
>;

pub fn require_roles(allowed: &'static [Role]) -> RoleGate {
    from_fn_with_state(allowed, gate as GateFn)
}

fn gate(
    State(allowed): State<&'static [Role]>,
    Extension(claims): Extension<Option<UserClaims>>,
    req: Request,
    next: Next,
) -> BoxFuture {
    Box::pin(async move {
        let Some(claims) = claims else {
            return RouteError::InvalidCredentials.into_response();
        };

        if claims.roles.iter().any(|role| allowed.contains(role)) {
            return next.run(req).await;
        }

        RouteError::Forbidden.into_response()
    })
}
