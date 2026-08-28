use axum::http::StatusCode;
use fake::Fake;
use http_body_util::BodyExt;
use manga_theka::entity::{
    Book, ChapterPageQuery, ChapterReleaseQuery, ImageExtension, PageNumber, PageUrl,
};
use uuid::Uuid;

use crate::fakers::{BookFaker, COVER_JPG, COVER_PNG, COVER_WEBP};
use crate::helpers::{RespWrapper, TestApp, assert_error, assert_stored, redirect_target};

#[tokio::test]
async fn store_chapter_release_mints_an_identifier() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let language_id = app.non_publication_language(book_id).await;

    let resp = app.post_release(chapter_id, language_id).await;
    let release_id = assert_stored(resp).await;

    let resp = app.get_release(release_id).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn store_chapter_release_in_the_publication_language_returns_422() {
    let app = TestApp::new().await;
    let book: Book = BookFaker::default().fake();
    app.insert_book(&book).await;
    let chapter_id = app.insert_random_chapter(book.id).await;

    let resp = app
        .post_release(chapter_id, book.publication_language.id)
        .await;
    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;
}

#[tokio::test]
async fn store_chapter_release_for_unknown_chapter_returns_404() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let language_id = app.non_publication_language(book_id).await;

    let resp = app.post_release(Uuid::now_v7(), language_id).await;
    assert_error(resp, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn store_chapter_release_with_unknown_language_returns_422() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;

    let resp = app.post_release(chapter_id, Uuid::now_v7()).await;
    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;
}

#[tokio::test]
async fn store_chapter_release_allows_competing_releases_in_one_language() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let language_id = app.non_publication_language(book_id).await;

    let first = app.post_release(chapter_id, language_id).await;
    let first_id = assert_stored(first).await;
    let second = app.post_release(chapter_id, language_id).await;
    let second_id = assert_stored(second).await;

    assert_ne!(first_id, second_id);
}

#[tokio::test]
async fn upload_chapter_pages_stages_every_part_and_returns_their_identifiers_in_order() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;

    let resp = app
        .post_upload_pages(release_id, &[COVER_PNG, COVER_JPG, COVER_WEBP])
        .await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let wrapper: RespWrapper<Vec<Uuid>> = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(wrapper.data.len(), 3);
    for id in &wrapper.data {
        assert!(app.object_exists(&app.release_pages_bucket, *id).await);
    }
}

#[tokio::test]
async fn upload_chapter_pages_sniffs_the_format_rather_than_trusting_the_part() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;

    let resp = app.post_upload_pages(release_id, &[COVER_PNG]).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let wrapper: RespWrapper<Vec<Uuid>> = serde_json::from_slice(&bytes).unwrap();
    let id = wrapper.data[0];

    let content_type = app.object_content_type(&app.release_pages_bucket, id).await;
    assert_eq!(content_type, "image/png");
}

#[tokio::test]
async fn upload_chapter_pages_with_an_unknown_format_returns_415() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;

    let resp = app
        .post_upload_pages(release_id, &[b"GIF89a not really an image"])
        .await;
    assert_error(resp, StatusCode::UNSUPPORTED_MEDIA_TYPE).await;

    assert_eq!(app.objects_count(&app.release_pages_bucket).await, 0);
    assert!(app.fetch_page_order(release_id).await.is_empty());
}

#[tokio::test]
async fn upload_chapter_pages_with_an_oversized_part_returns_413() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;

    let mut oversized = COVER_PNG.to_vec();
    oversized.resize(5 * 1024 * 1024 + 1, 0);

    let resp = app.post_upload_pages(release_id, &[&oversized]).await;
    assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);

    assert_eq!(app.objects_count(&app.release_pages_bucket).await, 0);
    assert!(app.fetch_page_order(release_id).await.is_empty());
}

#[tokio::test]
async fn upload_chapter_pages_stores_nothing_when_a_later_part_fails() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;

    let resp = app
        .post_upload_pages(release_id, &[COVER_PNG, COVER_JPG, b"GIF89a"])
        .await;
    assert_error(resp, StatusCode::UNSUPPORTED_MEDIA_TYPE).await;

    assert!(app.fetch_page_order(release_id).await.is_empty());
}

#[tokio::test]
async fn upload_chapter_pages_over_the_part_limit_returns_422() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;

    let parts: Vec<&[u8]> = (0..11).map(|_| COVER_PNG).collect();

    let resp = app.post_upload_pages(release_id, &parts).await;
    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;
}

