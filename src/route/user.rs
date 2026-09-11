use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    entity::{Email, Password, Timestamp, User, UserClaims, Username},
    error::{AppError, EntityError},
    route::{StoreResp, json_data_response},
    service::Service,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterReq {
    pub email: String,
    pub username: String,
    pub password: String,
}

impl TryFrom<(RegisterReq, Uuid, Timestamp)> for User {
    type Error = EntityError;

    fn try_from(ctx: (RegisterReq, Uuid, Timestamp)) -> Result<Self, Self::Error> {
        let (req, id, created_at) = ctx;

        let email = Email::try_from(req.email)?;
        let username = Username::try_from(req.username)?;
        let password = Password::try_from(req.password)?;

        Ok(Self {
            id,
            email,
            username,
            password,
            created_at,
        })
    }
}

pub async fn register(
    State(service): State<Service>,
    Json(req): Json<RegisterReq>,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    let id = Uuid::now_v7();
    let created_at = Timestamp::now();
    let user: User = (req, id, created_at).try_into()?;

    service.register(user).await?;

    Ok(json_data_response(StatusCode::CREATED, StoreResp { id }))
}

pub async fn get_me(
    State(service): State<Service>,
    claims: UserClaims,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    let user = service.get_user(claims.id).await?;

    Ok(json_data_response(StatusCode::OK, user))
}
