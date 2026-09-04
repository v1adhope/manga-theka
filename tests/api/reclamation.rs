use axum::http::StatusCode;
use fake::Fake;
use manga_theka::entity::{BookQuery, BookVisibility};
use time::OffsetDateTime;

use crate::fakers::{ACTION, BookFaker, COVER_JPG, COVER_PNG, EVERY_VISIBILITY, ROMANCE};
use crate::helpers::{TestApp, days_ago, hours_ago, labels, sorted, sweep_stale_staged_pages};

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
async fn a_page_committed_while_a_sweep_runs_survives_it() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;
    let staged = app
        .insert_staged_pages_at(release_id, 1, hours_ago(3))
        .await;

    let tx = app.begin_page_commit(staged[0]).await;

    let pool = app.pool.clone();
    let sweep = tokio::spawn(async move { sweep_stale_staged_pages(&pool).await });

    app.await_blocked_on_a_lock().await;
    tx.commit().await.expect("the page commit must succeed");

    let deleted = sweep.await.expect("the sweep must finish");

    assert_eq!(deleted, 0);
    assert_eq!(app.count_orphaned_objects().await, 0);
    assert_eq!(app.fetch_committed_page_ids(release_id).await, staged);
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

#[tokio::test]
async fn an_empty_rejected_book_past_the_grace_window_is_purged() {
    let app = TestApp::new().await;
    let book_id = app
        .insert_book_with_visibility_at(BookVisibility::Rejected, days_ago(8))
        .await;

    let deleted = app.delete_rejected_books().await;

    assert_eq!(deleted, 1);
    assert_eq!(app.fetch_book_id(book_id).await, None);
}

#[tokio::test]
async fn a_rejected_book_that_still_holds_chapters_is_left_alone() {
    let app = TestApp::new().await;
    let book_id = app
        .insert_book_with_visibility_at(BookVisibility::Rejected, days_ago(400))
        .await;
    app.insert_random_chapter(book_id).await;

    let deleted = app.delete_rejected_books().await;

    assert_eq!(deleted, 0);
    assert_eq!(app.fetch_book_id(book_id).await, Some(book_id));
}

#[tokio::test]
async fn an_emptied_rejected_book_is_reaped_by_the_next_run() {
    let app = TestApp::new().await;
    let book_id = app
        .insert_book_with_visibility_at(BookVisibility::Rejected, days_ago(8))
        .await;
    let chapter_id = app.insert_random_chapter(book_id).await;

    assert_eq!(app.delete_rejected_books().await, 0);

    app.delete_chapter_row(chapter_id).await;

    assert_eq!(app.delete_rejected_books().await, 1);
    assert_eq!(app.fetch_book_id(book_id).await, None);
}

#[tokio::test]
async fn a_purged_book_takes_every_record_that_belonged_to_it() {
    let app = TestApp::new().await;
    let book: BookQuery = BookFaker {
        exact_labels: Some(labels(&[ACTION, ROMANCE])),
        links: 2..=2,
        titles: 2..=2,
        creators: 2..=2,
        visibility: BookVisibility::Rejected,
        updated_at: Some(days_ago(8)),
        ..Default::default()
    }
    .fake();
    app.insert_book(&book).await;
    app.insert_cover(book.id, COVER_PNG).await;

    let deleted = app.delete_rejected_books().await;

    assert_eq!(deleted, 1);

    let sample = app.fetch_book_sample(book.id).await;
    assert_eq!(sample.name, None);
    assert_eq!(sample.labels, 0);
    assert_eq!(sample.links, 0);
    assert_eq!(sample.titles, 0);
    assert_eq!(app.count_book_creators(book.id).await, 0);
    assert!(app.fetch_covers(book.id).await.is_empty());
}

#[tokio::test]
async fn every_purged_cover_leaves_its_storage_key_on_record() {
    let app = TestApp::new().await;
    let book_id = app
        .insert_book_with_visibility_at(BookVisibility::Rejected, days_ago(8))
        .await;
    let covers = vec![
        app.insert_cover(book_id, COVER_PNG).await,
        app.insert_cover(book_id, COVER_JPG).await,
    ];

    let deleted = app.delete_rejected_books().await;

    assert_eq!(deleted, 1);

    let orphans = app.fetch_orphaned_objects().await;
    assert_eq!(
        orphans.iter().map(|o| o.object_key).collect::<Vec<_>>(),
        sorted(covers)
    );
    for orphan in &orphans {
        assert_eq!(orphan.kind, "BookCover");
        assert_eq!(orphan.source, "RejectedBook");
        assert!(orphan.processed_at.is_none());
    }
}

#[tokio::test]
async fn a_purge_records_a_key_for_every_cover_it_strands() {
    let app = TestApp::new().await;
    let mut stranded: i64 = 0;
    for covers in [0, 1, 2] {
        let book_id = app
            .insert_book_with_visibility_at(BookVisibility::Rejected, days_ago(8))
            .await;
        for _ in 0..covers {
            app.insert_cover(book_id, COVER_PNG).await;
            stranded += 1;
        }
    }

    let kept_id = app
        .insert_book_with_visibility_at(BookVisibility::Rejected, days_ago(1))
        .await;
    app.insert_cover(kept_id, COVER_PNG).await;

    let deleted = app.delete_rejected_books().await;

    assert_eq!(deleted, 3);
    assert_eq!(app.count_orphaned_objects().await, stranded);
    assert_eq!(app.fetch_covers(kept_id).await.len(), 1);
}

#[tokio::test]
async fn no_other_visibility_is_ever_purged() {
    let app = TestApp::new().await;
    let mut kept = Vec::new();
    for visibility in EVERY_VISIBILITY
        .into_iter()
        .filter(|v| *v != BookVisibility::Rejected)
    {
        kept.push(
            app.insert_book_with_visibility_at(visibility, days_ago(400))
                .await,
        );
    }

    let deleted = app.delete_rejected_books().await;

    assert_eq!(deleted, 0);
    for id in kept {
        assert_eq!(app.fetch_book_id(id).await, Some(id));
    }
}

#[tokio::test]
async fn a_purge_with_nothing_eligible_does_nothing() {
    let app = TestApp::new().await;

    let deleted = app.delete_rejected_books().await;

    assert_eq!(deleted, 0);
    assert_eq!(app.count_orphaned_objects().await, 0);
}

#[tokio::test]
async fn a_purge_stops_at_the_book_cap_and_the_next_run_drains_the_rest() {
    let app = TestApp::new().await;
    app.seed_rejected_books(5001, days_ago(8)).await;

    let first = app.delete_rejected_books().await;
    let second = app.delete_rejected_books().await;

    assert_eq!(first, 5000);
    assert_eq!(second, 1);
}

#[tokio::test]
async fn a_rejected_book_inside_the_grace_window_is_left_alone() {
    let app = TestApp::new().await;
    let book_id = app
        .insert_book_with_visibility_at(BookVisibility::Rejected, days_ago(6))
        .await;

    let deleted = app.delete_rejected_books().await;

    assert_eq!(deleted, 0);
    assert_eq!(app.fetch_book_id(book_id).await, Some(book_id));
}
