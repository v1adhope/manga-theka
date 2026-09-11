use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    entity::{Creator, CreatorQuery, Filter, Name, Timestamp},
    error::{AppError, EntityError},
    route::{PaginationQuery, StoreResp, json_data_response, json_response},
    service::Service,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatorReq {
    pub first_name: String,
    pub last_name: String,
}

impl TryFrom<(CreatorReq, Uuid, Timestamp)> for Creator {
    type Error = EntityError;

    fn try_from(ctx: (CreatorReq, Uuid, Timestamp)) -> Result<Self, Self::Error> {
        let (req, id, created_at) = ctx;

        let first_name = Name::try_from(req.first_name)?;
        let last_name = Name::try_from(req.last_name)?;

        Ok(Self {
            id,
            first_name,
            last_name,
            created_at,
        })
    }
}

// TODO: any signed-in User can now propose a Creator with no Book attached; add a sweeper
// that deletes Creator rows unreferenced by book_creators after some retention window.
pub async fn store_creator(
    State(service): State<Service>,
    Json(req): Json<CreatorReq>,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    let id = Uuid::now_v7();
    let created_at = Timestamp::now();
    let creator: Creator = (req, id, created_at).try_into()?;

    service.store_creator(creator).await?;

    Ok(json_data_response(StatusCode::CREATED, StoreResp { id }))
}

pub async fn update_creator(
    State(service): State<Service>,
    Path(id): Path<Uuid>,
    Json(req): Json<CreatorReq>,
) -> Result<StatusCode, AppError> {
    let creator: Creator = (req, id, Timestamp::UNIX_EPOCH).try_into()?;
    service.update_creator(creator).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetCreatorsResp {
    pub data: Vec<CreatorQuery>,
    pub next_cursor: Option<Uuid>,
}

pub async fn get_creators(
    State(service): State<Service>,
    Query(query): Query<PaginationQuery>,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    let filter: Filter = query.try_into()?;

    let (data, next_cursor) = service.get_creators(filter).await?;

    Ok(json_response(
        StatusCode::OK,
        GetCreatorsResp { data, next_cursor },
    ))
}

pub async fn get_creator(
    State(service): State<Service>,
    Path(id): Path<Uuid>,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    let creator = service.get_creator(id).await?;
    Ok(json_data_response(StatusCode::OK, creator))
}

pub async fn delete_creator(
    State(service): State<Service>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    service.delete_creator(id).await?;
    Ok(StatusCode::NO_CONTENT)
}
