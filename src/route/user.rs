use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::Deserialize;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    entity::{Email, Password, User, UserClaims, Username},
    error::AppError,
    route::json_data_response,
    service::Service,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterReq {
    pub email: String,
    pub username: String,
    pub password: String,
}

impl TryFrom<RegisterReq> for User {
    type Error = AppError;

    fn try_from(req: RegisterReq) -> Result<Self, Self::Error> {
        Ok(Self {
            email: Email::try_from(req.email)?,
            username: Username::try_from(req.username)?,
            password: Password::try_from(req.password)?,
        })
    }
}

pub async fn register_user(
    State(service): State<Service>,
    Json(req): Json<RegisterReq>,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    let id = Uuid::now_v7();
    let created_at = OffsetDateTime::now_utc();

    let user = User::try_from(req)?;

    let stored = service.register_user(id, user, created_at).await?;

    Ok(json_data_response(StatusCode::CREATED, stored))
}

pub async fn get_me(
    State(service): State<Service>,
    claims: UserClaims,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    let user = service.get_user(claims.id).await?;

    Ok(json_data_response(StatusCode::OK, user))
}
