use std::future::Future;
use std::pin::Pin;

use axum::{
    Extension,
    extract::{Request, State},
    http::header,
    middleware::{FromFnLayer, Next, from_fn_with_state},
    response::{IntoResponse, Response},
};

use crate::{
    entity::{Role, UserClaims},
    error::RouteError,
    service::Service,
};

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
    let claims = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split_once(' '))
        .filter(|(scheme, _)| scheme.eq_ignore_ascii_case("bearer"))
        .map(|(_, token)| token.trim())
        .filter(|token| !token.is_empty())
        .and_then(|token| service.jwt.verify_access(token).ok())
        .map(|access| UserClaims {
            id: access.sub,
            sid: access.sid,
            roles: access.roles,
        });

    req.extensions_mut().insert::<Option<UserClaims>>(claims);
    next.run(req).await
}

type BoxFuture = Pin<Box<dyn Future<Output = Response> + Send>>;

type GateFn = fn(State<&'static [Role]>, Extension<Option<UserClaims>>, Request, Next) -> BoxFuture;

/// The layer type `require_roles` produces.
pub type RoleGate = FromFnLayer<
    GateFn,
    &'static [Role],
    (
        State<&'static [Role]>,
        Extension<Option<UserClaims>>,
        Request,
    ),
>;

/// A `route_layer` over the same extension: empty role intersection -> 403,
/// absent claims -> 401. Pure OR, no hierarchy.
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
