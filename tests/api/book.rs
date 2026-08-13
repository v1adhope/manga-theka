use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use http_body_util::BodyExt;
use tower::ServiceExt;

use crate::fakers::{BookFaker, CONTENT_RATINGS, LabelFaker};
use crate::helpers::{
    RespWrapper, TestApp, assert_error, assert_stored, label_keys, link_keys, title_keys,
};
use fake::Fake;
use manga_theka::entity::{Book, Label};

#[tokio::test]
async fn store_book_with_valid_body_passes() {
    let app = TestApp::new().await;
    let book: Book = BookFaker {
        labels: 1..=5,
        links: 1..=5,
        titles: 1..=5,
        ..Default::default()
    }
    .fake();

    let body = serde_json::json!({
        "name": book.name.as_ref(),
        "description": book.description.as_ref(),
        "publicationYear": book.publication_year,
        "contentRating": book.content_rating.id,
        "status": book.status.as_ref(),
        "kind": book.kind.as_ref(),
        "publicationLanguage": book.publication_language.id,
        "labelIds": book.labels.iter().map(|l| l.id).collect::<Vec<_>>(),
        "links": &book.links,
        "titles": &book.titles,
    })
    .to_string();

    let req = Request::post("/books")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.router.clone().oneshot(req).await.unwrap();
    let id = assert_stored(resp).await;

    let got = app.fetch_book(id).await;

    assert_eq!(got.name, book.name);
    assert_eq!(got.description, book.description);
    assert_eq!(got.publication_year, book.publication_year);
    assert_eq!(got.content_rating, book.content_rating);
    assert_eq!(got.status, book.status);
    assert_eq!(got.kind, book.kind);
    assert_eq!(got.publication_language.id, book.publication_language.id);
    assert_eq!(label_keys(&got.labels), label_keys(&book.labels));
    assert_eq!(link_keys(&got.links), link_keys(&book.links));
    assert_eq!(title_keys(&got.titles), title_keys(&book.titles));
    assert!(got.creators.is_empty());
    assert!(got.updated_at.is_none());
    assert_ne!(got.created_at, time::OffsetDateTime::UNIX_EPOCH);
}

#[tokio::test]
async fn store_book_without_relations_passes() {
    let app = TestApp::new().await;
    let book: Book = BookFaker::default().fake();

    let body = serde_json::json!({
        "name": book.name.as_ref(),
        "description": book.description.as_ref(),
        "publicationYear": book.publication_year,
        "contentRating": book.content_rating.id,
        "status": book.status.as_ref(),
        "kind": book.kind.as_ref(),
        "publicationLanguage": book.publication_language.id,
    })
    .to_string();

    let req = Request::post("/books")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.router.clone().oneshot(req).await.unwrap();
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

    let resp = app.router.oneshot(req).await.unwrap();
    assert_error(resp, StatusCode::BAD_REQUEST).await;
}

#[tokio::test]
async fn store_book_with_missing_name_returns_422() {
    let app = TestApp::new().await;
    let book: Book = BookFaker::default().fake();

    let body = serde_json::json!({
        "description": book.description.as_ref(),
        "publicationYear": book.publication_year,
        "contentRating": book.content_rating.id,
        "status": book.status.as_ref(),
        "kind": book.kind.as_ref(),
        "publicationLanguage": book.publication_language.id,
    })
    .to_string();

    let req = Request::post("/books")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();
    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;
}

#[tokio::test]
async fn store_book_with_unknown_status_returns_422() {
    let app = TestApp::new().await;
    let book: Book = BookFaker::default().fake();

    let body = serde_json::json!({
        "name": book.name.as_ref(),
        "description": book.description.as_ref(),
        "publicationYear": book.publication_year,
        "contentRating": book.content_rating.id,
        "status": "abandoned",
        "kind": book.kind.as_ref(),
        "publicationLanguage": book.publication_language.id,
    })
    .to_string();

    let req = Request::post("/books")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();
    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;
}

