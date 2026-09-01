use axum::{
    Json,
    extract::{Multipart, Path, Query, State},
    http::{StatusCode, header},
    response::IntoResponse,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    entity::{
        ChapterPageParams, ChapterPages, ChapterRelease, MAX_PARTS_PER_REQUEST, Ordinal, PageOrder,
        UserClaims,
    },
    error::{AppError, RouteError},
    route::{StoreResp, collect_image_part, json_data_response},
    service::Service,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterReleaseReq {
    pub language_id: Uuid,
}

// deferred: gate to Uploader/Moderator/Admin
pub async fn store_chapter_release(
    State(service): State<Service>,
    Path(chapter_id): Path<Uuid>,
    Json(req): Json<ChapterReleaseReq>,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    let id = Uuid::now_v7();
    let release = ChapterRelease {
        id,
        chapter_id,
        language_id: req.language_id,
    };

    service.store_chapter_release(release).await?;

    Ok(json_data_response(StatusCode::CREATED, StoreResp { id }))
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

// deferred: gate to Uploader/Moderator/Admin; scope staged pages to the calling uploader
pub async fn upload_chapter_pages(
    State(service): State<Service>,
    Path(release_id): Path<Uuid>,
    mut multipart: Multipart,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    service.ensure_chapter_release_writable(release_id).await?;

    let mut images = Vec::with_capacity(MAX_PARTS_PER_REQUEST);

    while let Some(field) = multipart.next_field().await.map_err(RouteError::from)? {
        ChapterRelease::ensure_part_capacity(images.len() + 1)?;

        images.push(collect_image_part(field).await?);
    }

    let pages = ChapterPages {
        release_id,
        images: images.try_into()?,
    };

    let ids: Vec<Uuid> = pages
        .images
        .as_slice()
        .iter()
        .map(|image| image.id)
        .collect();

    service.store_chapter_pages(pages).await?;

    Ok(json_data_response(StatusCode::CREATED, ids))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitReq {
    pub page_order: Vec<Uuid>,
}

// deferred: gate to Uploader/Moderator/Admin; commit only the caller's own staged pages
pub async fn commit_chapter_release(
    State(service): State<Service>,
    Path(id): Path<Uuid>,
    Json(req): Json<CommitReq>,
) -> Result<StatusCode, AppError> {
    let order = PageOrder::try_from(req.page_order)?;

    service.commit_chapter_release(id, &order).await?;

    Ok(StatusCode::NO_CONTENT)
}

// deferred: also admit the book's submitter once `books` records one
pub async fn get_chapter_pages(
    State(service): State<Service>,
    Path(release_id): Path<Uuid>,
    Query(params): Query<ChapterPageParams>,
    claims: Option<UserClaims>,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    let pages = service
        .get_chapter_pages(release_id, params, claims.as_ref())
        .await?;

    Ok(json_data_response(StatusCode::OK, pages))
}

// deferred: also admit the book's submitter once `books` records one
pub async fn get_chapter_page_image(
    State(service): State<Service>,
    Path((release_id, page_id)): Path<(Uuid, Uuid)>,
    claims: Option<UserClaims>,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    let url = service
        .presign_chapter_page(release_id, page_id, claims.as_ref())
        .await?;

    Ok((StatusCode::FOUND, [(header::LOCATION, url)]))
}

// deferred: also admit the book's submitter once `books` records one
pub async fn get_chapter_page(
    State(service): State<Service>,
    Path((release_id, page_number)): Path<(Uuid, i32)>,
    claims: Option<UserClaims>,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    let number = Ordinal::try_from(page_number)?;

    let url = service
        .presign_chapter_page_by_number(release_id, number, claims.as_ref())
        .await?;

    Ok((StatusCode::FOUND, [(header::LOCATION, url)]))
}

// deferred: gate to Uploader/Moderator/Admin
pub async fn delete_chapter_release(
    State(service): State<Service>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    service.delete_chapter_release(id).await?;

    Ok(StatusCode::NO_CONTENT)
}
