use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use axum_extra::extract::CookieJar;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    entity::{Email, LoginForm, Password, Timestamp, UserClaims},
    error::{AppError, EntityError, RouteError},
    route::{
        cookie::{REFRESH_COOKIE, clear_refresh_cookie},
        header::{forwarded_ip, user_agent},
        json_data_response, session_tokens_response,
    },
    service::Service,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginFormReq {
    pub email: String,
    pub password: String,
}

impl TryFrom<(LoginFormReq, HeaderMap, Timestamp)> for LoginForm {
    type Error = EntityError;

    fn try_from(ctx: (LoginFormReq, HeaderMap, Timestamp)) -> Result<Self, Self::Error> {
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
    let now = Timestamp::now();
    let form: LoginForm = (req, headers, now).try_into()?;

    let tokens = service.login(form).await?;

    let (jar, body) = session_tokens_response(CookieJar::new(), tokens);

    Ok((StatusCode::CREATED, jar, body))
}

pub async fn refresh(
    State(service): State<Service>,
    jar: CookieJar,
) -> Result<(CookieJar, impl IntoResponse), AppError> {
    let token = jar
        .get(REFRESH_COOKIE)
        .map(|c| c.value().to_owned())
        .ok_or(RouteError::InvalidCredentials)?;
    let now = Timestamp::now();

    let tokens = service.refresh_session(&token, now).await?;

    Ok(session_tokens_response(jar, tokens))
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
        .revoke_all_sessions(claims.id, claims.sid, Timestamp::now())
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
        .revoke_session(claims.id, claims.sid, sid, Timestamp::now())
        .await?;

    let jar = clear_cookie_for_own_session(jar, sid, claims.sid);

    Ok((StatusCode::NO_CONTENT, jar))
}

fn clear_cookie_for_own_session(jar: CookieJar, revoked_sid: Uuid, current_sid: Uuid) -> CookieJar {
    if revoked_sid == current_sid {
        clear_refresh_cookie(jar)
    } else {
        jar
    }
}
