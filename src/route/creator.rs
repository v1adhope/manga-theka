use axum::{Json, extract::State, http::StatusCode};
use serde::Deserialize;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    entity::{Creator, CreatorRole, Name},
    error::{AppError, EntityError},
    service::Service,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreCreatorReq {
    pub first_name: String,
    pub last_name: String,
    pub role: CreatorRole,
}

impl TryFrom<(StoreCreatorReq, Uuid, OffsetDateTime)> for Creator {
    type Error = EntityError;

    fn try_from(ctx: (StoreCreatorReq, Uuid, OffsetDateTime)) -> Result<Self, Self::Error> {
        let (req, id, created_at) = ctx;

        let first_name = Name::try_from(req.first_name)?;
        let last_name = Name::try_from(req.last_name)?;

        Ok(Self {
            id,
            first_name,
            last_name,
            role: req.role,
            created_at,
        })
    }
}

pub async fn store_creator(
    State(service): State<Service>,
    Json(req): Json<StoreCreatorReq>,
) -> Result<StatusCode, AppError> {
    let id = Uuid::now_v7();
    let created_at = OffsetDateTime::now_utc();
    let creator: Creator = (req, id, created_at).try_into()?;

    service.store_creator(creator).await?;

    Ok(StatusCode::OK)
}
