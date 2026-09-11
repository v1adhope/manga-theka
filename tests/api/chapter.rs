use axum::http::StatusCode;

use crate::helpers::fakers::{ChapterFaker, LANGUAGES};
use crate::helpers::{RespWrapper, TestApp, assert_error, assert_stored, localization_keys};
use fake::Fake;
use manga_theka::entity::{Chapter, ChapterNumber, Timestamp};

#[tokio::test]
async fn store_chapter_with_valid_body_passes() {
    let app = TestApp::new().await;
    let book_id = app.db_insert_random_book().await;
    let chapter: Chapter = ChapterFaker {
        book_id,
        localizations: 1..=3,
    }
    .fake();

    let body = serde_json::json!({
        "number": chapter.number,
        "name": chapter.name,
        "volume": chapter.volume,
        "localizations": &chapter.localizations,
    });

    let resp = app
        .post_json(&format!("/books/{book_id}/chapters"), body)
        .await;
    let id = assert_stored(resp).await;

    let got = app.db_fetch_chapter(id).await;

    assert_eq!(got.book_id, book_id);
    assert_eq!(got.number, chapter.number);
    assert_eq!(got.name, chapter.name);
    assert_eq!(got.volume, chapter.volume);
    assert_eq!(
        localization_keys(got.localizations.as_slice()),
        localization_keys(chapter.localizations.as_slice())
    );
    assert!(got.updated_at.is_none());
    assert_ne!(got.created_at, Timestamp::UNIX_EPOCH);
}

#[tokio::test]
async fn store_chapter_with_only_a_number_passes() {
    let app = TestApp::new().await;
    let book_id = app.db_insert_random_book().await;

    let resp = app
        .post_json(
            &format!("/books/{book_id}/chapters"),
            serde_json::json!({ "number": 12.5 }),
        )
        .await;
    let id = assert_stored(resp).await;

    let got = app.db_fetch_chapter(id).await;

    assert_eq!(got.number, ChapterNumber::try_from(12.5).unwrap());
    assert!(got.name.is_none());
    assert!(got.volume.is_none());
    assert!(got.localizations.is_empty());
}

#[tokio::test]
async fn store_chapter_with_duplicate_number_returns_409() {
    let app = TestApp::new().await;
    let book_id = app.db_insert_random_book().await;
    let mut chapter: Chapter = ChapterFaker::new(book_id).fake();
    chapter.number = ChapterNumber::try_from(7.0).unwrap();

    app.db_insert_chapter(&chapter).await;

    let resp = app
        .post_json(
            &format!("/books/{book_id}/chapters"),
            serde_json::json!({ "number": 7 }),
        )
        .await;
    assert_error(resp, StatusCode::CONFLICT).await;
}

#[tokio::test]
async fn store_chapter_reuses_a_number_taken_in_another_book() {
    let app = TestApp::new().await;
    let book_id = app.db_insert_random_book().await;
    let other_book_id = app.db_insert_random_book().await;
    let mut chapter: Chapter = ChapterFaker::new(other_book_id).fake();
    chapter.number = ChapterNumber::try_from(7.0).unwrap();

    app.db_insert_chapter(&chapter).await;

    let resp = app
        .post_json(
            &format!("/books/{book_id}/chapters"),
            serde_json::json!({ "number": 7 }),
        )
        .await;
    let id = assert_stored(resp).await;

    let got = app.db_fetch_chapter(id).await;

    assert_eq!(got.book_id, book_id);
}

