use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use http_body_util::BodyExt;

use crate::fakers::{BookFaker, CONTENT_RATINGS, CreatorFaker, LabelFaker, UserFaker};
use crate::helpers::{
    RespWrapper, TestApp, assert_error, assert_stored, creator_keys, label_keys, link_keys,
    title_keys,
};
use fake::Fake;
use manga_theka::entity::{BookQuery, Creator, Label, Role, UserQuery};

#[tokio::test]
async fn store_book_with_valid_body_passes() {
    let app = TestApp::new().await;
    let user: UserQuery = UserFaker::default().fake();
    app.insert_user(&user).await;
    let book: BookQuery = BookFaker {
        labels: 1..=5,
        links: 1..=5,
        titles: 1..=5,
        ..Default::default()
    }
    .fake();
    let creator: Creator = CreatorFaker.fake();
    app.insert_creator(&creator).await;

    let body = serde_json::json!({
        "name": book.name.as_ref(),
        "description": book.description.as_ref(),
        "publicationYear": book.publication_year,
        "contentRatingId": book.content_rating.id,
        "status": book.status.as_ref(),
        "kind": book.kind.as_ref(),
        "publicationLanguageId": book.publication_language.id,
        "publicationDemographic": book.publication_demographic.as_ref(),
        "labelIds": book.labels.as_slice().iter().map(|l| l.id).collect::<Vec<_>>(),
        "links": &book.links,
        "titles": &book.titles,
        "creators": [{ "creatorId": creator.id, "role": "Author" }],
    })
    .to_string();

    let req = Request::post("/books")
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            header::AUTHORIZATION,
            app.bearer(user.id, uuid::Uuid::now_v7(), &[Role::Reader]),
        )
        .body(Body::from(body))
        .unwrap();

    let resp = app.send(req).await;
    let id = assert_stored(resp).await;

    let got = app.fetch_book(id).await;

    assert_eq!(got.name, book.name);
    assert_eq!(got.description, book.description);
    assert_eq!(got.publication_year, book.publication_year);
    assert_eq!(got.content_rating, book.content_rating);
    assert_eq!(got.status, book.status);
    assert_eq!(got.kind, book.kind);
    assert_eq!(got.publication_language.id, book.publication_language.id);
    assert_eq!(got.publication_demographic, book.publication_demographic);
    assert_eq!(
        label_keys(got.labels.as_slice()),
        label_keys(book.labels.as_slice())
    );
    assert_eq!(
        link_keys(got.links.as_slice()),
        link_keys(book.links.as_slice())
    );
    assert_eq!(
        title_keys(got.titles.as_slice()),
        title_keys(book.titles.as_slice())
    );
    assert_eq!(
        creator_keys(got.creators.as_slice()),
        vec![(
            creator.id,
            creator.first_name.as_ref(),
            creator.last_name.as_ref(),
            vec!["Author"]
        )]
    );
    assert!(got.updated_at.is_none());
    assert_ne!(got.created_at, time::OffsetDateTime::UNIX_EPOCH);
    assert_eq!(got.created_by, user.id);
}

#[tokio::test]
async fn store_book_without_relations_passes() {
    let app = TestApp::new().await;
    let book: BookQuery = BookFaker::default().fake();

    let body = serde_json::json!({
        "name": book.name.as_ref(),
        "description": book.description.as_ref(),
        "publicationYear": book.publication_year,
        "contentRatingId": book.content_rating.id,
        "status": book.status.as_ref(),
        "kind": book.kind.as_ref(),
        "publicationLanguageId": book.publication_language.id,
        "publicationDemographic": book.publication_demographic.as_ref(),
    })
    .to_string();

    let req = Request::post("/books")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.send(req).await;
    let id = assert_stored(resp).await;

    let sample = app.fetch_book_sample(id).await;

    assert_eq!(sample.name.as_deref(), Some(book.name.as_ref()));
    assert_eq!(sample.labels, 0);
    assert_eq!(sample.links, 0);
    assert_eq!(sample.titles, 0);
}

