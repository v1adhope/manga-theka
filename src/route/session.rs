use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use axum_extra::extract::CookieJar;
use serde::Deserialize;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    entity::{Email, LoginForm, Password, UserClaims},
    error::{AppError, EntityError, RouteError},
    route::{
        AccessTokenResp,
        cookie::{REFRESH_COOKIE, clear_refresh_cookie, refresh_cookie},
        header::{forwarded_ip, user_agent},
        json_data, json_data_response,
    },
    service::Service,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginFormReq {
    pub email: String,
    pub password: String,
}

impl TryFrom<(LoginFormReq, HeaderMap, OffsetDateTime)> for LoginForm {
    type Error = EntityError;

    fn try_from(ctx: (LoginFormReq, HeaderMap, OffsetDateTime)) -> Result<Self, Self::Error> {
        let (req, headers, now) = ctx;

        let email = Email::try_from(req.email)?;
        let password = Password::try_from(req.password)?;
        let ua = user_agent(&headers);
        let ip = forwarded_ip(&headers);

        Ok(Self {
            email,
            password,
            ua,
            ip,
            now,
        })
    }
}

pub async fn login(
    State(service): State<Service>,
    headers: HeaderMap,
    Json(req): Json<LoginFormReq>,
) -> Result<(StatusCode, CookieJar, impl IntoResponse), AppError> {
    let now = OffsetDateTime::now_utc();
    let form: LoginForm = (req, headers, now).try_into()?;

    let tokens = service.login(form).await?;

    let jar = CookieJar::new().add(refresh_cookie(tokens.refresh, service.jwt.refresh_ttl()));

    Ok((
        StatusCode::CREATED,
        jar,
        json_data(AccessTokenResp {
            access_token: tokens.access,
        }),
    ))
}

pub async fn refresh(
    State(service): State<Service>,
    jar: CookieJar,
) -> Result<(CookieJar, impl IntoResponse), AppError> {
    let token = jar
        .get(REFRESH_COOKIE)
        .map(|c| c.value().to_owned())
        .ok_or(RouteError::InvalidCredentials)?;
    let now = OffsetDateTime::now_utc();

    let tokens = service.refresh_session(&token, now).await?;

    let jar = jar.add(refresh_cookie(tokens.refresh, service.jwt.refresh_ttl()));

    Ok((
        jar,
        json_data(AccessTokenResp {
            access_token: tokens.access,
        }),
    ))
}

pub async fn list_my_sessions(
    State(service): State<Service>,
    claims: UserClaims,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
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
