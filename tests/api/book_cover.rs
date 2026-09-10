use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use axum_test::multipart::{MultipartForm, Part};
use http_body_util::BodyExt;
use manga_theka::entity::{BookCoverQuery, ImageExtension};
use uuid::Uuid;

use crate::helpers::fakers::{COVER_JPG, COVER_PNG, COVER_WEBP};
use crate::helpers::{RespWrapper, TestApp, assert_error, assert_stored};

#[tokio::test]
async fn store_book_cover_is_sniffed_not_trusted_from_content_type() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;

    let form = MultipartForm::new().add_part(
        "cover",
        Part::bytes(COVER_PNG)
            .file_name("cover.jpg")
            .mime_type("image/jpeg"),
    );

    let req = Request::post(format!("/books/{book_id}/covers"))
        .header(header::CONTENT_TYPE, form.content_type())
        .body(Body::from(form))
        .unwrap();

    let resp = app.send(req).await;
    let cover_id = assert_stored(resp).await;
    let content_type = app.object_content_type(&app.covers_bucket, cover_id).await;

    assert_eq!(content_type, "image/png");
}

#[tokio::test]
async fn store_book_cover_with_unknown_format_returns_415() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;

    let resp = app.post_cover(book_id, b"GIF89a not really an image").await;
    assert_error(resp, StatusCode::UNSUPPORTED_MEDIA_TYPE).await;

    let obj_count = app.objects_count(&app.covers_bucket).await;
    let covers = app.fetch_covers(book_id).await;

    assert_eq!(obj_count, 0);
    assert!(covers.is_empty());
}

#[tokio::test]
async fn store_book_cover_under_the_limit_passes() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;

    let mut body = COVER_PNG.to_vec();
    body.resize(3 * 1024 * 1024, 0);

    let resp = app.post_cover(book_id, &body).await;
    let cover_id = assert_stored(resp).await;

    let obj_exists = app.object_exists(&app.covers_bucket, cover_id).await;
    assert!(obj_exists);
}

#[tokio::test]
async fn store_book_cover_with_oversized_body_returns_413() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;

    let mut body = COVER_PNG.to_vec();
    body.resize(5 * 1024 * 1024 + 1, 0);

    let resp = app.post_cover(book_id, &body).await;
    assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);

    let obj_count = app.objects_count(&app.covers_bucket).await;
    assert_eq!(obj_count, 0);
}

#[tokio::test]
async fn store_book_cover_for_unknown_book_returns_404() {
    let app = TestApp::new().await;

    let resp = app.post_cover(Uuid::now_v7(), COVER_PNG).await;
    assert_error(resp, StatusCode::NOT_FOUND).await;

    let obj_count = app.objects_count(&app.covers_bucket).await;
    assert_eq!(obj_count, 0,);
}

#[tokio::test]
async fn store_book_cover_with_malformed_book_id_returns_400() {
    let app = TestApp::new().await;

    let req = Request::post("/books/not-a-uuid/covers")
        .body(Body::from(COVER_PNG))
        .unwrap();

    let resp = app.send(req).await;
    assert_error(resp, StatusCode::BAD_REQUEST).await;
}

#[tokio::test]
async fn store_book_cover_leaves_the_gallery_unflagged() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;

    for image in [COVER_PNG, COVER_JPG] {
        let resp = app.post_cover(book_id, image).await;
        assert_stored(resp).await;
    }

    let covers = app.fetch_covers(book_id).await;
    let flagged_count = covers.iter().filter(|c| c.is_main).count();

    assert_eq!(flagged_count, 0);
}

#[tokio::test]
async fn get_book_covers_without_uploads_returns_an_empty_gallery() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;

    let resp = app.get_covers(book_id).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let wrapper: RespWrapper<Vec<BookCoverQuery>> = serde_json::from_slice(&bytes).unwrap();
    assert!(wrapper.data.is_empty());
}

#[tokio::test]
async fn get_book_covers_for_unknown_book_returns_404() {
    let app = TestApp::new().await;

    let resp = app.get_covers(Uuid::now_v7()).await;
    assert_error(resp, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn get_book_covers_lists_uploads_in_upload_order() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;

    let first_id = app.insert_cover(book_id, COVER_PNG).await;
    let second_id = app.insert_cover(book_id, COVER_JPG).await;
    let third_id = app.insert_cover(book_id, COVER_WEBP).await;

    let resp = app.get_covers(book_id).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let wrapper: RespWrapper<Vec<BookCoverQuery>> = serde_json::from_slice(&bytes).unwrap();

    let cover_ids: Vec<Uuid> = wrapper.data.iter().map(|c| c.id).collect();
    assert_eq!(cover_ids, vec![first_id, second_id, third_id]);
}

#[tokio::test]
async fn get_book_covers_returns_all_fields() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let cover_id = app.insert_cover(book_id, COVER_PNG).await;

    let resp = app.get_covers(book_id).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let wrapper: RespWrapper<Vec<BookCoverQuery>> = serde_json::from_slice(&bytes).unwrap();
    let cover = &wrapper.data[0];
    let expected_url = format!("/covers/{cover_id}/image");

    assert_eq!(wrapper.data.len(), 1);
    assert_eq!(cover.id, cover_id);
    assert_eq!(cover.book_id, book_id);
    assert_eq!(cover.extension, ImageExtension::Png);
    assert!(!cover.is_main);
    assert_eq!(cover.url.as_ref(), expected_url);
}

