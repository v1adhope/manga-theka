use axum::http::StatusCode;
use time::OffsetDateTime;

use crate::fakers::COVER_PNG;
use crate::helpers::{TestApp, hours_ago, sorted};

#[tokio::test]
async fn a_release_idle_past_the_grace_window_loses_every_staged_page() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;
    app.insert_staged_pages_at(release_id, 3, hours_ago(3))
        .await;

    let deleted = app.delete_stale_staged_chapter_pages().await;

    assert_eq!(deleted, 3);
    assert_eq!(app.count_chapter_pages(release_id).await, 0);
}

#[tokio::test]
async fn one_fresh_staged_page_holds_its_whole_release_alive() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;
    app.insert_staged_pages_at(release_id, 3, hours_ago(9))
        .await;
    app.insert_staged_pages_at(release_id, 1, OffsetDateTime::now_utc())
        .await;

    let deleted = app.delete_stale_staged_chapter_pages().await;

    assert_eq!(deleted, 0);
    assert_eq!(app.count_chapter_pages(release_id).await, 4);
}

#[tokio::test]
async fn committed_pages_are_never_swept() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;
    let committed = app
        .insert_committed_page_at(release_id, 1, hours_ago(9))
        .await;
    app.insert_staged_pages_at(release_id, 2, hours_ago(9))
        .await;

    let deleted = app.delete_stale_staged_chapter_pages().await;

    assert_eq!(deleted, 2);
    assert_eq!(
        app.fetch_committed_page_ids(release_id).await,
        vec![committed]
    );
}

#[tokio::test]
async fn a_swept_release_survives_and_takes_new_staged_pages() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;
    app.insert_staged_pages_at(release_id, 2, hours_ago(3))
        .await;

    app.delete_stale_staged_chapter_pages().await;

    assert_eq!(app.fetch_release_id(release_id).await, Some(release_id));

    let resp = app.post_upload_pages(release_id, &[COVER_PNG]).await;

    assert_eq!(resp.status(), StatusCode::CREATED);
    assert_eq!(app.count_chapter_pages(release_id).await, 1);
}

#[tokio::test]
async fn every_swept_page_leaves_its_storage_key_on_record() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;
    let staged = app
        .insert_staged_pages_at(release_id, 3, hours_ago(3))
        .await;

    let deleted = app.delete_stale_staged_chapter_pages().await;

    let orphans = app.fetch_orphaned_objects().await;
    assert_eq!(orphans.len() as i64, deleted);
    assert_eq!(
        orphans.iter().map(|o| o.object_key).collect::<Vec<_>>(),
        sorted(staged)
    );
    for orphan in &orphans {
        assert_eq!(orphan.kind, "ChapterPage");
        assert_eq!(orphan.source, "StaleStagedPage");
        assert!(orphan.processed_at.is_none());
    }
}

#[tokio::test]
async fn a_sweep_records_a_key_for_every_page_it_deletes() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;

    for pages in [1, 2, 3] {
        let release_id = app.insert_random_release(book_id, chapter_id).await;
        app.insert_staged_pages_at(release_id, pages, hours_ago(3))
            .await;
    }

    let fresh_release_id = app.insert_random_release(book_id, chapter_id).await;
    app.insert_staged_pages_at(fresh_release_id, 4, OffsetDateTime::now_utc())
        .await;

    let deleted = app.delete_stale_staged_chapter_pages().await;

    assert_eq!(deleted, 6);
    assert_eq!(app.count_orphaned_objects().await, deleted);
}

#[tokio::test]
async fn a_sweep_with_nothing_stale_does_nothing() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;
    app.insert_staged_pages_at(release_id, 2, OffsetDateTime::now_utc())
        .await;

    let deleted = app.delete_stale_staged_chapter_pages().await;

    assert_eq!(deleted, 0);
    assert_eq!(app.count_chapter_pages(release_id).await, 2);
    assert_eq!(app.count_orphaned_objects().await, 0);
}

#[tokio::test]
async fn a_sweep_stops_at_the_release_cap_and_the_next_run_drains_the_rest() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    app.seed_staged_releases(book_id, 501, hours_ago(3)).await;

    let first = app.delete_stale_staged_chapter_pages().await;
    let second = app.delete_stale_staged_chapter_pages().await;

    assert_eq!(first, 500);
    assert_eq!(second, 1);
    assert_eq!(app.count_orphaned_objects().await, 501);
}