#[tokio::test]
async fn upload_chapter_pages_for_unknown_release_returns_404() {
    let app = TestApp::new().await;

    let resp = app.post_upload_pages(Uuid::now_v7(), &[COVER_PNG]).await;
    assert_error(resp, StatusCode::NOT_FOUND).await;

    assert_eq!(app.objects_count(&app.release_pages_bucket).await, 0);
}

#[tokio::test]
async fn upload_chapter_pages_over_the_release_row_budget_returns_422() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;

    sqlx::query!(
        r#"
insert into chapter_pages(id, release_id, sort_order, extension)
select gen_random_uuid(), $1, null, 'png'
from generate_series(1, 400);
        "#,
        release_id
    )
    .execute(&app.pool)
    .await
    .unwrap();

    let resp = app.post_upload_pages(release_id, &[COVER_PNG]).await;
    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;
}

#[tokio::test]
async fn commit_chapter_release_publishes_it_in_the_declared_order() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;

    let staged = app
        .insert_staged_pages(release_id, &[COVER_PNG, COVER_JPG, COVER_WEBP])
        .await;
    let declared = vec![staged[2], staged[0], staged[1]];

    let resp = app.post_commit(release_id, &declared).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    assert_eq!(app.fetch_committed_page_ids(release_id).await, declared);
}

#[tokio::test]
async fn recommitting_a_chapter_release_bumps_the_version() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;

    let staged = app
        .insert_staged_pages(release_id, &[COVER_PNG, COVER_JPG])
        .await;

    let resp = app.post_commit(release_id, &staged).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    assert_eq!(app.fetch_release_version(release_id).await, 2);

    let resp = app.post_commit(release_id, &staged[..1]).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    assert_eq!(app.fetch_release_version(release_id).await, 3);
}

#[tokio::test]
async fn commit_chapter_release_naming_a_foreign_page_returns_422() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;
    let other_release_id = app.insert_random_release(book_id, chapter_id).await;

    let staged = app.insert_staged_pages(release_id, &[COVER_PNG]).await;
    let foreign = app
        .insert_staged_pages(other_release_id, &[COVER_JPG])
        .await;

    let declared = vec![staged[0], foreign[0]];

    let resp = app.post_commit(release_id, &declared).await;
    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;

    assert_eq!(app.fetch_release_version(release_id).await, 1);
    assert!(
        app.object_exists(&app.release_pages_bucket, foreign[0])
            .await
    );
}

#[tokio::test]
async fn commit_chapter_release_naming_a_page_twice_returns_422() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;
    let staged = app.insert_staged_pages(release_id, &[COVER_PNG]).await;

    let declared = vec![staged[0], staged[0]];

    let resp = app.post_commit(release_id, &declared).await;
    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;
}

#[tokio::test]
async fn commit_chapter_release_with_an_empty_order_returns_422() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;
    app.insert_staged_pages(release_id, &[COVER_PNG]).await;

    let resp = app.post_commit(release_id, &[]).await;
    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;
}

#[tokio::test]
async fn commit_chapter_release_for_unknown_release_returns_404() {
    let app = TestApp::new().await;

    let resp = app.post_commit(Uuid::now_v7(), &[Uuid::now_v7()]).await;
    assert_error(resp, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn commit_chapter_release_moves_a_page_onto_a_position_another_page_still_holds() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;

    let first = app.insert_page(release_id, Some(1), COVER_PNG).await;
    let second = app.insert_page(release_id, Some(2), COVER_JPG).await;
    let third = app.insert_page(release_id, Some(3), COVER_WEBP).await;

    let declared = vec![third, first, second];

    let resp = app.post_commit(release_id, &declared).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    assert_eq!(app.fetch_committed_page_ids(release_id).await, declared);
}

#[tokio::test]
async fn commit_chapter_release_removes_undeclared_pages_and_their_images() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;

    let kept = app.insert_page(release_id, Some(1), COVER_PNG).await;
    let dropped = app.insert_page(release_id, Some(2), COVER_JPG).await;
    let staged = app.insert_page(release_id, None, COVER_WEBP).await;

    let resp = app.post_commit(release_id, &[kept]).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    assert_eq!(app.fetch_committed_page_ids(release_id).await, vec![kept]);
    assert!(app.object_exists(&app.release_pages_bucket, kept).await);
    assert!(!app.object_exists(&app.release_pages_bucket, dropped).await);
    assert!(!app.object_exists(&app.release_pages_bucket, staged).await);
}

