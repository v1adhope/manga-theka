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
        AlternativeTitle, Book, BookKind, BookLink, BookLinkKind, BookName, BookStatus, BookWrite,
        DEFAULT_LIMIT, Description, Limit, LinkUrl, Pagination, Titles,
    },
    error::{AppError, EntityError},
    route::{StoreResp, json_data_response, json_response},
    service::Service,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookLinkReq {
    #[serde(rename = "type")]
    pub kind: BookLinkKind,
    pub url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlternativeTitleReq {
    pub language_id: Uuid,
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookReq {
    pub name: String,
    pub description: String,
    pub publication_year: i16,
    pub content_rating: Uuid,
    pub status: BookStatus,
    #[serde(rename = "type")]
    pub kind: BookKind,
    pub publication_language: Uuid,
    #[serde(default)]
    pub label_ids: Vec<Uuid>,
    #[serde(default)]
    pub links: Vec<BookLinkReq>,
    #[serde(default)]
    pub titles: Vec<AlternativeTitleReq>,
}

impl TryFrom<(BookReq, Uuid, Option<OffsetDateTime>, OffsetDateTime)> for BookWrite {
    type Error = EntityError;

    fn try_from(
        ctx: (BookReq, Uuid, Option<OffsetDateTime>, OffsetDateTime),
    ) -> Result<Self, Self::Error> {
        let (req, id, updated_at, created_at) = ctx;

        let book = Book {
            id,
            name: BookName::try_from(req.name)?,
            description: Description::try_from(req.description)?,
            publication_year: req.publication_year,
            content_rating: req.content_rating,
            status: req.status,
            kind: req.kind,
            publication_language: req.publication_language,
            updated_at,
            created_at,
        };

        let mut links = Vec::with_capacity(req.links.len());
        for link in req.links {
            links.push(BookLink {
                id: Uuid::now_v7(),
                kind: link.kind,
                url: LinkUrl::try_from(link.url)?,
            });
        }

        let mut titles = Vec::with_capacity(req.titles.len());
        for title in req.titles {
            titles.push(AlternativeTitle {
                id: Uuid::now_v7(),
                language_id: title.language_id,
                name: BookName::try_from(title.name)?,
            });
        }

        Ok(Self {
            book,
            label_ids: req.label_ids,
            links,
            titles: Titles::try_from(titles)?,
        })
    }
}

pub async fn store_book(
    State(service): State<Service>,
    Json(req): Json<BookReq>,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    let id = Uuid::now_v7();
    let created_at = OffsetDateTime::now_utc();
    let book: BookWrite = (req, id, None, created_at).try_into()?;

    service.store_book(book).await?;

    Ok(json_data_response(StatusCode::CREATED, StoreResp { id }))
}

pub async fn update_book(
    State(service): State<Service>,
    Path(id): Path<Uuid>,
    Json(req): Json<BookReq>,
) -> Result<StatusCode, AppError> {
    let updated_at = OffsetDateTime::now_utc();
    let book: BookWrite = (req, id, Some(updated_at), OffsetDateTime::UNIX_EPOCH).try_into()?;

    service.update_book(book).await?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBooksQuery {
    pub after: Option<Uuid>,
    pub limit: Option<u32>,
}

impl TryFrom<GetBooksQuery> for Pagination {
    type Error = EntityError;

    fn try_from(q: GetBooksQuery) -> Result<Self, Self::Error> {
        let limit = Limit::try_from(q.limit.unwrap_or(DEFAULT_LIMIT))?;
        Ok(Self {
            after: q.after,
            limit,
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBooksResp {
    pub data: Vec<Book>,
    pub next_cursor: Option<Uuid>,
}

pub async fn get_books(
    State(service): State<Service>,
    Query(query): Query<GetBooksQuery>,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    let pg: Pagination = query.try_into()?;

    let (data, next_cursor) = service.get_books(pg).await?;

    Ok(json_response(
        StatusCode::OK,
        GetBooksResp { data, next_cursor },
    ))
}

pub async fn get_book(
    State(service): State<Service>,
    Path(id): Path<Uuid>,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    let book = service.get_book(id).await?;
    Ok(json_data_response(StatusCode::OK, book))
}

pub async fn delete_book(
    State(service): State<Service>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    service.delete_book(id).await?;
    Ok(StatusCode::NO_CONTENT)
}
