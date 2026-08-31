use axum::{
    Json,
    extract::{Multipart, Path, Query, State},
    http::{StatusCode, header},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    entity::{
        AlternativeTitle, Book, BookCover, BookCreator, BookCreators, BookFilter, BookKind,
        BookLabelIds, BookLink, BookLinkKind, BookLinks, BookName, BookQuery, BookStatus,
        BookTitles, BookVisibility, CreatorRole, Filter, LinkUrl, Text, UserClaims,
        VisibilityTransition,
    },
    error::{AppError, EntityError, RouteError},
    route::{StoreResp, collect_image_part, json_data_response, json_response},
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
pub struct BookCreatorReq {
    pub creator_id: Uuid,
    pub role: CreatorRole,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookReq {
    pub name: String,
    pub description: String,
    pub publication_year: i16,
    pub content_rating_id: Uuid,
    pub status: BookStatus,
    pub kind: BookKind,
    pub publication_language_id: Uuid,
    #[serde(default)]
    pub label_ids: Vec<Uuid>,
    #[serde(default)]
    pub links: Vec<BookLinkReq>,
    #[serde(default)]
    pub titles: Vec<AlternativeTitleReq>,
    #[serde(default)]
    pub creators: Vec<BookCreatorReq>,
}

struct BookWithRelations {
    req: BookReq,
    id: Uuid,
    updated_at: Option<OffsetDateTime>,
    created_at: OffsetDateTime,
}

impl TryFrom<BookWithRelations> for Book {
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
            content_rating_id,
            status,
            kind,
            publication_language_id,
            label_ids,
            links: link_reqs,
            titles: title_reqs,
            creators: creator_reqs,
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

        let creators: Vec<BookCreator> = creator_reqs
            .into_iter()
            .map(|c| BookCreator {
                creator_id: c.creator_id,
                role: c.role,
            })
            .collect();

        Ok(Book {
            id,
            name: BookName::try_from(name)?,
            description: Text::try_from(description)?,
            publication_year,
            content_rating_id,
            status,
            kind,
            publication_language_id,
            label_ids: BookLabelIds::try_from(label_ids)?,
            links: BookLinks::try_from(links)?,
            titles: BookTitles::try_from(titles)?,
            creators: BookCreators::try_from(creators)?,
            updated_at,
            created_at,
        })
    }
}

// deferred: gate to any signed-in User; record the caller as the submitter
pub async fn store_book(
    State(service): State<Service>,
    Json(req): Json<BookReq>,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    let id = Uuid::now_v7();
    let created_at = OffsetDateTime::now_utc();
    let book: Book = BookWithRelations {
        req,
        id,
        updated_at: None,
        created_at,
    }
    .try_into()?;

    service.store_book(book).await?;

    Ok(json_data_response(StatusCode::CREATED, StoreResp { id }))
}

// deferred: gate to Uploader/Moderator/Admin, plus the submitter on their own Draft
pub async fn update_book(
    State(service): State<Service>,
    Path(id): Path<Uuid>,
    Json(req): Json<BookReq>,
) -> Result<StatusCode, AppError> {
    let updated_at = OffsetDateTime::now_utc();
    let book: Book = BookWithRelations {
        req,
        id,
        updated_at: Some(updated_at),
        created_at: OffsetDateTime::UNIX_EPOCH,
    }
    .try_into()?;

    service.update_book(book).await?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBooksResp {
    pub data: Vec<BookQuery>,
    pub next_cursor: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookListQuery {
    pub visibility: Option<BookVisibility>,
    pub after: Option<Uuid>,
    pub limit: Option<u32>,
}

impl TryFrom<BookListQuery> for BookFilter {
    type Error = EntityError;

    fn try_from(q: BookListQuery) -> Result<Self, Self::Error> {
        let page = Filter::builder().after(q.after).limit(q.limit).build()?;

        Ok(Self {
            page,
            visibility: q.visibility,
        })
    }
}

pub async fn get_books(
    State(service): State<Service>,
    Query(query): Query<BookListQuery>,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    // deferred: gate the ?visibility= override to Moderator/Admin
    let filter: BookFilter = query.try_into()?;

    let (data, next_cursor) = service.get_books(filter).await?;

    Ok(json_response(
        StatusCode::OK,
        GetBooksResp { data, next_cursor },
    ))
}

// deferred: scope non-Listed reads to the submitter or a Moderator
pub async fn get_book(
    State(service): State<Service>,
    Path(id): Path<Uuid>,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    let book = service.get_book(id).await?;
    Ok(json_data_response(StatusCode::OK, book))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookVisibilityReq {
    pub visibility: BookVisibility,
    pub note: Option<String>,
}

struct BookVisibilityWithContext {
    req: BookVisibilityReq,
    id: Uuid,
    now: OffsetDateTime,
}

impl TryFrom<BookVisibilityWithContext> for VisibilityTransition {
    type Error = EntityError;

    fn try_from(item: BookVisibilityWithContext) -> Result<Self, Self::Error> {
        let BookVisibilityWithContext { req, id, now } = item;
        let BookVisibilityReq { visibility, note } = req;

        Ok(Self {
            id,
            visibility,
            note: note.map(Text::try_from).transpose()?,
            now,
        })
    }
}

// deferred: gate to the submitter for Draft -> PendingReview, Moderator/Admin otherwise
pub async fn update_book_visibility(
    State(service): State<Service>,
    Path(id): Path<Uuid>,
    Json(req): Json<BookVisibilityReq>,
) -> Result<StatusCode, AppError> {
    let transition: VisibilityTransition = BookVisibilityWithContext {
        req,
        id,
        now: OffsetDateTime::now_utc(),
    }
    .try_into()?;

    service.set_book_visibility(transition).await?;

    Ok(StatusCode::NO_CONTENT)
}

// deferred: gate to Moderator/Admin
pub async fn delete_book(
    State(service): State<Service>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    service.delete_book(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// deferred: gate to Uploader/Moderator/Admin
pub async fn store_book_cover(
    State(service): State<Service>,
    Path(book_id): Path<Uuid>,
    mut multipart: Multipart,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    let field = multipart
        .next_field()
        .await
        .map_err(RouteError::from)?
        .ok_or(RouteError::ImagePartMissing)?;

    let image = collect_image_part(field).await?;

    let id = image.id;
    let cover = BookCover { book_id, image };

    service.store_book_cover(cover).await?;

    Ok(json_data_response(StatusCode::CREATED, StoreResp { id }))
}

pub async fn get_book_covers(
    State(service): State<Service>,
    Path(book_id): Path<Uuid>,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    let covers = service.get_book_covers(book_id).await?;

    Ok(json_data_response(StatusCode::OK, covers))
}

// deferred: also admit the book's submitter once `books` records one
pub async fn get_book_cover_image(
    State(service): State<Service>,
    Path(cover_id): Path<Uuid>,
    claims: UserClaims,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    let url = service.presign_book_cover(cover_id, &claims).await?;

    Ok((StatusCode::FOUND, [(header::LOCATION, url)]))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MainCoverReq {
    pub cover_id: Uuid,
}

// deferred: gate to Uploader/Moderator/Admin
pub async fn update_book_main_cover(
    State(service): State<Service>,
    Path(book_id): Path<Uuid>,
    Json(req): Json<MainCoverReq>,
) -> Result<StatusCode, AppError> {
    service.promote_book_cover(book_id, req.cover_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// deferred: gate to Uploader/Moderator/Admin
pub async fn delete_book_cover(
    State(service): State<Service>,
    Path(cover_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    service.delete_book_cover(cover_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