#[tokio::test]
async fn commit_chapter_release_closes_the_gap_left_by_a_dropped_page() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;

    let first = app.insert_page(release_id, Some(1), COVER_PNG).await;
    let second = app.insert_page(release_id, Some(2), COVER_JPG).await;
    let third = app.insert_page(release_id, Some(3), COVER_WEBP).await;

    let resp = app.post_commit(release_id, &[first, third]).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    assert_eq!(
        app.fetch_page_order(release_id).await,
        vec![(first, Some(1)), (third, Some(2))]
    );
    assert!(!app.object_exists(&app.release_pages_bucket, second).await);
}

#[tokio::test]
async fn commit_chapter_release_inserts_a_staged_page_between_committed_ones() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;

    let first = app.insert_page(release_id, Some(1), COVER_PNG).await;
    let second = app.insert_page(release_id, Some(2), COVER_JPG).await;
    let inserted = app.insert_staged_pages(release_id, &[COVER_WEBP]).await[0];

    let declared = vec![first, inserted, second];

    let resp = app.post_commit(release_id, &declared).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    assert_eq!(app.fetch_committed_page_ids(release_id).await, declared);
}

#[tokio::test]
async fn commit_chapter_release_leaves_untouched_pages_with_their_identifiers_and_images() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;

    let kept = app.insert_page(release_id, Some(1), COVER_PNG).await;
    let superseded = app.insert_page(release_id, Some(2), COVER_JPG).await;
    let replacement = app.insert_staged_pages(release_id, &[COVER_WEBP]).await[0];

    let declared = vec![kept, replacement];

    let resp = app.post_commit(release_id, &declared).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    assert_eq!(app.fetch_committed_page_ids(release_id).await, declared);
    assert!(app.object_exists(&app.release_pages_bucket, kept).await);
    assert!(
        !app.object_exists(&app.release_pages_bucket, superseded)
            .await
    );
}

#[tokio::test]
async fn get_chapter_releases_lists_a_release_that_was_never_committed() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let draft_id = app.insert_random_release(book_id, chapter_id).await;
    app.insert_page(draft_id, None, COVER_PNG).await;

    let resp = app.get_releases(chapter_id).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let wrapper: RespWrapper<Vec<ChapterReleaseQuery>> = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(wrapper.data.len(), 1);
    assert_eq!(wrapper.data[0].id, draft_id);
    assert_eq!(wrapper.data[0].page_count, 0);
}

#[tokio::test]
async fn get_chapter_releases_lists_a_committed_release_with_all_its_fields() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;
    let language_id = app.non_publication_language(book_id).await;

    app.insert_page(release_id, Some(1), COVER_PNG).await;
    app.insert_page(release_id, Some(2), COVER_JPG).await;

    let resp = app.get_releases(chapter_id).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let wrapper: RespWrapper<Vec<ChapterReleaseQuery>> = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(wrapper.data.len(), 1);

    let release = &wrapper.data[0];
    assert_eq!(release.id, release_id);
    assert_eq!(release.chapter_id, chapter_id);
    assert_eq!(release.language.id, language_id);
    assert!(!release.language.code.is_empty());
    assert!(!release.language.name.is_empty());
    assert_eq!(release.page_count, 2);
    assert_eq!(release.version.as_i32(), 1);
}