#[tokio::test]
async fn store_book_with_unknown_label_id_returns_422() {
    let app = TestApp::new().await;
    let book: Book = BookFaker::default().fake();

    let body = serde_json::json!({
        "name": book.name.as_ref(),
        "description": book.description.as_ref(),
        "publicationYear": book.publication_year,
        "contentRating": book.content_rating.id,
        "status": book.status.as_ref(),
        "kind": book.kind.as_ref(),
        "publicationLanguage": book.publication_language.id,
        "labelIds": [uuid::Uuid::now_v7()],
    })
    .to_string();

    let req = Request::post("/books")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();
    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;

    let count = sqlx::query_scalar!(r#"select count(*) as "count!" from books"#)
        .fetch_one(&app.pool)
        .await
        .unwrap();

    assert_eq!(count, 0, "failed write must roll back book");
}

#[tokio::test]
async fn get_book_with_valid_id_passes() {
    let app = TestApp::new().await;
    let book: Book = BookFaker {
        labels: 1..=5,
        links: 1..=5,
        titles: 1..=5,
        ..Default::default()
    }
    .fake();

    app.insert_book(&book).await;

    let req = Request::get(format!("/books/{}", book.id))
        .body(Body::empty())
        .unwrap();
    let resp = app.router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let wrapper: RespWrapper<Book> = serde_json::from_slice(&bytes).unwrap();
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
    assert_eq!(label_keys(&got.labels), label_keys(&book.labels));
    assert_eq!(link_keys(&got.links), link_keys(&book.links));
    assert_eq!(title_keys(&got.titles), title_keys(&book.titles));
    assert!(got.creators.is_empty());
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

    let resp = app.router.oneshot(req).await.unwrap();

    assert_error(resp, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn get_book_with_malformed_id_returns_400() {
    let app = TestApp::new().await;

    let req = Request::get("/books/not-a-uuid")
        .body(Body::empty())
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();
    assert_error(resp, StatusCode::BAD_REQUEST).await;
}

#[tokio::test]
async fn get_books_embeds_each_books_own_arrays() {
    let app = TestApp::new().await;
    let bare: Book = BookFaker {
        labels: 0..=0,
        links: 0..=0,
        titles: 0..=0,
        ..Default::default()
    }
    .fake();

    let full: Book = BookFaker {
        labels: 1..=5,
        links: 1..=5,
        titles: 1..=5,
        ..Default::default()
    }
    .fake();

    app.insert_book(&bare).await;
    app.insert_book(&full).await;

    let req = Request::get("/books").body(Body::empty()).unwrap();
    let resp = app.router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let listed: RespWrapper<Vec<Book>> = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(listed.data.len(), 2);

    let got_full = listed.data.iter().find(|b| b.id == full.id).unwrap();
    assert_eq!(label_keys(&got_full.labels), label_keys(&full.labels));
    assert_eq!(link_keys(&got_full.links), link_keys(&full.links));
    assert_eq!(title_keys(&got_full.titles), title_keys(&full.titles));

    let got_bare = listed.data.iter().find(|b| b.id == bare.id).unwrap();
    assert_eq!(got_bare.content_rating, bare.content_rating);
    assert!(got_bare.labels.is_empty());
    assert!(got_bare.links.is_empty());
    assert!(got_bare.titles.is_empty());
    assert!(got_bare.creators.is_empty());
}

#[tokio::test]
async fn get_books_returns_default_limit_and_next_cursor() {
    let app = TestApp::new().await;

    for _ in 0..25 {
        let book: Book = BookFaker::default().fake();
        app.insert_book(&book).await;
    }

    let req = Request::get("/books").body(Body::empty()).unwrap();
    let resp = app.router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let listed: RespWrapper<Vec<Book>> = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(listed.data.len(), 20);
    assert!(listed.next_cursor.is_some());
}

#[tokio::test]
async fn get_books_with_after_and_limit_3_returns_next_page() {
    let app = TestApp::new().await;

    for _ in 0..6 {
        let book: Book = BookFaker::default().fake();
        app.insert_book(&book).await;
    }

    let req = Request::get("/books?limit=3").body(Body::empty()).unwrap();
    let resp = app.router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let first: RespWrapper<Vec<Book>> = serde_json::from_slice(&bytes).unwrap();

    let ids: Vec<uuid::Uuid> = first.data.iter().map(|b| b.id).collect();
    let cursor = first.next_cursor.unwrap();

    let second_req = Request::get(format!("/books?limit=3&after={cursor}"))
        .body(Body::empty())
        .unwrap();
    let second_resp = app.router.oneshot(second_req).await.unwrap();
    assert_eq!(second_resp.status(), StatusCode::OK);

    let second_bytes = second_resp.into_body().collect().await.unwrap().to_bytes();
    let second: RespWrapper<Vec<Book>> = serde_json::from_slice(&second_bytes).unwrap();
    let second_ids: Vec<uuid::Uuid> = second.data.iter().map(|b| b.id).collect();

    assert_eq!(second_ids.len(), 3);
    assert!(second_ids.iter().all(|id| !ids.contains(id)));
    assert!(second.next_cursor.is_none());
}

#[tokio::test]
async fn get_books_desc_order_confirmed() {
    let app = TestApp::new().await;

    let mut ids = Vec::with_capacity(3);
    for _ in 0..3 {
        let book: Book = BookFaker::default().fake();
        app.insert_book(&book).await;
        ids.push(book.id);
    }
    ids.sort_by(|a, b| b.cmp(a));

    let req = Request::get("/books").body(Body::empty()).unwrap();
    let resp = app.router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let listed: RespWrapper<Vec<Book>> = serde_json::from_slice(&bytes).unwrap();
    let returned_ids: Vec<uuid::Uuid> = listed.data.iter().map(|b| b.id).collect();

    assert_eq!(returned_ids, ids);
}

#[tokio::test]
async fn get_books_zero_limit_returns_422() {
    let app = TestApp::new().await;

    let req = Request::get("/books?limit=0").body(Body::empty()).unwrap();
    let resp = app.router.oneshot(req).await.unwrap();

    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;
}

#[tokio::test]
async fn get_books_invalid_after_returns_400() {
    let app = TestApp::new().await;

    let req = Request::get("/books?after=not-a-uuid")
        .body(Body::empty())
        .unwrap();
    let resp = app.router.oneshot(req).await.unwrap();

    assert_error(resp, StatusCode::BAD_REQUEST).await;
}

#[tokio::test]
async fn update_book_with_valid_body_passes() {
    let app = TestApp::new().await;
    let book: Book = BookFaker {
        labels: 1..=5,
        links: 1..=5,
        titles: 1..=5,
        ..Default::default()
    }
    .fake();

    app.insert_book(&book).await;

    let updated: Book = BookFaker {
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
        "contentRating": updated.content_rating.id,
        "status": updated.status.as_ref(),
        "kind": updated.kind.as_ref(),
        "publicationLanguage": updated.publication_language.id,
        "labelIds": updated.labels.iter().map(|l| l.id).collect::<Vec<_>>(),
        "links": &updated.links,
        "titles": &updated.titles,
    })
    .to_string();

    let req = Request::put(format!("/books/{}", book.id))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();
    let resp = app.router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let got = app.fetch_book(book.id).await;

    assert_eq!(got.name, updated.name);
    assert_eq!(got.description, updated.description);
    assert_eq!(got.publication_year, updated.publication_year);
    assert_eq!(got.content_rating, updated.content_rating);
    assert_eq!(got.status, updated.status);
    assert_eq!(got.kind, updated.kind);
    assert_eq!(got.publication_language.id, updated.publication_language.id);

    assert_eq!(
        label_keys(&got.labels),
        label_keys(&updated.labels),
        "arrays must be replaced wholesale"
    );
    assert_eq!(
        link_keys(&got.links),
        link_keys(&updated.links),
        "arrays must be replaced wholesale"
    );
    assert_eq!(
        title_keys(&got.titles),
        title_keys(&updated.titles),
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
    let mut book: Book = BookFaker::default().fake();
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
        "contentRating": other.id,
        "status": book.status.as_ref(),
        "kind": book.kind.as_ref(),
        "publicationLanguage": book.publication_language.id,
    })
    .to_string();

    let req = Request::put(format!("/books/{}", book.id))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(updated_body))
        .unwrap();
    let resp = app.router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let got = app.fetch_book(book.id).await;

    assert_eq!(got.content_rating, other);
}