#[tokio::test]
async fn store_chapter_with_unknown_book_returns_404() {
    let app = TestApp::new().await;

    let resp = app
        .post_json(
            &format!("/books/{}/chapters", uuid::Uuid::now_v7()),
            serde_json::json!({ "number": 1 }),
        )
        .await;
    assert_error(resp, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn store_chapter_with_repeated_localization_language_returns_422() {
    let app = TestApp::new().await;
    let book_id = app.db_insert_random_book().await;
    let language = LANGUAGES.first().expect("a seeded language must exist");

    let body = serde_json::json!({
        "number": 1,
        "localizations": [
            { "languageId": language.id, "name": "First" },
            { "languageId": language.id, "name": "Second" },
        ],
    });

    let resp = app
        .post_json(&format!("/books/{book_id}/chapters"), body)
        .await;
    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;

    let count = sqlx::query_scalar!(r#"select count(*) as "count!" from chapters"#)
        .fetch_one(&app.pool)
        .await
        .unwrap();

    assert_eq!(count, 0, "failed write must roll back the chapter");
}

#[tokio::test]
async fn store_chapter_with_broken_json_returns_400() {
    let app = TestApp::new().await;
    let book_id = app.db_insert_random_book().await;

    let resp = app
        .post_raw(&format!("/books/{book_id}/chapters"), "{not json")
        .await;
    assert_error(resp, StatusCode::BAD_REQUEST).await;
}

#[tokio::test]
async fn get_chapter_with_valid_id_embeds_its_localizations() {
    let app = TestApp::new().await;
    let book_id = app.db_insert_random_book().await;
    let chapter: Chapter = ChapterFaker {
        book_id,
        localizations: 1..=3,
    }
    .fake();

    app.db_insert_chapter(&chapter).await;

    let got = app
        .get_ok_json::<RespWrapper<Chapter>>(&format!("/chapters/{}", chapter.id))
        .await
        .data;

    assert_eq!(got.id, chapter.id);
    assert_eq!(got.book_id, book_id);
    assert_eq!(got.number, chapter.number);
    assert_eq!(got.name, chapter.name);
    assert_eq!(got.volume, chapter.volume);
    assert_eq!(
        localization_keys(got.localizations.as_slice()),
        localization_keys(chapter.localizations.as_slice())
    );
    assert_eq!(got.updated_at, chapter.updated_at);
}

#[tokio::test]
async fn get_chapter_with_unknown_id_returns_404() {
    let app = TestApp::new().await;

    let resp = app
        .get_raw(&format!("/chapters/{}", uuid::Uuid::now_v7()))
        .await;
    assert_error(resp, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn get_chapter_resolves_without_a_book_scope() {
    let app = TestApp::new().await;
    app.db_insert_random_book().await;
    let owner_id = app.db_insert_random_book().await;
    let chapter_id = app.db_insert_random_chapter(owner_id).await;

    let got = app
        .get_ok_json::<RespWrapper<Chapter>>(&format!("/chapters/{chapter_id}"))
        .await;

    assert_eq!(
        got.data.book_id, owner_id,
        "the flat route must name the owning book in the payload"
    );
}

#[tokio::test]
async fn get_chapters_defaults_to_descending_number_order() {
    let app = TestApp::new().await;
    let book_id = app.db_insert_random_book().await;
    let numbers = [13.0, 0.0, 12.5, 12.0];

    app.db_insert_numbered_chapters(book_id, &numbers).await;

    let listed = app
        .get_chapters(&format!("/books/{book_id}/chapters"))
        .await;
    let got: Vec<f32> = listed.data.iter().map(|c| c.number.as_f32()).collect();

    assert_eq!(got, vec![13.0, 12.5, 12.0, 0.0]);
}

#[tokio::test]
async fn get_chapters_with_asc_order_reverses_the_page() {
    let app = TestApp::new().await;
    let book_id = app.db_insert_random_book().await;
    let numbers = [1.0, 2.0, 3.0];

    app.db_insert_numbered_chapters(book_id, &numbers).await;

    let listed = app
        .get_chapters(&format!("/books/{book_id}/chapters?order=Asc"))
        .await;
    let got: Vec<f32> = listed.data.iter().map(|c| c.number.as_f32()).collect();

    assert_eq!(got, vec![1.0, 2.0, 3.0]);
}

#[tokio::test]
async fn get_chapters_lists_only_its_own_books_chapters() {
    let app = TestApp::new().await;
    let book_id = app.db_insert_random_book().await;
    let other_book_id = app.db_insert_random_book().await;

    app.db_insert_numbered_chapters(book_id, &[1.0]).await;
    app.db_insert_numbered_chapters(other_book_id, &[1.0]).await;

    let listed = app
        .get_chapters(&format!("/books/{book_id}/chapters"))
        .await;

    assert_eq!(listed.data.len(), 1);
    assert_eq!(listed.data[0].book_id, book_id);
}

#[tokio::test]
async fn get_chapters_returns_default_limit_and_next_cursor() {
    let app = TestApp::new().await;
    let book_id = app.db_insert_random_book().await;
    let numbers: Vec<f32> = (0..25).map(|n| n as f32).collect();

    app.db_insert_numbered_chapters(book_id, &numbers).await;

    let listed = app
        .get_chapters(&format!("/books/{book_id}/chapters"))
        .await;

    assert_eq!(listed.data.len(), 20);
    assert!(listed.next_cursor.is_some());
}

#[tokio::test]
async fn get_chapters_with_after_and_limit_3_returns_next_page() {
    let app = TestApp::new().await;
    let book_id = app.db_insert_random_book().await;
    let numbers: Vec<f32> = (0..6).map(|n| n as f32).collect();

    app.db_insert_numbered_chapters(book_id, &numbers).await;

    let first = app
        .get_chapters(&format!("/books/{book_id}/chapters?limit=3"))
        .await;
    let first_numbers: Vec<f32> = first.data.iter().map(|c| c.number.as_f32()).collect();
    let cursor = first.next_cursor.unwrap();

    assert_eq!(first_numbers, vec![5.0, 4.0, 3.0]);

    let second = app
        .get_chapters(&format!("/books/{book_id}/chapters?limit=3&after={cursor}"))
        .await;
    let second_numbers: Vec<f32> = second.data.iter().map(|c| c.number.as_f32()).collect();

    assert_eq!(second_numbers, vec![2.0, 1.0, 0.0]);
    assert!(second.next_cursor.is_none());
}

#[tokio::test]
async fn get_chapters_with_after_walks_ascending_pages() {
    let app = TestApp::new().await;
    let book_id = app.db_insert_random_book().await;
    let numbers: Vec<f32> = (0..4).map(|n| n as f32).collect();

    app.db_insert_numbered_chapters(book_id, &numbers).await;

    let first = app
        .get_chapters(&format!("/books/{book_id}/chapters?order=Asc&limit=2"))
        .await;
    let cursor = first.next_cursor.unwrap();

    let second = app
        .get_chapters(&format!(
            "/books/{book_id}/chapters?order=Asc&limit=2&after={cursor}"
        ))
        .await;
    let second_numbers: Vec<f32> = second.data.iter().map(|c| c.number.as_f32()).collect();

    assert_eq!(second_numbers, vec![2.0, 3.0]);
}

#[tokio::test]
async fn get_chapters_with_unknown_after_returns_an_empty_page() {
    let app = TestApp::new().await;
    let book_id = app.db_insert_random_book().await;

    app.db_insert_numbered_chapters(book_id, &[1.0, 2.0]).await;

    let listed = app
        .get_chapters(&format!(
            "/books/{book_id}/chapters?after={}",
            uuid::Uuid::now_v7()
        ))
        .await;

    assert!(listed.data.is_empty());
    assert!(listed.next_cursor.is_none());
}

#[tokio::test]
async fn get_chapters_embeds_each_chapters_own_localizations() {
    let app = TestApp::new().await;
    let book_id = app.db_insert_random_book().await;

    let mut bare: Chapter = ChapterFaker {
        book_id,
        localizations: 0..=0,
    }
    .fake();
    bare.number = ChapterNumber::try_from(1.0).unwrap();

    let mut full: Chapter = ChapterFaker {
        book_id,
        localizations: 1..=3,
    }
    .fake();
    full.number = ChapterNumber::try_from(2.0).unwrap();

    app.db_insert_chapter(&bare).await;
    app.db_insert_chapter(&full).await;

    let listed = app
        .get_chapters(&format!("/books/{book_id}/chapters"))
        .await;

    let got_bare = listed.data.iter().find(|c| c.id == bare.id).unwrap();
    assert!(got_bare.localizations.is_empty());

    let got_full = listed.data.iter().find(|c| c.id == full.id).unwrap();
    assert_eq!(
        localization_keys(got_full.localizations.as_slice()),
        localization_keys(full.localizations.as_slice())
    );
}

#[tokio::test]
async fn get_chapters_with_unknown_book_returns_404() {
    let app = TestApp::new().await;

    let resp = app
        .get_raw(&format!("/books/{}/chapters", uuid::Uuid::now_v7()))
        .await;
    assert_error(resp, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn get_chapters_zero_limit_returns_422() {
    let app = TestApp::new().await;
    let book_id = app.db_insert_random_book().await;

    let resp = app
        .get_raw(&format!("/books/{book_id}/chapters?limit=0"))
        .await;
    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;
}

#[tokio::test]
async fn get_chapters_unknown_order_returns_400() {
    let app = TestApp::new().await;
    let book_id = app.db_insert_random_book().await;

    let resp = app
        .get_raw(&format!("/books/{book_id}/chapters?order=sideways"))
        .await;
    assert_error(resp, StatusCode::BAD_REQUEST).await;
}

#[tokio::test]
async fn get_chapters_with_malformed_after_returns_400() {
    let app = TestApp::new().await;
    let book_id = app.db_insert_random_book().await;

    let resp = app
        .get_raw(&format!("/books/{book_id}/chapters?after=not-a-uuid"))
        .await;
    assert_error(resp, StatusCode::BAD_REQUEST).await;
}

#[tokio::test]
async fn update_chapter_with_valid_body_passes() {
    let app = TestApp::new().await;
    let book_id = app.db_insert_random_book().await;
    let chapter: Chapter = ChapterFaker {
        book_id,
        localizations: 1..=3,
    }
    .fake();

    app.db_insert_chapter(&chapter).await;

    let updated: Chapter = ChapterFaker {
        book_id,
        localizations: 1..=3,
    }
    .fake();

    let body = serde_json::json!({
        "number": updated.number,
        "name": updated.name,
        "volume": updated.volume,
        "localizations": &updated.localizations,
    });

    let resp = app
        .put_json(&format!("/chapters/{}", chapter.id), body)
        .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let got = app.db_fetch_chapter(chapter.id).await;

    assert_eq!(got.number, updated.number);
    assert_eq!(got.name, updated.name);
    assert_eq!(got.volume, updated.volume);
    assert_eq!(
        localization_keys(got.localizations.as_slice()),
        localization_keys(updated.localizations.as_slice()),
        "localizations must be replaced wholesale"
    );
    assert!(got.updated_at.is_some());
    assert_eq!(
        got.created_at.unix_timestamp(),
        chapter.created_at.unix_timestamp(),
        "created_at must stay immutable"
    );
}

#[tokio::test]
async fn update_chapter_with_empty_localizations_detaches_everything() {
    let app = TestApp::new().await;
    let book_id = app.db_insert_random_book().await;
    let chapter: Chapter = ChapterFaker {
        book_id,
        localizations: 1..=3,
    }
    .fake();

    app.db_insert_chapter(&chapter).await;

    let body = serde_json::json!({
        "number": chapter.number,
        "localizations": [],
    });

    let resp = app
        .put_json(&format!("/chapters/{}", chapter.id), body)
        .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let sample = app.db_fetch_chapter_sample(chapter.id).await;

    assert_eq!(sample.localizations, 0);
    assert!(sample.name.is_none(), "an omitted name must be cleared");
}

#[tokio::test]
async fn update_chapter_renumbers_without_touching_other_chapters() {
    let app = TestApp::new().await;
    let book_id = app.db_insert_random_book().await;
    let mut chapter: Chapter = ChapterFaker::new(book_id).fake();
    chapter.number = ChapterNumber::try_from(1.0).unwrap();
    let mut other: Chapter = ChapterFaker::new(book_id).fake();
    other.number = ChapterNumber::try_from(2.0).unwrap();

    app.db_insert_chapter(&chapter).await;
    app.db_insert_chapter(&other).await;

    let resp = app
        .put_json(
            &format!("/chapters/{}", chapter.id),
            serde_json::json!({ "number": 1.5 }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let sample = app.db_fetch_chapter_sample(chapter.id).await;
    assert_eq!(sample.number, Some(1.5));

    let other_sample = app.db_fetch_chapter_sample(other.id).await;
    assert_eq!(other_sample.number, Some(2.0));
    assert!(other_sample.updated_at.is_none());
    assert_eq!(other_sample.localizations, other.localizations.len() as i64);
}

#[tokio::test]
async fn update_chapter_to_a_taken_number_returns_409() {
    let app = TestApp::new().await;
    let book_id = app.db_insert_random_book().await;
    let mut chapter: Chapter = ChapterFaker::new(book_id).fake();
    chapter.number = ChapterNumber::try_from(1.0).unwrap();
    let mut other: Chapter = ChapterFaker::new(book_id).fake();
    other.number = ChapterNumber::try_from(2.0).unwrap();

    app.db_insert_chapter(&chapter).await;
    app.db_insert_chapter(&other).await;

    let resp = app
        .put_json(
            &format!("/chapters/{}", chapter.id),
            serde_json::json!({ "number": 2 }),
        )
        .await;
    assert_error(resp, StatusCode::CONFLICT).await;
}

#[tokio::test]
async fn update_chapter_with_unknown_id_returns_404() {
    let app = TestApp::new().await;

    let resp = app
        .put_json(
            &format!("/chapters/{}", uuid::Uuid::now_v7()),
            serde_json::json!({ "number": 1 }),
        )
        .await;
    assert_error(resp, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn update_chapter_leaves_the_owning_book_untouched() {
    let app = TestApp::new().await;
    app.db_insert_random_book().await;
    let owner_id = app.db_insert_random_book().await;
    let chapter_id = app.db_insert_random_chapter(owner_id).await;

    let resp = app
        .put_json(
            &format!("/chapters/{chapter_id}"),
            serde_json::json!({ "number": 1 }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let got = app.db_fetch_chapter(chapter_id).await;

    assert_eq!(got.number, ChapterNumber::try_from(1.0).unwrap());
    assert_eq!(
        got.book_id, owner_id,
        "a chapter never changes books on update"
    );
}

#[tokio::test]
async fn delete_chapter_with_valid_id_passes() {
    let app = TestApp::new().await;
    let book_id = app.db_insert_random_book().await;
    let chapter_id = app.db_insert_random_chapter(book_id).await;

    let resp = app.delete_chapter(chapter_id).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let sample = app.db_fetch_chapter_sample(chapter_id).await;

    assert!(sample.number.is_none());
    assert_eq!(sample.localizations, 0);
}

#[tokio::test]
async fn delete_chapter_leaves_other_chapters_untouched() {
    let app = TestApp::new().await;
    let book_id = app.db_insert_random_book().await;
    let mut chapter: Chapter = ChapterFaker::new(book_id).fake();
    chapter.number = ChapterNumber::try_from(1.0).unwrap();
    let mut other: Chapter = ChapterFaker {
        book_id,
        localizations: 1..=3,
    }
    .fake();
    other.number = ChapterNumber::try_from(2.0).unwrap();

    app.db_insert_chapter(&chapter).await;
    app.db_insert_chapter(&other).await;

    let resp = app.delete_chapter(chapter.id).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let sample = app.db_fetch_chapter_sample(other.id).await;

    assert_eq!(sample.number, Some(2.0));
    assert_eq!(sample.localizations, other.localizations.len() as i64);
}

#[tokio::test]
async fn delete_chapter_with_unknown_id_returns_404() {
    let app = TestApp::new().await;

    let resp = app.delete_chapter(uuid::Uuid::now_v7()).await;
    assert_error(resp, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn delete_chapter_leaves_other_books_chapters_alone() {
    let app = TestApp::new().await;
    let bystander_book_id = app.db_insert_random_book().await;
    let owner_id = app.db_insert_random_book().await;
    let chapter_id = app.db_insert_random_chapter(owner_id).await;
    let bystander_chapter_id = app.db_insert_random_chapter(bystander_book_id).await;

    let resp = app.delete_chapter(chapter_id).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let listed = app
        .get_chapters(&format!("/books/{bystander_book_id}/chapters"))
        .await;

    assert_eq!(listed.data.len(), 1);
    assert_eq!(
        listed.data.first().map(|c| c.id),
        Some(bystander_chapter_id)
    );
}

#[tokio::test]
async fn delete_book_cascades_its_chapters() {
    let app = TestApp::new().await;
    let book_id = app.db_insert_random_book().await;
    let chapter_id = app.db_insert_random_chapter(book_id).await;

    let resp = app.delete_book(book_id).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let sample = app.db_fetch_chapter_sample(chapter_id).await;

    assert!(sample.number.is_none());
    assert_eq!(sample.localizations, 0);
}