#[tokio::test]
async fn get_chapter_releases_for_unknown_chapter_returns_404() {
    let app = TestApp::new().await;

    let resp = app.get_releases(Uuid::now_v7()).await;
    assert_error(resp, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn get_chapter_release_for_unknown_release_returns_404() {
    let app = TestApp::new().await;

    let resp = app.get_release(Uuid::now_v7()).await;
    assert_error(resp, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn get_chapter_pages_lists_a_committed_page_with_all_its_fields() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;

    let committed_id = app.insert_page(release_id, Some(1), COVER_PNG).await;
    app.insert_page(release_id, None, COVER_WEBP).await;

    let resp = app.get_pages(release_id).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let wrapper: RespWrapper<Vec<ChapterPageQuery>> = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(wrapper.data.len(), 1);
    let ChapterPageQuery::Committed {
        id,
        page_number,
        extension,
        url,
    } = wrapper.data.into_iter().next().unwrap()
    else {
        panic!("the committed listing must carry the Committed variant");
    };
    assert_eq!(id, committed_id);
    let first_page = PageNumber::try_from(1).unwrap();
    assert_eq!(page_number, first_page);
    assert_eq!(extension, ImageExtension::Png);
    assert_eq!(url, PageUrl::from((release_id, first_page)));
}

#[tokio::test]
async fn get_chapter_pages_for_unknown_release_returns_404() {
    let app = TestApp::new().await;

    let resp = app.get_pages(Uuid::now_v7()).await;
    assert_error(resp, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn get_staged_chapter_pages_lists_a_staged_page_with_all_its_fields() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;

    app.insert_page(release_id, Some(1), COVER_PNG).await;
    let staged_id = app.insert_page(release_id, None, COVER_WEBP).await;

    let resp = app.get_staged(release_id).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let wrapper: RespWrapper<Vec<ChapterPageQuery>> = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(wrapper.data.len(), 1);
    let ChapterPageQuery::Staged { id, extension } = wrapper.data.into_iter().next().unwrap()
    else {
        panic!("the staged listing must carry the Staged variant");
    };
    assert_eq!(id, staged_id);
    assert_eq!(extension, ImageExtension::Webp);
}

#[tokio::test]
async fn get_staged_chapter_pages_for_unknown_release_returns_404() {
    let app = TestApp::new().await;

    let resp = app.get_staged(Uuid::now_v7()).await;
    assert_error(resp, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn get_chapter_page_image_redirects_for_staged_and_committed_pages_alike() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;

    let committed = app.insert_page(release_id, Some(1), COVER_PNG).await;
    let staged = app.insert_page(release_id, None, COVER_WEBP).await;

    for page_id in [committed, staged] {
        let resp = app.get_page_image(release_id, page_id).await;
        assert_eq!(resp.status(), StatusCode::FOUND);

        assert!(redirect_target(&resp).contains("X-Amz-Signature"));
    }
}

#[tokio::test]
async fn get_chapter_page_image_of_another_release_returns_404() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;
    let other_release_id = app.insert_random_release(book_id, chapter_id).await;

    let page_id = app.insert_page(release_id, Some(1), COVER_PNG).await;

    let resp = app.get_page_image(other_release_id, page_id).await;
    assert_error(resp, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn get_chapter_page_redirects_to_storage_rather_than_serving_the_bytes() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;

    let page_id = app.insert_page(release_id, Some(1), COVER_PNG).await;

    let resp = app.get_page(release_id, 1).await;
    assert_eq!(resp.status(), StatusCode::FOUND);

    let location = redirect_target(&resp);
    assert!(location.contains("X-Amz-Signature"));
    assert!(location.contains(&page_id.to_string()));
}

#[tokio::test]
async fn get_chapter_page_at_an_untouched_position_survives_a_replacement_elsewhere() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;

    let kept = app.insert_page(release_id, Some(1), COVER_PNG).await;
    let superseded = app.insert_page(release_id, Some(2), COVER_JPG).await;
    let replacement = app.insert_page(release_id, None, COVER_WEBP).await;

    let resp = app.post_commit(release_id, &[kept, replacement]).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let resp = app.get_page(release_id, 1).await;
    assert_eq!(resp.status(), StatusCode::FOUND);

    let location = redirect_target(&resp);
    assert!(location.contains(&kept.to_string()));
    assert!(!location.contains(&superseded.to_string()));
}

#[tokio::test]
async fn get_chapter_page_resolves_to_whatever_a_reorder_moved_onto_the_position() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;

    let first = app.insert_page(release_id, Some(1), COVER_PNG).await;
    let second = app.insert_page(release_id, Some(2), COVER_JPG).await;

    let resp = app.get_page(release_id, 1).await;
    assert!(redirect_target(&resp).contains(&first.to_string()));

    let resp = app.post_commit(release_id, &[second, first]).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let resp = app.get_page(release_id, 1).await;
    assert_eq!(resp.status(), StatusCode::FOUND);

    assert!(redirect_target(&resp).contains(&second.to_string()));
}

#[tokio::test]
async fn get_chapter_page_of_a_never_committed_release_resolves_for_preview() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;

    let page_id = app.insert_page(release_id, Some(1), COVER_PNG).await;
    assert_eq!(app.fetch_release_version(release_id).await, 1);

    let resp = app.get_page(release_id, 1).await;
    assert_eq!(resp.status(), StatusCode::FOUND);

    assert!(redirect_target(&resp).contains(&page_id.to_string()));
}

#[tokio::test]
async fn get_chapter_page_past_the_end_of_the_release_returns_404() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;

    app.insert_page(release_id, Some(1), COVER_PNG).await;

    let resp = app.get_page(release_id, 2).await;
    assert_error(resp, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn get_chapter_page_of_an_unknown_release_returns_404() {
    let app = TestApp::new().await;

    let resp = app.get_page(Uuid::now_v7(), 1).await;
    assert_error(resp, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn get_chapter_page_at_a_position_that_is_not_a_number_returns_400() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;

    app.insert_page(release_id, Some(1), COVER_PNG).await;

    let req = axum::http::Request::get(format!("/releases/{release_id}/pages/first"))
        .body(axum::body::Body::empty())
        .unwrap();
    let resp = tower::ServiceExt::oneshot(app.router.clone(), req)
        .await
        .unwrap();

    assert_error(resp, StatusCode::BAD_REQUEST).await;
}

#[tokio::test]
async fn get_chapter_page_at_a_non_positive_position_returns_422() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;

    app.insert_page(release_id, Some(1), COVER_PNG).await;

    for page_number in [0, -1] {
        let resp = app.get_page(release_id, page_number).await;
        assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;
    }
}

#[tokio::test]
async fn delete_chapter_release_purges_its_pages_and_images() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;

    let committed_id = app.insert_page(release_id, Some(1), COVER_PNG).await;
    let staged_id = app.insert_page(release_id, None, COVER_WEBP).await;

    let resp = app.delete_release(release_id).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    assert!(app.fetch_release_id(release_id).await.is_none());
    assert!(
        !app.object_exists(&app.release_pages_bucket, committed_id)
            .await
    );
    assert!(
        !app.object_exists(&app.release_pages_bucket, staged_id)
            .await
    );
}

#[tokio::test]
async fn delete_chapter_release_twice_returns_404() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;

    for expected_status in [StatusCode::NO_CONTENT, StatusCode::NOT_FOUND] {
        let resp = app.delete_release(release_id).await;
        assert_eq!(resp.status(), expected_status);
    }
}

#[tokio::test]
async fn delete_chapter_holding_a_release_returns_409() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;
    let page_id = app.insert_page(release_id, Some(1), COVER_PNG).await;

    let resp = app.delete_chapter(chapter_id).await;
    assert_error(resp, StatusCode::CONFLICT).await;

    let resp = app.get_release(release_id).await;
    assert_eq!(resp.status(), StatusCode::OK);

    assert!(app.object_exists(&app.release_pages_bucket, page_id).await);
}

