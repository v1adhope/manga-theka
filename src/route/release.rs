use axum::{
    Json,
    extract::{Multipart, Path, Query, State},
    http::{StatusCode, header},
    response::IntoResponse,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    entity::{ChapterPageParams, ChapterPages, ChapterRelease, MAX_PARTS_PER_REQUEST, PageOrder},
    error::{AppError, RouteError},
    route::{StoreResp, collect_image_part, json_data_response},
    service::Service,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterReleaseReq {
    pub language_id: Uuid,
}

pub async fn store_chapter_release(
    State(service): State<Service>,
    Path(chapter_id): Path<Uuid>,
    Json(req): Json<ChapterReleaseReq>,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    let release = ChapterRelease {
        id: Uuid::now_v7(),
        chapter_id,
        language_id: req.language_id,
    };

    service.store_chapter_release(&release).await?;

    Ok(json_data_response(
        StatusCode::CREATED,
        StoreResp { id: release.id },
    ))
}

pub async fn get_chapter_releases(
    State(service): State<Service>,
    Path(chapter_id): Path<Uuid>,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    let releases = service.get_chapter_releases(chapter_id).await?;

    Ok(json_data_response(StatusCode::OK, releases))
}

pub async fn get_chapter_release(
    State(service): State<Service>,
    Path(id): Path<Uuid>,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    let release = service.get_chapter_release(id).await?;

    Ok(json_data_response(StatusCode::OK, release))
}

pub async fn upload_chapter_pages(
    State(service): State<Service>,
    Path(release_id): Path<Uuid>,
    mut multipart: Multipart,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    service.ensure_chapter_release_exists(release_id).await?;

    let mut images = Vec::with_capacity(MAX_PARTS_PER_REQUEST);

    while let Some(field) = multipart.next_field().await.map_err(RouteError::from)? {
        ChapterRelease::ensure_part_capacity(images.len() + 1)?;

        images.push(collect_image_part(field).await?);
    }

    let pages = ChapterPages {
        release_id,
        images: images.try_into()?,
    };

    let ids = service.store_chapter_pages(pages).await?;

    Ok(json_data_response(StatusCode::CREATED, ids))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitReq {
    pub page_order: Vec<Uuid>,
}

pub async fn commit_chapter_release(
    State(service): State<Service>,
    Path(id): Path<Uuid>,
    Json(req): Json<CommitReq>,
) -> Result<StatusCode, AppError> {
    let order = PageOrder::try_from(req.page_order)?;

    service.commit_chapter_release(id, &order).await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_chapter_pages(
    State(service): State<Service>,
    Path(release_id): Path<Uuid>,
    Query(params): Query<ChapterPageParams>,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    let pages = service.get_chapter_pages(release_id, params).await?;

    Ok(json_data_response(StatusCode::OK, pages))
}

pub async fn get_chapter_page_image(
    State(service): State<Service>,
    Path((release_id, page_id)): Path<(Uuid, Uuid)>,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    let url = service.presign_chapter_page(release_id, page_id).await?;

    Ok((StatusCode::FOUND, [(header::LOCATION, url)]))
}

pub async fn delete_chapter_release(
    State(service): State<Service>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    service.delete_chapter_release(id).await?;

    Ok(StatusCode::NO_CONTENT)
}
