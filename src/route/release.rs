use axum::{
    Json,
    extract::{Multipart, Path, State, multipart::Field},
    http::{StatusCode, header},
    response::IntoResponse,
};
use bytes::BytesMut;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    entity::{
        ChapterPage, ChapterRelease, DEFAULT_IMAGE_MAX_BYTES, Image, ImageContent,
        MAX_PARTS_PER_REQUEST, PageOrder,
    },
    error::{AppError, EntityError},
    route::{StoreResp, json_data_response},
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

    let mut pages = Vec::new();
    let mut failure = None;

    while let Some(field) = multipart.next_field().await? {
        if pages.len() >= MAX_PARTS_PER_REQUEST {
            failure = Some(
                EntityError::UploadPartsExceedLimit(pages.len() + 1, MAX_PARTS_PER_REQUEST).into(),
            );
            break;
        }

        match collect_part(field).await {
            Ok(image) => pages.push(ChapterPage { release_id, image }),
            Err(e) => {
                failure = Some(e);
                break;
            }
        }
    }

    let ids = service.store_chapter_pages(release_id, pages).await?;

    if let Some(err) = failure {
        return Err(err);
    }

    Ok(json_data_response(StatusCode::CREATED, ids))
}

async fn collect_part(mut field: Field<'_>) -> Result<Image, AppError> {
    let file_name = field.file_name().map(str::to_owned);
    let mut content = BytesMut::new();

    while let Some(chunk) = field.chunk().await? {
        if content.len() + chunk.len() > DEFAULT_IMAGE_MAX_BYTES {
            return Err(EntityError::ImageExceedsByteLimit(DEFAULT_IMAGE_MAX_BYTES).into());
        }

        content.extend_from_slice(&chunk);
    }

    let content = ImageContent::try_from(content.freeze())?;
    let image = Image::new(Uuid::now_v7(), content, file_name)?;

    Ok(image)
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
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    let pages = service.get_chapter_pages(release_id).await?;

    Ok(json_data_response(StatusCode::OK, pages))
}

pub async fn get_staged_chapter_pages(
    State(service): State<Service>,
    Path(release_id): Path<Uuid>,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    let pages = service.get_staged_chapter_pages(release_id).await?;

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
