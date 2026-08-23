use axum::{
    Json,
    extract::{Multipart, Path, State, multipart::Field},
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
};
use bytes::{Bytes, BytesMut};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    entity::{
        ChapterPage, ChapterRelease, MAX_PARTS_PER_REQUEST, MAX_RELEASE_ROWS, PAGE_MAX_BYTES,
        PageExtension, PageOrder, ReleaseVersion, StagedPageQuery,
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
) -> Result<impl IntoResponse, AppError> {
    let release = service.get_chapter_release(id).await?;
    let etag = release.version.to_string();

    Ok((
        StatusCode::OK,
        [(header::ETAG, etag)],
        Json(serde_json::json!({ "data": release })),
    ))
}

pub async fn upload_chapter_pages(
    State(service): State<Service>,
    Path(release_id): Path<Uuid>,
    mut multipart: Multipart,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    service.ensure_chapter_release_exists(release_id).await?;

    let mut rows = service.count_chapter_pages(release_id).await? as usize;
    let mut staged: Vec<StagedPageQuery> = Vec::new();

    while let Some(field) = multipart.next_field().await? {
        if staged.len() >= MAX_PARTS_PER_REQUEST {
            return Err(EntityError::UploadPartsExceedLimit(
                staged.len() + 1,
                MAX_PARTS_PER_REQUEST,
            )
            .into());
        }
        if rows >= MAX_RELEASE_ROWS {
            return Err(EntityError::ReleaseRowsExceedLimit(rows, MAX_RELEASE_ROWS).into());
        }

        let content = collect_part(field).await?;
        let extension = PageExtension::try_from(content.as_ref())?;
        let page = ChapterPage {
            id: Uuid::now_v7(),
            release_id,
            extension,
            content,
        };

        service.store_chapter_page(&page).await?;

        rows += 1;
        staged.push(StagedPageQuery {
            id: page.id,
            extension: page.extension,
        });
    }

    Ok(json_data_response(StatusCode::CREATED, staged))
}

async fn collect_part(mut field: Field<'_>) -> Result<Bytes, AppError> {
    let mut content = BytesMut::new();

    while let Some(chunk) = field.chunk().await? {
        if content.len() + chunk.len() > PAGE_MAX_BYTES {
            return Err(EntityError::PageExceedsByteLimit(PAGE_MAX_BYTES).into());
        }

        content.extend_from_slice(&chunk);
    }

    Ok(content.freeze())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitReq {
    pub page_order: Vec<Uuid>,
}

pub async fn commit_chapter_release(
    State(service): State<Service>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<CommitReq>,
) -> Result<StatusCode, AppError> {
    let version = if_match(&headers)?;
    let order = PageOrder::try_from(req.page_order)?;

    service.commit_chapter_release(id, version, &order).await?;

    Ok(StatusCode::NO_CONTENT)
}

fn if_match(headers: &HeaderMap) -> Result<ReleaseVersion, EntityError> {
    let raw = headers
        .get(header::IF_MATCH)
        .ok_or(EntityError::ReleaseVersionIsAbsent)?;

    raw.to_str()
        .map_err(|_| {
            EntityError::ReleaseVersionIsMalformed(
                String::from_utf8_lossy(raw.as_bytes()).into_owned(),
            )
        })?
        .parse()
}

pub async fn get_chapter_pages(
    State(service): State<Service>,
    Path(release_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let (pages, version) = service.get_chapter_pages(release_id).await?;
    let etag = version.to_string();

    Ok((
        StatusCode::OK,
        [(header::ETAG, etag)],
        Json(serde_json::json!({ "data": pages })),
    ))
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