#[tokio::test]
async fn store_book_with_broken_json_returns_400() {
    let app = TestApp::new().await;

    let req = Request::post("/books")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{not json"))
        .unwrap();

    let resp = app.send(req).await;
    assert_error(resp, StatusCode::BAD_REQUEST).await;
}

#[tokio::test]
async fn store_book_with_missing_name_returns_422() {
    let app = TestApp::new().await;
    let book: BookQuery = BookFaker::default().fake();

    let body = serde_json::json!({
        "description": book.description.as_ref(),
        "publicationYear": book.publication_year,
        "contentRatingId": book.content_rating.id,
        "status": book.status.as_ref(),
        "kind": book.kind.as_ref(),
        "publicationLanguageId": book.publication_language.id,
        "publicationDemographic": book.publication_demographic.as_ref(),
    })
    .to_string();

    let req = Request::post("/books")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.send(req).await;
    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;
}

#[tokio::test]
async fn store_book_with_unknown_status_returns_422() {
    let app = TestApp::new().await;
    let book: BookQuery = BookFaker::default().fake();

    let body = serde_json::json!({
        "name": book.name.as_ref(),
        "description": book.description.as_ref(),
        "publicationYear": book.publication_year,
        "contentRatingId": book.content_rating.id,
        "status": "abandoned",
        "kind": book.kind.as_ref(),
        "publicationLanguageId": book.publication_language.id,
        "publicationDemographic": book.publication_demographic.as_ref(),
    })
    .to_string();

    let req = Request::post("/books")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.send(req).await;
    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;
}

#[tokio::test]
async fn store_book_with_unknown_publication_demographic_returns_422() {
    let app = TestApp::new().await;
    let book: BookQuery = BookFaker::default().fake();

    let body = serde_json::json!({
        "name": book.name.as_ref(),
        "description": book.description.as_ref(),
        "publicationYear": book.publication_year,
        "contentRatingId": book.content_rating.id,
        "status": book.status.as_ref(),
        "kind": book.kind.as_ref(),
        "publicationLanguageId": book.publication_language.id,
        "publicationDemographic": "Unknown",
    })
    .to_string();

    let req = Request::post("/books")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.send(req).await;
    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;
}

#[tokio::test]
async fn store_book_without_publication_demographic_returns_422() {
    let app = TestApp::new().await;
    let book: BookQuery = BookFaker::default().fake();

    let body = serde_json::json!({
        "name": book.name.as_ref(),
        "description": book.description.as_ref(),
        "publicationYear": book.publication_year,
        "contentRatingId": book.content_rating.id,
        "status": book.status.as_ref(),
        "kind": book.kind.as_ref(),
        "publicationLanguageId": book.publication_language.id,
    })
    .to_string();

    let req = Request::post("/books")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.send(req).await;
    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;
}

#[tokio::test]
async fn store_book_with_unknown_label_id_returns_422() {
    let app = TestApp::new().await;
    let book: BookQuery = BookFaker::default().fake();

    let body = serde_json::json!({
        "name": book.name.as_ref(),
        "description": book.description.as_ref(),
        "publicationYear": book.publication_year,
        "contentRatingId": book.content_rating.id,
        "status": book.status.as_ref(),
        "kind": book.kind.as_ref(),
        "publicationLanguageId": book.publication_language.id,
        "publicationDemographic": book.publication_demographic.as_ref(),
        "labelIds": [uuid::Uuid::now_v7()],
    })
    .to_string();

    let req = Request::post("/books")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.send(req).await;
    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;

    let count = sqlx::query_scalar!(r#"select count(*) as "count!" from books"#)
        .fetch_one(&app.pool)
        .await
        .unwrap();

    assert_eq!(count, 0, "failed write must roll back book");
}

#[tokio::test]
async fn store_book_with_unknown_creator_id_returns_422() {
    let app = TestApp::new().await;
    let book: BookQuery = BookFaker::default().fake();

    let body = serde_json::json!({
        "name": book.name.as_ref(),
        "description": book.description.as_ref(),
        "publicationYear": book.publication_year,
        "contentRatingId": book.content_rating.id,
        "status": book.status.as_ref(),
        "kind": book.kind.as_ref(),
        "publicationLanguageId": book.publication_language.id,
        "publicationDemographic": book.publication_demographic.as_ref(),
        "creators": [{ "creatorId": uuid::Uuid::now_v7(), "role": "Author" }],
    })
    .to_string();

    let req = Request::post("/books")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.send(req).await;
    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;

    let count = sqlx::query_scalar!(r#"select count(*) as "count!" from books"#)
        .fetch_one(&app.pool)
        .await
        .unwrap();

    assert_eq!(count, 0, "failed write must roll back book");
}

#[tokio::test]
async fn store_book_with_dual_role_creator_merges_into_one_credit() {
    let app = TestApp::new().await;
    let book: BookQuery = BookFaker::default().fake();
    let creator: Creator = CreatorFaker.fake();
    app.insert_creator(&creator).await;

    let body = serde_json::json!({
        "name": book.name.as_ref(),
        "description": book.description.as_ref(),
        "publicationYear": book.publication_year,
        "contentRatingId": book.content_rating.id,
        "status": book.status.as_ref(),
        "kind": book.kind.as_ref(),
        "publicationLanguageId": book.publication_language.id,
        "publicationDemographic": book.publication_demographic.as_ref(),
        "creators": [
            { "creatorId": creator.id, "role": "Author" },
            { "creatorId": creator.id, "role": "Artist" },
        ],
    })
    .to_string();

    let req = Request::post("/books")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.send(req).await;
    let id = assert_stored(resp).await;

    let got = app.fetch_book(id).await;

    assert_eq!(
        creator_keys(got.creators.as_slice()),
        vec![(
            creator.id,
            creator.first_name.as_ref(),
            creator.last_name.as_ref(),
            vec!["Artist", "Author"]
        )],
        "a creator credited as both roles on one book must merge into a single entry"
    );
}

#[tokio::test]
async fn store_book_with_duplicate_creator_role_returns_422() {
    let app = TestApp::new().await;
    let book: BookQuery = BookFaker::default().fake();
    let creator: Creator = CreatorFaker.fake();
    app.insert_creator(&creator).await;

    let body = serde_json::json!({
        "name": book.name.as_ref(),
        "description": book.description.as_ref(),
        "publicationYear": book.publication_year,
        "contentRatingId": book.content_rating.id,
        "status": book.status.as_ref(),
        "kind": book.kind.as_ref(),
        "publicationLanguageId": book.publication_language.id,
        "publicationDemographic": book.publication_demographic.as_ref(),
        "creators": [
            { "creatorId": creator.id, "role": "Author" },
            { "creatorId": creator.id, "role": "Author" },
        ],
    })
    .to_string();

    let req = Request::post("/books")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.send(req).await;
    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;
}

#[tokio::test]
async fn get_book_with_valid_id_passes() {
    let app = TestApp::new().await;
    let book: BookQuery = BookFaker {
        labels: 1..=5,
        links: 1..=5,
        titles: 1..=5,
        creators: 1..=5,
        ..Default::default()
    }
    .fake();

    app.insert_book(&book).await;

    let req = Request::get(format!("/books/{}", book.id))
        .body(Body::empty())
        .unwrap();
    let resp = app.send(req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let wrapper: RespWrapper<BookQuery> = serde_json::from_slice(&bytes).unwrap();
    let got = wrapper.data;

    assert_eq!(got.id, book.id);
    assert_eq!(got.name, book.name);
    assert_eq!(got.description, book.description);
    assert_eq!(got.publication_year, book.publication_year);
    assert_eq!(got.content_rating, book.content_rating,);
    assert_eq!(got.status, book.status);
    assert_eq!(got.kind, book.kind);
    assert_eq!(got.publication_language.id, book.publication_language.id);
    assert_eq!(
        got.publication_language.code,
        book.publication_language.code,
    );
    assert_eq!(
        got.publication_language.name,
        book.publication_language.name
    );
    assert_eq!(got.publication_demographic, book.publication_demographic);
    assert_eq!(
        label_keys(got.labels.as_slice()),
        label_keys(book.labels.as_slice())
    );
    assert_eq!(
        link_keys(got.links.as_slice()),
        link_keys(book.links.as_slice())
    );
    assert_eq!(
        title_keys(got.titles.as_slice()),
        title_keys(book.titles.as_slice())
    );
    assert_eq!(
        creator_keys(got.creators.as_slice()),
        creator_keys(book.creators.as_slice())
    );
    assert_eq!(got.updated_at, book.updated_at);
    assert_eq!(
        got.created_at.unix_timestamp(),
        book.created_at.unix_timestamp()
    );
}

#[tokio::test]
async fn get_book_with_unknown_id_returns_404() {
    let app = TestApp::new().await;

    let req = Request::get(format!("/books/{}", uuid::Uuid::now_v7()))
        .body(Body::empty())
        .unwrap();

    let resp = app.send(req).await;

    assert_error(resp, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn get_book_with_malformed_id_returns_400() {
    let app = TestApp::new().await;

    let req = Request::get("/books/not-a-uuid")
        .body(Body::empty())
        .unwrap();

    let resp = app.send(req).await;
    assert_error(resp, StatusCode::BAD_REQUEST).await;
}

#[tokio::test]
async fn update_book_with_valid_body_passes() {
    let app = TestApp::new().await;
    let book: BookQuery = BookFaker {
        labels: 1..=5,
        links: 1..=5,
        titles: 1..=5,
        ..Default::default()
    }
    .fake();

    app.insert_book(&book).await;

    let updated: BookQuery = BookFaker {
        labels: 1..=5,
        links: 1..=5,
        titles: 1..=5,
        ..Default::default()
    }
    .fake();

    let body = serde_json::json!({
        "name": updated.name.as_ref(),
        "description": updated.description.as_ref(),
        "publicationYear": updated.publication_year,
        "contentRatingId": updated.content_rating.id,
        "status": updated.status.as_ref(),
        "kind": updated.kind.as_ref(),
        "publicationLanguageId": updated.publication_language.id,
        "publicationDemographic": updated.publication_demographic.as_ref(),
        "labelIds": updated.labels.as_slice().iter().map(|l| l.id).collect::<Vec<_>>(),
        "links": &updated.links,
        "titles": &updated.titles,
    })
    .to_string();

    let req = Request::put(format!("/books/{}", book.id))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();
    let resp = app.send(req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let got = app.fetch_book(book.id).await;

    assert_eq!(got.name, updated.name);
    assert_eq!(got.description, updated.description);
    assert_eq!(got.publication_year, updated.publication_year);
    assert_eq!(got.content_rating, updated.content_rating);
    assert_eq!(got.status, updated.status);
    assert_eq!(got.kind, updated.kind);
    assert_eq!(got.publication_language.id, updated.publication_language.id);
    assert_eq!(got.publication_demographic, updated.publication_demographic);

    assert_eq!(
        label_keys(got.labels.as_slice()),
        label_keys(updated.labels.as_slice()),
        "arrays must be replaced wholesale"
    );
    assert_eq!(
        link_keys(got.links.as_slice()),
        link_keys(updated.links.as_slice()),
        "arrays must be replaced wholesale"
    );
    assert_eq!(
        title_keys(got.titles.as_slice()),
        title_keys(updated.titles.as_slice()),
        "arrays must be replaced wholesale"
    );
    assert!(got.updated_at.is_some());
    assert_eq!(
        got.created_at.unix_timestamp(),
        book.created_at.unix_timestamp(),
        "created_at must stay immutable"
    );
}

#[tokio::test]
async fn update_book_swaps_the_content_rating() {
    let app = TestApp::new().await;
    let mut book: BookQuery = BookFaker::default().fake();
    book.content_rating = CONTENT_RATINGS
        .first()
        .expect("content rating to swap from must exist")
        .clone();

    app.insert_book(&book).await;

    let other = CONTENT_RATINGS
        .get(1)
        .expect("content rating to swap to must exist")
        .clone();

    let updated_body = serde_json::json!({
        "name": book.name.as_ref(),
        "description": book.description.as_ref(),
        "publicationYear": book.publication_year,
        "contentRatingId": other.id,
        "status": book.status.as_ref(),
        "kind": book.kind.as_ref(),
        "publicationLanguageId": book.publication_language.id,
        "publicationDemographic": book.publication_demographic.as_ref(),
    })
    .to_string();

    let req = Request::put(format!("/books/{}", book.id))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(updated_body))
        .unwrap();
    let resp = app.send(req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let got = app.fetch_book(book.id).await;

    assert_eq!(got.content_rating, other);
}

#[tokio::test]
async fn update_book_with_empty_arrays_detaches_everything() {
    let app = TestApp::new().await;
    let book: BookQuery = BookFaker {
        labels: 1..=5,
        links: 1..=5,
        titles: 1..=5,
        ..Default::default()
    }
    .fake();

    app.insert_book(&book).await;

    let detached = serde_json::json!({
        "name": book.name.as_ref(),
        "description": book.description.as_ref(),
        "publicationYear": book.publication_year,
        "contentRatingId": book.content_rating.id,
        "status": book.status.as_ref(),
        "kind": book.kind.as_ref(),
        "publicationLanguageId": book.publication_language.id,
        "publicationDemographic": book.publication_demographic.as_ref(),
        "labelIds": [],
        "links": [],
        "titles": [],
    })
    .to_string();

    let req = Request::put(format!("/books/{}", book.id))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(detached))
        .unwrap();
    let resp = app.send(req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let sample = app.fetch_book_sample(book.id).await;

    assert_eq!(sample.labels, 0);
    assert_eq!(sample.links, 0);
    assert_eq!(sample.titles, 0);
}

#[tokio::test]
async fn update_book_leaves_other_books_untouched() {
    let app = TestApp::new().await;
    let book: BookQuery = BookFaker::default().fake();
    let other: BookQuery = BookFaker::default().fake();

    app.insert_book(&book).await;
    app.insert_book(&other).await;

    let renamed: BookQuery = BookFaker::default().fake();
    let updated_body = serde_json::json!({
        "name": renamed.name.as_ref(),
        "description": book.description.as_ref(),
        "publicationYear": book.publication_year,
        "contentRatingId": book.content_rating.id,
        "status": book.status.as_ref(),
        "kind": book.kind.as_ref(),
        "publicationLanguageId": book.publication_language.id,
        "publicationDemographic": book.publication_demographic.as_ref(),
    })
    .to_string();

    let req = Request::put(format!("/books/{}", book.id))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(updated_body))
        .unwrap();
    let resp = app.send(req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let sample = app.fetch_book_sample(other.id).await;

    assert_eq!(sample.name.as_deref(), Some(other.name.as_ref()));
    assert!(sample.updated_at.is_none());
    assert_eq!(sample.labels, other.labels.len() as i64);
    assert_eq!(sample.links, other.links.len() as i64);
    assert_eq!(sample.titles, other.titles.len() as i64);
}

#[tokio::test]
async fn update_book_with_unknown_id_returns_404() {
    let app = TestApp::new().await;
    let book: BookQuery = BookFaker::default().fake();

    let body = serde_json::json!({
        "name": book.name.as_ref(),
        "description": book.description.as_ref(),
        "publicationYear": book.publication_year,
        "contentRatingId": book.content_rating.id,
        "status": book.status.as_ref(),
        "kind": book.kind.as_ref(),
        "publicationLanguageId": book.publication_language.id,
        "publicationDemographic": book.publication_demographic.as_ref(),
    })
    .to_string();

    let req = Request::put(format!("/books/{}", uuid::Uuid::now_v7()))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.send(req).await;
    assert_error(resp, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn delete_book_with_valid_id_passes() {
    let app = TestApp::new().await;
    let book: BookQuery = BookFaker {
        labels: 1..=5,
        links: 1..=5,
        titles: 1..=5,
        ..Default::default()
    }
    .fake();

    app.insert_book(&book).await;

    let req = Request::delete(format!("/books/{}", book.id))
        .body(Body::empty())
        .unwrap();

    let resp = app.send(req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let sample = app.fetch_book_sample(book.id).await;

    assert!(sample.name.is_none());
    assert_eq!(sample.labels, 0);
    assert_eq!(sample.links, 0);
    assert_eq!(sample.titles, 0);
}

#[tokio::test]
async fn delete_book_leaves_other_books_untouched() {
    let app = TestApp::new().await;
    let labels: Label = LabelFaker.fake();

    let mut book: BookQuery = BookFaker {
        labels: 0..=0,
        ..Default::default()
    }
    .fake();
    book.labels = vec![labels.clone()].try_into().unwrap();

    let mut other: BookQuery = BookFaker {
        labels: 0..=0,
        ..Default::default()
    }
    .fake();
    other.labels = vec![labels].try_into().unwrap();

    app.insert_book(&book).await;
    app.insert_book(&other).await;

    let req = Request::delete(format!("/books/{}", book.id))
        .body(Body::empty())
        .unwrap();
    let resp = app.send(req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let sample = app.fetch_book_sample(other.id).await;

    assert_eq!(sample.name.as_deref(), Some(other.name.as_ref()));
    assert_eq!(sample.labels, other.labels.len() as i64);
    assert_eq!(sample.links, other.links.len() as i64);
    assert_eq!(sample.titles, other.titles.len() as i64);
}

#[tokio::test]
async fn delete_book_with_unknown_id_returns_404() {
    let app = TestApp::new().await;

    let req = Request::delete(format!("/books/{}", uuid::Uuid::now_v7()))
        .body(Body::empty())
        .unwrap();

    let resp = app.send(req).await;

    assert_error(resp, StatusCode::NOT_FOUND).await;
}
