use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use http_body_util::BodyExt;
use serde::Deserialize;
use tower::ServiceExt;
use uuid::Uuid;

use crate::fakers::{BookFaker, COVER_JPG, COVER_PNG, COVER_WEBP};
use crate::helpers::{RespWrapper, TestApp, assert_error, assert_stored};
use fake::Fake;
use manga_theka::entity::Book;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct CoverEntry {
    id: Uuid,
    is_main: bool,
    url: String,
}

async fn insert_book(app: &TestApp) -> Uuid {
    let book: Book = BookFaker {
        creators: 0..=0,
        ..Default::default()
    }
    .fake();
    app.insert_book(&book).await;

    book.id
}

async fn upload_cover(app: &TestApp, book_id: Uuid, image: &'static [u8]) -> Uuid {
    let req = Request::post(format!("/books/{book_id}/covers"))
        .body(Body::from(image))
        .unwrap();

    let resp = app.router.clone().oneshot(req).await.unwrap();

    assert_stored(resp).await
}

async fn fetch_covers(app: &TestApp, book_id: Uuid) -> Vec<CoverEntry> {
    let req = Request::get(format!("/books/{book_id}/covers"))
        .body(Body::empty())
        .unwrap();

    let resp = app.router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let wrapper: RespWrapper<Vec<CoverEntry>> =
        serde_json::from_slice(&bytes).expect("gallery must carry a data array");

    assert!(wrapper.next_cursor.is_none(), "gallery must not paginate",);

    wrapper.data
}