#[tokio::test]
async fn delete_chapter_holding_a_never_committed_release_returns_409() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let draft_id = app.insert_random_release(book_id, chapter_id).await;

    let resp = app.delete_chapter(chapter_id).await;
    assert_error(resp, StatusCode::CONFLICT).await;

    let resp = app.get_release(draft_id).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn delete_chapter_passes_once_its_releases_are_deleted() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;
    app.insert_page(release_id, Some(1), COVER_PNG).await;

    let resp = app.delete_release(release_id).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let resp = app.delete_chapter(chapter_id).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let sample = app.fetch_chapter_sample(chapter_id).await;
    assert!(sample.number.is_none());
}

#[tokio::test]
async fn delete_book_holding_a_release_returns_409() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;
    let page_id = app.insert_page(release_id, Some(1), COVER_PNG).await;

    let resp = app.delete_book(book_id).await;
    assert_error(resp, StatusCode::CONFLICT).await;

    let resp = app.get_release(release_id).await;
    assert_eq!(resp.status(), StatusCode::OK);

    assert!(app.object_exists(&app.release_pages_bucket, page_id).await);
}

#[tokio::test]
async fn delete_book_holding_a_never_committed_release_returns_409() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let draft_id = app.insert_random_release(book_id, chapter_id).await;

    let resp = app.delete_book(book_id).await;
    assert_error(resp, StatusCode::CONFLICT).await;

    let resp = app.get_release(draft_id).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn commit_chapter_release_over_the_committed_page_ceiling_returns_422() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;

    let staged: Vec<Uuid> = sqlx::query_scalar!(
        r#"
insert into chapter_pages(id, release_id, sort_order, extension)
select gen_random_uuid(), $1, null, 'png'
from generate_series(1, 201)
returning id;
        "#,
        release_id
    )
    .fetch_all(&app.pool)
    .await
    .unwrap();

    let resp = app.post_commit(release_id, &staged).await;
    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;

    let resp = app.post_commit(release_id, &staged[..200]).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn release_routes_with_a_malformed_identifier_return_400() {
    let app = TestApp::new().await;

    let req = axum::http::Request::get("/releases/not-a-uuid")
        .body(axum::body::Body::empty())
        .unwrap();
    let resp = tower::ServiceExt::oneshot(app.router.clone(), req)
        .await
        .unwrap();

    assert_error(resp, StatusCode::BAD_REQUEST).await;
}
