use std::net::IpAddr;

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
};
use axum_extra::extract::{
    CookieJar,
    cookie::{Cookie, SameSite},
};
use serde::Deserialize;
use serde_json::json;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::{
    entity::{Text, UserClaims},
    error::{AppError, RouteError},
    route::json_data_response,
    service::{Service, SessionTokens},
};

const REFRESH_COOKIE: &str = "refresh_token";
const REFRESH_COOKIE_PATH: &str = "/sessions";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginReq {
    pub email: String,
    pub password: String,
}

pub async fn login(
    State(service): State<Service>,
    headers: HeaderMap,
    Json(req): Json<LoginReq>,
) -> Result<(StatusCode, CookieJar, Json<serde_json::Value>), AppError> {
    let ua = user_agent(&headers);
    let ip = forwarded_ip(&headers);
    let now = OffsetDateTime::now_utc();

    // The email is not validated here: a malformed one takes the no-such-user
    // path, so every failed login returns 401 after the same work.
    let SessionTokens { access, refresh } =
        service.login(req.email, req.password, ua, ip, now).await?;

    let jar = CookieJar::new().add(refresh_cookie(refresh, service.jwt.refresh_ttl()));

    Ok((
        StatusCode::CREATED,
        jar,
        Json(json!({ "data": { "accessToken": access } })),
    ))
}

pub async fn refresh(
    State(service): State<Service>,
    jar: CookieJar,
) -> Result<(CookieJar, Json<serde_json::Value>), AppError> {
    let token = jar
        .get(REFRESH_COOKIE)
        .map(|c| c.value().to_owned())
        .ok_or(RouteError::InvalidCredentials)?;
    let now = OffsetDateTime::now_utc();

    let SessionTokens { access, refresh } = service.refresh_session(&token, now).await?;

    let jar = jar.add(refresh_cookie(refresh, service.jwt.refresh_ttl()));

    Ok((jar, Json(json!({ "data": { "accessToken": access } }))))
}

pub async fn list_my_sessions(
    State(service): State<Service>,
    claims: UserClaims,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    let sessions = service.list_sessions(claims.id).await?;

    Ok(json_data_response(StatusCode::OK, sessions))
}

pub async fn revoke_all_sessions(
    State(service): State<Service>,
    claims: UserClaims,
    jar: CookieJar,
) -> Result<(StatusCode, CookieJar), AppError> {
    service
        .revoke_all_sessions(claims.id, claims.sid, OffsetDateTime::now_utc())
        .await?;

    Ok((StatusCode::NO_CONTENT, clear_refresh_cookie(jar)))
}

pub async fn revoke_current_session(
    State(service): State<Service>,
    claims: UserClaims,
    jar: CookieJar,
) -> Result<(StatusCode, CookieJar), AppError> {
    service
        .revoke_current_session(claims.id, claims.sid)
        .await?;

    Ok((StatusCode::NO_CONTENT, clear_refresh_cookie(jar)))
}

pub async fn revoke_session(
    State(service): State<Service>,
    claims: UserClaims,
    jar: CookieJar,
    Path(sid): Path<Uuid>,
) -> Result<(StatusCode, CookieJar), AppError> {
    service
        .revoke_session(claims.id, claims.sid, sid, OffsetDateTime::now_utc())
        .await?;

    // Revoking one's own session by id must also drop the now-dead cookie.
    let jar = if sid == claims.sid {
        clear_refresh_cookie(jar)
    } else {
        jar
    };

    Ok((StatusCode::NO_CONTENT, jar))
}

fn refresh_cookie(value: String, ttl: i64) -> Cookie<'static> {
    Cookie::build((REFRESH_COOKIE, value))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Strict)
        .path(REFRESH_COOKIE_PATH)
        .max_age(Duration::seconds(ttl))
        .build()
}

fn clear_refresh_cookie(jar: CookieJar) -> CookieJar {
    jar.add(
        Cookie::build((REFRESH_COOKIE, ""))
            .http_only(true)
            .secure(true)
            .same_site(SameSite::Strict)
            .path(REFRESH_COOKIE_PATH)
            .max_age(Duration::ZERO)
            .build(),
    )
}

fn user_agent(headers: &HeaderMap) -> Option<Text> {
    headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| Text::try_from(v.to_owned()).ok())
}

fn forwarded_ip(headers: &HeaderMap) -> Option<IpAddr> {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .and_then(|v| v.trim().parse().ok())
}