async fn promote_cover(app: &TestApp, book_id: Uuid, cover_id: Uuid) -> StatusCode {
    let body = serde_json::json!({ "coverId": cover_id }).to_string();
    let req = Request::put(format!("/books/{book_id}/main-cover"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    app.router.clone().oneshot(req).await.unwrap().status()
}

#[tokio::test]
async fn store_book_cover_with_jpeg_passes() {
    let app = TestApp::new().await;
    let book_id = insert_book(&app).await;

    let id = upload_cover(&app, book_id, COVER_JPG).await;

    assert!(app.object_exists(id).await, "object must be uploaded");
    assert_eq!(app.object_content_type(id).await, "image/jpeg");
}

#[tokio::test]
async fn store_book_cover_with_png_passes() {
    let app = TestApp::new().await;
    let book_id = insert_book(&app).await;

    let id = upload_cover(&app, book_id, COVER_PNG).await;

    assert!(app.object_exists(id).await, "object must be uploaded");
    assert_eq!(app.object_content_type(id).await, "image/png");
}

#[tokio::test]
async fn store_book_cover_with_webp_passes() {
    let app = TestApp::new().await;
    let book_id = insert_book(&app).await;

    let id = upload_cover(&app, book_id, COVER_WEBP).await;

    assert!(app.object_exists(id).await, "object must be uploaded");
    assert_eq!(app.object_content_type(id).await, "image/webp");
}

#[tokio::test]
async fn store_book_cover_is_sniffed_not_trusted_from_content_type() {
    let app = TestApp::new().await;
    let book_id = insert_book(&app).await;

    let req = Request::post(format!("/books/{book_id}/covers"))
        .header(header::CONTENT_TYPE, "image/jpeg")
        .body(Body::from(COVER_PNG))
        .unwrap();

    let resp = app.router.clone().oneshot(req).await.unwrap();
    let id = assert_stored(resp).await;

    assert_eq!(app.object_content_type(id).await, "image/png");
}

#[tokio::test]
async fn store_book_cover_with_unknown_format_returns_415() {
    let app = TestApp::new().await;
    let book_id = insert_book(&app).await;

    let req = Request::post(format!("/books/{book_id}/covers"))
        .header(header::CONTENT_TYPE, "image/png")
        .body(Body::from("GIF89a not really an image"))
        .unwrap();

    let resp = app.router.clone().oneshot(req).await.unwrap();
    assert_error(resp, StatusCode::UNSUPPORTED_MEDIA_TYPE).await;

    assert_eq!(
        app.objects_count().await,
        0,
        "a rejected upload must write nothing"
    );
    assert!(fetch_covers(&app, book_id).await.is_empty());
}

#[tokio::test]
async fn store_book_cover_under_the_limit_passes() {
    let app = TestApp::new().await;
    let book_id = insert_book(&app).await;

    let mut body = COVER_PNG.to_vec();
    body.resize(3 * 1024 * 1024, 0);

    let req = Request::post(format!("/books/{book_id}/covers"))
        .body(Body::from(body))
        .unwrap();

    let resp = app.router.clone().oneshot(req).await.unwrap();
    let id = assert_stored(resp).await;

    assert!(app.object_exists(id).await);
}

#[tokio::test]
async fn store_book_cover_with_oversized_body_returns_413() {
    let app = TestApp::new().await;
    let book_id = insert_book(&app).await;

    let mut body = COVER_PNG.to_vec();
    body.resize(5 * 1024 * 1024 + 1, 0);

    let req = Request::post(format!("/books/{book_id}/covers"))
        .body(Body::from(body))
        .unwrap();

    let resp = app.router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);

    assert_eq!(
        app.objects_count().await,
        0,
        "a rejected upload must write nothing"
    );
}

#[tokio::test]
async fn store_book_cover_for_unknown_book_returns_404() {
    let app = TestApp::new().await;

    let req = Request::post(format!("/books/{}/covers", Uuid::now_v7()))
        .body(Body::from(COVER_PNG))
        .unwrap();

    let resp = app.router.clone().oneshot(req).await.unwrap();
    assert_error(resp, StatusCode::NOT_FOUND).await;

    assert_eq!(
        app.objects_count().await,
        0,
        "the book must be confirmed before any bytes are uploaded"
    );
}

#[tokio::test]
async fn store_book_cover_with_malformed_book_id_returns_400() {
    let app = TestApp::new().await;

    let req = Request::post("/books/not-a-uuid/covers")
        .body(Body::from(COVER_PNG))
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();
    assert_error(resp, StatusCode::BAD_REQUEST).await;
}

#[tokio::test]
async fn store_book_cover_leaves_the_gallery_unflagged() {
    let app = TestApp::new().await;
    let book_id = insert_book(&app).await;

    upload_cover(&app, book_id, COVER_PNG).await;
    upload_cover(&app, book_id, COVER_JPG).await;

    let covers = fetch_covers(&app, book_id).await;

    assert!(
        covers.iter().all(|c| !c.is_main),
        "a newly uploaded cover must be unflagged"
    );
}

#[tokio::test]
async fn get_book_covers_for_a_book_without_covers_is_empty() {
    let app = TestApp::new().await;
    let book_id = insert_book(&app).await;

    assert!(fetch_covers(&app, book_id).await.is_empty());
}

#[tokio::test]
async fn get_book_covers_for_unknown_book_returns_404() {
    let app = TestApp::new().await;

    let req = Request::get(format!("/books/{}/covers", Uuid::now_v7()))
        .body(Body::empty())
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();
    assert_error(resp, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn get_book_covers_lists_uploads_in_upload_order_with_relative_urls() {
    let app = TestApp::new().await;
    let book_id = insert_book(&app).await;

    let first = upload_cover(&app, book_id, COVER_PNG).await;
    let second = upload_cover(&app, book_id, COVER_JPG).await;
    let third = upload_cover(&app, book_id, COVER_WEBP).await;

    let covers = fetch_covers(&app, book_id).await;

    assert_eq!(
        covers.iter().map(|c| c.id).collect::<Vec<_>>(),
        vec![first, second, third]
    );
    assert_eq!(
        covers[0].url,
        format!("/books/{book_id}/covers/{first}/image")
    );
}

#[tokio::test]
async fn get_book_cover_image_redirects_to_a_presigned_url() {
    let app = TestApp::new().await;
    let book_id = insert_book(&app).await;
    let id = upload_cover(&app, book_id, COVER_JPG).await;

    let req = Request::get(format!("/books/{book_id}/covers/{id}/image"))
        .body(Body::empty())
        .unwrap();

    let resp = app.router.clone().oneshot(req).await.unwrap();

    assert_eq!(resp.status(), StatusCode::FOUND);

    let location = resp
        .headers()
        .get(header::LOCATION)
        .expect("redirect must carry a Location")
        .to_str()
        .unwrap()
        .to_owned();

    assert!(
        location.contains("X-Amz-Signature"),
        "target must be presigned: {location}"
    );
    assert!(
        location.contains(&format!("{id}.jpg")),
        "target must carry a filename built from the stored extension: {location}"
    );
}

#[tokio::test]
async fn get_book_cover_image_for_unknown_cover_returns_404() {
    let app = TestApp::new().await;
    let book_id = insert_book(&app).await;

    let req = Request::get(format!("/books/{book_id}/covers/{}/image", Uuid::now_v7()))
        .body(Body::empty())
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();
    assert_error(resp, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn get_book_cover_image_of_another_book_returns_404() {
    let app = TestApp::new().await;
    let book_id = insert_book(&app).await;
    let other_book_id = insert_book(&app).await;
    let id = upload_cover(&app, book_id, COVER_JPG).await;

    let req = Request::get(format!("/books/{other_book_id}/covers/{id}/image"))
        .body(Body::empty())
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();
    assert_error(resp, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn promote_book_cover_flags_it_and_demotes_the_previous_one() {
    let app = TestApp::new().await;
    let book_id = insert_book(&app).await;
    let first = upload_cover(&app, book_id, COVER_PNG).await;
    let second = upload_cover(&app, book_id, COVER_JPG).await;

    assert_eq!(
        promote_cover(&app, book_id, first).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        promote_cover(&app, book_id, second).await,
        StatusCode::NO_CONTENT
    );

    let covers = fetch_covers(&app, book_id).await;
    let flagged: Vec<Uuid> = covers.iter().filter(|c| c.is_main).map(|c| c.id).collect();

    assert_eq!(flagged, vec![second]);
}

#[tokio::test]
async fn promote_book_cover_is_idempotent() {
    let app = TestApp::new().await;
    let book_id = insert_book(&app).await;
    let id = upload_cover(&app, book_id, COVER_PNG).await;

    assert_eq!(
        promote_cover(&app, book_id, id).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        promote_cover(&app, book_id, id).await,
        StatusCode::NO_CONTENT
    );

    let covers = fetch_covers(&app, book_id).await;

    assert_eq!(covers.iter().filter(|c| c.is_main).count(), 1);
}

#[tokio::test]
async fn promote_unknown_book_cover_returns_404() {
    let app = TestApp::new().await;
    let book_id = insert_book(&app).await;

    assert_eq!(
        promote_cover(&app, book_id, Uuid::now_v7()).await,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn promote_book_cover_of_another_book_returns_404() {
    let app = TestApp::new().await;
    let book_id = insert_book(&app).await;
    let other_book_id = insert_book(&app).await;
    let id = upload_cover(&app, book_id, COVER_PNG).await;

    assert_eq!(
        promote_cover(&app, other_book_id, id).await,
        StatusCode::NOT_FOUND
    );

    let covers = fetch_covers(&app, book_id).await;

    assert!(
        covers.iter().all(|c| !c.is_main),
        "a foreign promotion must not flag anything"
    );
}

#[tokio::test]
async fn delete_book_cover_removes_it_from_the_gallery_and_purges_the_object() {
    let app = TestApp::new().await;
    let book_id = insert_book(&app).await;
    let first = upload_cover(&app, book_id, COVER_PNG).await;
    let second = upload_cover(&app, book_id, COVER_JPG).await;

    let req = Request::delete(format!("/books/{book_id}/covers/{first}"))
        .body(Body::empty())
        .unwrap();

    let resp = app.router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let covers = fetch_covers(&app, book_id).await;

    assert_eq!(
        covers.iter().map(|c| c.id).collect::<Vec<_>>(),
        vec![second]
    );
    assert!(!app.object_exists(first).await, "object must be purged");
    assert!(app.object_exists(second).await);
}

#[tokio::test]
async fn delete_book_cover_twice_returns_404() {
    let app = TestApp::new().await;
    let book_id = insert_book(&app).await;
    let id = upload_cover(&app, book_id, COVER_PNG).await;

    for expected in [StatusCode::NO_CONTENT, StatusCode::NOT_FOUND] {
        let req = Request::delete(format!("/books/{book_id}/covers/{id}"))
            .body(Body::empty())
            .unwrap();

        let resp = app.router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), expected);
    }
}

#[tokio::test]
async fn delete_book_cover_of_another_book_returns_404() {
    let app = TestApp::new().await;
    let book_id = insert_book(&app).await;
    let other_book_id = insert_book(&app).await;
    let id = upload_cover(&app, book_id, COVER_PNG).await;

    let req = Request::delete(format!("/books/{other_book_id}/covers/{id}"))
        .body(Body::empty())
        .unwrap();

    let resp = app.router.clone().oneshot(req).await.unwrap();
    assert_error(resp, StatusCode::NOT_FOUND).await;

    assert!(app.object_exists(id).await, "object must survive");
}

#[tokio::test]
async fn delete_the_flagged_book_cover_leaves_the_oldest_remaining_one() {
    let app = TestApp::new().await;
    let book_id = insert_book(&app).await;
    let first = upload_cover(&app, book_id, COVER_PNG).await;
    let second = upload_cover(&app, book_id, COVER_JPG).await;

    promote_cover(&app, book_id, second).await;

    let req = Request::delete(format!("/books/{book_id}/covers/{second}"))
        .body(Body::empty())
        .unwrap();

    let resp = app.router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let covers = fetch_covers(&app, book_id).await;

    assert_eq!(covers.iter().map(|c| c.id).collect::<Vec<_>>(), vec![first]);
    assert!(
        covers.iter().all(|c| !c.is_main),
        "the oldest remaining cover is the Cover without carrying the flag"
    );
}

#[tokio::test]
async fn delete_book_removes_its_covers_and_purges_their_objects() {
    let app = TestApp::new().await;
    let book_id = insert_book(&app).await;
    let first = upload_cover(&app, book_id, COVER_PNG).await;
    let second = upload_cover(&app, book_id, COVER_JPG).await;

    let req = Request::delete(format!("/books/{book_id}"))
        .body(Body::empty())
        .unwrap();

    let resp = app.router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let req = Request::get(format!("/books/{book_id}/covers"))
        .body(Body::empty())
        .unwrap();

    let resp = app.router.clone().oneshot(req).await.unwrap();
    assert_error(resp, StatusCode::NOT_FOUND).await;

    assert!(!app.object_exists(first).await, "objects must be purged");
    assert!(!app.object_exists(second).await, "objects must be purged");
}
