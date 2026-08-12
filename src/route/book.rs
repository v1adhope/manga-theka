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
        AlternativeTitle, Book, BookKind, BookLink, BookLinkKind, BookName, BookStatus,
        ContentRating, Description, Language, LinkUrl, Pagination,
    },
    error::{AppError, EntityError},
    route::{PaginationQuery, StoreResp, json_data_response, json_response},
    service::Service,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookLinkReq {
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
    pub kind: BookKind,
    pub publication_language: Uuid,
    #[serde(default)]
    pub label_ids: Vec<Uuid>,
    #[serde(default)]
    pub links: Vec<BookLinkReq>,
    #[serde(default)]
    pub titles: Vec<AlternativeTitleReq>,
}

struct BookWithRelations {
    req: BookReq,
    id: Uuid,
    updated_at: Option<OffsetDateTime>,
    created_at: OffsetDateTime,
}

impl TryFrom<BookWithRelations> for (Book, Vec<Uuid>) {
    type Error = EntityError;

    fn try_from(item: BookWithRelations) -> Result<Self, Self::Error> {
        let BookWithRelations {
            req,
            id,
            updated_at,
            created_at,
        } = item;

        let BookReq {
            name,
            description,
            publication_year,
            content_rating,
            status,
            kind,
            publication_language,
            label_ids,
            links: link_reqs,
            titles: title_reqs,
        } = req;

        let mut links = Vec::with_capacity(link_reqs.len());
        for link in link_reqs {
            links.push(BookLink {
                kind: link.kind,
                url: LinkUrl::try_from(link.url)?,
            });
        }

        let mut titles = Vec::with_capacity(title_reqs.len());
        for title in title_reqs {
            titles.push(AlternativeTitle {
                language_id: title.language_id,
                name: BookName::try_from(title.name)?,
            });
        }

        let book = Book {
            id,
            name: BookName::try_from(name)?,
            description: Description::try_from(description)?,
            publication_year,
            content_rating: ContentRating {
                id: content_rating,
                ..Default::default()
            },
            status,
            kind,
            publication_language: Language {
                id: publication_language,
                ..Default::default()
            },
            labels: Vec::new(),
            links,
            titles,
            creators: Vec::new(),
            updated_at,
            created_at,
        };

        Ok((book, label_ids))
    }
}

pub async fn store_book(
    State(service): State<Service>,
    Json(req): Json<BookReq>,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    let id = Uuid::now_v7();
    let created_at = OffsetDateTime::now_utc();
    let (book, label_ids): (Book, Vec<Uuid>) = BookWithRelations {
        req,
        id,
        updated_at: None,
        created_at,
    }
    .try_into()?;

    service.store_book(book, label_ids).await?;

    Ok(json_data_response(StatusCode::CREATED, StoreResp { id }))
}

pub async fn update_book(
    State(service): State<Service>,
    Path(id): Path<Uuid>,
    Json(req): Json<BookReq>,
) -> Result<StatusCode, AppError> {
    let updated_at = OffsetDateTime::now_utc();
    let (book, label_ids): (Book, Vec<Uuid>) = BookWithRelations {
        req,
        id,
        updated_at: Some(updated_at),
        created_at: OffsetDateTime::UNIX_EPOCH,
    }
    .try_into()?;

    service.update_book(book, label_ids).await?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBooksResp {
    pub data: Vec<Book>,
    pub next_cursor: Option<Uuid>,
}

pub async fn get_books(
    State(service): State<Service>,
    Query(query): Query<PaginationQuery>,
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
