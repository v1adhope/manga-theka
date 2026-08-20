use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    entity::{
        Chapter, ChapterLocalization, ChapterName, ChapterNumber, ChapterVolume, Filter, SortOrder,
    },
    error::{AppError, EntityError},
    route::{StoreResp, json_data_response, json_response},
    service::Service,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterLocalizationReq {
    pub language_id: Uuid,
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterReq {
    pub number: f32,
    pub name: Option<String>,
    pub volume: Option<i16>,
    #[serde(default)]
    pub localizations: Vec<ChapterLocalizationReq>,
}

struct ChapterWithRelations {
    req: ChapterReq,
    id: Uuid,
    book_id: Uuid,
    updated_at: Option<OffsetDateTime>,
    created_at: OffsetDateTime,
}

impl TryFrom<ChapterWithRelations> for Chapter {
    type Error = EntityError;

    fn try_from(item: ChapterWithRelations) -> Result<Self, Self::Error> {
        let ChapterWithRelations {
            req,
            id,
            book_id,
            updated_at,
            created_at,
        } = item;

        let ChapterReq {
            number,
            name,
            volume,
            localizations: localization_reqs,
        } = req;

        let mut localizations = Vec::with_capacity(localization_reqs.len());
        for localization in localization_reqs {
            localizations.push(ChapterLocalization {
                language_id: localization.language_id,
                name: ChapterName::try_from(localization.name)?,
            });
        }

        Ok(Chapter {
            id,
            book_id,
            number: ChapterNumber::try_from(number)?,
            name: name.map(ChapterName::try_from).transpose()?,
            volume: volume.map(ChapterVolume::try_from).transpose()?,
            localizations,
            updated_at,
            created_at,
        })
    }
}

pub async fn store_chapter(
    State(service): State<Service>,
    Path(book_id): Path<Uuid>,
    Json(req): Json<ChapterReq>,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    let id = Uuid::now_v7();
    let chapter: Chapter = ChapterWithRelations {
        req,
        id,
        book_id,
        updated_at: None,
        created_at: OffsetDateTime::now_utc(),
    }
    .try_into()?;

    service.store_chapter(chapter).await?;

    Ok(json_data_response(StatusCode::CREATED, StoreResp { id }))
}

pub async fn update_chapter(
    State(service): State<Service>,
    Path(id): Path<Uuid>,
    Json(req): Json<ChapterReq>,
) -> Result<StatusCode, AppError> {
    let chapter: Chapter = ChapterWithRelations {
        req,
        id,
        book_id: Uuid::nil(),
        updated_at: Some(OffsetDateTime::now_utc()),
        created_at: OffsetDateTime::UNIX_EPOCH,
    }
    .try_into()?;

    service.update_chapter(chapter).await?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterListQuery {
    pub order: Option<SortOrder>,
    pub after: Option<Uuid>,
    pub limit: Option<u32>,
}

impl TryFrom<ChapterListQuery> for Filter {
    type Error = EntityError;

    fn try_from(q: ChapterListQuery) -> Result<Self, Self::Error> {
        Self::builder()
            .after(q.after)
            .limit(q.limit)
            .sort_order(q.order)
            .build()
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetChaptersResp {
    pub data: Vec<Chapter>,
    pub next_cursor: Option<Uuid>,
}

pub async fn get_chapters(
    State(service): State<Service>,
    Path(book_id): Path<Uuid>,
    Query(query): Query<ChapterListQuery>,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    let filter: Filter = query.try_into()?;

    let (data, next_cursor) = service.get_chapters(book_id, filter).await?;

    Ok(json_response(
        StatusCode::OK,
        GetChaptersResp { data, next_cursor },
    ))
}

pub async fn get_chapter(
    State(service): State<Service>,
    Path(id): Path<Uuid>,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    let chapter = service.get_chapter(id).await?;

    Ok(json_data_response(StatusCode::OK, chapter))
}

pub async fn delete_chapter(
    State(service): State<Service>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    service.delete_chapter(id).await?;

    Ok(StatusCode::NO_CONTENT)
}