#[tokio::test]
async fn update_book_with_empty_arrays_detaches_everything() {
    let app = TestApp::new().await;
    let book: Book = BookFaker {
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
        "contentRating": book.content_rating.id,
        "status": book.status.as_ref(),
        "kind": book.kind.as_ref(),
        "publicationLanguage": book.publication_language.id,
        "labelIds": [],
        "links": [],
        "titles": [],
    })
    .to_string();

    let req = Request::put(format!("/books/{}", book.id))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(detached))
        .unwrap();
    let resp = app.router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let sample = app.fetch_book_sample(book.id).await;

    assert_eq!(sample.labels, 0);
    assert_eq!(sample.links, 0);
    assert_eq!(sample.titles, 0);
}

#[tokio::test]
async fn update_book_leaves_other_books_untouched() {
    let app = TestApp::new().await;
    let book: Book = BookFaker::default().fake();
    let other: Book = BookFaker::default().fake();

    app.insert_book(&book).await;
    app.insert_book(&other).await;

    let renamed: Book = BookFaker::default().fake();
    let updated_body = serde_json::json!({
        "name": renamed.name.as_ref(),
        "description": book.description.as_ref(),
        "publicationYear": book.publication_year,
        "contentRating": book.content_rating.id,
        "status": book.status.as_ref(),
        "kind": book.kind.as_ref(),
        "publicationLanguage": book.publication_language.id,
    })
    .to_string();

    let req = Request::put(format!("/books/{}", book.id))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(updated_body))
        .unwrap();
    let resp = app.router.clone().oneshot(req).await.unwrap();
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
    let book: Book = BookFaker::default().fake();

    let body = serde_json::json!({
        "name": book.name.as_ref(),
        "description": book.description.as_ref(),
        "publicationYear": book.publication_year,
        "contentRating": book.content_rating.id,
        "status": book.status.as_ref(),
        "kind": book.kind.as_ref(),
        "publicationLanguage": book.publication_language.id,
    })
    .to_string();

    let req = Request::put(format!("/books/{}", uuid::Uuid::now_v7()))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();
    assert_error(resp, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn delete_book_with_valid_id_passes() {
    let app = TestApp::new().await;
    let book: Book = BookFaker {
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

    let resp = app.router.clone().oneshot(req).await.unwrap();
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

    let mut book: Book = BookFaker {
        labels: 0..=0,
        ..Default::default()
    }
    .fake();
    book.labels.push(labels.clone());

    let mut other: Book = BookFaker {
        labels: 0..=0,
        ..Default::default()
    }
    .fake();
    other.labels.push(labels);

    app.insert_book(&book).await;
    app.insert_book(&other).await;

    let req = Request::delete(format!("/books/{}", book.id))
        .body(Body::empty())
        .unwrap();
    let resp = app.router.clone().oneshot(req).await.unwrap();
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

    let resp = app.router.oneshot(req).await.unwrap();

    assert_error(resp, StatusCode::NOT_FOUND).await;
}