#[tokio::test]
async fn get_book_cover_image_redirects_to_a_presigned_url() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let cover_id = app.insert_cover(book_id, COVER_JPG).await;

    let resp = app.get_cover_image(cover_id).await;
    assert_eq!(resp.status(), StatusCode::FOUND);

    let location = resp
        .headers()
        .get(header::LOCATION)
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();

    assert!(location.contains("X-Amz-Signature"));
}

#[tokio::test]
async fn get_book_cover_image_for_unknown_cover_returns_404() {
    let app = TestApp::new().await;

    let resp = app.get_cover_image(Uuid::now_v7()).await;
    assert_error(resp, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn promote_book_cover_flags_it_and_demotes_the_previous_one() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let first_id = app.insert_cover(book_id, COVER_PNG).await;
    let second_id = app.insert_cover(book_id, COVER_JPG).await;

    let first_status = app.put_main_cover(book_id, first_id).await;
    let second_status = app.put_main_cover(book_id, second_id).await;
    let covers = app.fetch_covers(book_id).await;
    let flagged_ids: Vec<Uuid> = covers.iter().filter(|c| c.is_main).map(|c| c.id).collect();

    assert_eq!(first_status, StatusCode::NO_CONTENT);
    assert_eq!(second_status, StatusCode::NO_CONTENT);
    assert_eq!(flagged_ids, vec![second_id]);
}

#[tokio::test]
async fn promote_book_cover_is_idempotent() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let cover_id = app.insert_cover(book_id, COVER_PNG).await;

    let first_status = app.put_main_cover(book_id, cover_id).await;
    let second_status = app.put_main_cover(book_id, cover_id).await;
    let covers = app.fetch_covers(book_id).await;
    let flagged_count = covers.iter().filter(|c| c.is_main).count();

    assert_eq!(first_status, StatusCode::NO_CONTENT);
    assert_eq!(second_status, StatusCode::NO_CONTENT);
    assert_eq!(flagged_count, 1);
}

#[tokio::test]
async fn promote_unknown_book_cover_returns_404() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;

    let status = app.put_main_cover(book_id, Uuid::now_v7()).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn promote_book_cover_of_another_book_returns_404() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let other_book_id = app.insert_random_book().await;
    let cover_id = app.insert_cover(book_id, COVER_PNG).await;

    let status = app.put_main_cover(other_book_id, cover_id).await;
    let covers = app.fetch_covers(book_id).await;
    let flagged_count = covers.iter().filter(|c| c.is_main).count();

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(flagged_count, 0,);
}

#[tokio::test]
async fn delete_book_cover_removes_it_from_the_gallery_and_purges_the_object() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let first_id = app.insert_cover(book_id, COVER_PNG).await;
    let second_id = app.insert_cover(book_id, COVER_JPG).await;

    let resp = app.delete_cover(first_id).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let covers = app.fetch_covers(book_id).await;
    let cover_ids: Vec<Uuid> = covers.iter().map(|c| c.id).collect();
    let first_obj_exists = app.object_exists(&app.covers_bucket, first_id).await;
    let second_obj_exists = app.object_exists(&app.covers_bucket, second_id).await;

    assert_eq!(cover_ids, vec![second_id]);
    assert!(!first_obj_exists);
    assert!(second_obj_exists);
}

#[tokio::test]
async fn delete_book_cover_twice_returns_404() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let cover_id = app.insert_cover(book_id, COVER_PNG).await;

    for expected_status in [StatusCode::NO_CONTENT, StatusCode::NOT_FOUND] {
        let resp = app.delete_cover(cover_id).await;
        assert_eq!(resp.status(), expected_status);
    }
}

#[tokio::test]
async fn delete_the_flagged_book_cover_leaves_the_oldest_remaining_one() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let first_id = app.insert_cover(book_id, COVER_PNG).await;
    let second_id = app.insert_cover(book_id, COVER_JPG).await;

    app.put_main_cover(book_id, second_id).await;

    let resp = app.delete_cover(second_id).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let covers = app.fetch_covers(book_id).await;
    let cover_ids: Vec<Uuid> = covers.iter().map(|c| c.id).collect();
    let flagged_count = covers.iter().filter(|c| c.is_main).count();

    assert_eq!(cover_ids, vec![first_id]);
    assert_eq!(flagged_count, 0);
}

#[tokio::test]
async fn delete_book_removes_its_covers_and_purges_their_objects() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let first_id = app.insert_cover(book_id, COVER_PNG).await;
    let second_id = app.insert_cover(book_id, COVER_JPG).await;

    let req = Request::delete(format!("/books/{book_id}"))
        .body(Body::empty())
        .unwrap();

    let resp = app.send(req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let resp = app.get_covers(book_id).await;
    assert_error(resp, StatusCode::NOT_FOUND).await;

    let first_obj_exists = app.object_exists(&app.covers_bucket, first_id).await;
    let second_obj_exists = app.object_exists(&app.covers_bucket, second_id).await;

    assert!(!first_obj_exists);
    assert!(!second_obj_exists);
}
