use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use http_body_util::BodyExt;
use tower::ServiceExt;

use crate::fakers::{ChapterFaker, LANGUAGES};
use crate::helpers::{RespWrapper, TestApp, assert_error, assert_stored, localization_keys};
use fake::Fake;
use manga_theka::entity::{Chapter, ChapterNumber, ChapterTitle, Volume};

#[tokio::test]
async fn store_chapter_with_valid_body_passes() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
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
    })
    .to_string();

    let req = Request::post(format!("/books/{book_id}/chapters"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.router.clone().oneshot(req).await.unwrap();
    let id = assert_stored(resp).await;

    let got = app.fetch_chapter(id).await;

    assert_eq!(got.book_id, book_id);
    assert_eq!(got.number, chapter.number);
    assert_eq!(got.name, chapter.name);
    assert_eq!(got.volume, chapter.volume);
    assert_eq!(
        localization_keys(&got.localizations),
        localization_keys(&chapter.localizations)
    );
    assert!(got.updated_at.is_none());
    assert_ne!(got.created_at, time::OffsetDateTime::UNIX_EPOCH);
}

#[tokio::test]
async fn store_chapter_with_only_a_number_passes() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;

    let body = serde_json::json!({ "number": 1 }).to_string();

    let req = Request::post(format!("/books/{book_id}/chapters"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.router.clone().oneshot(req).await.unwrap();
    let id = assert_stored(resp).await;

    let got = app.fetch_chapter(id).await;

    assert_eq!(got.number, ChapterNumber::try_from(1.0).unwrap());
    assert!(got.name.is_none());
    assert!(got.volume.is_none());
    assert!(got.localizations.is_empty());
}

#[tokio::test]
async fn store_chapter_with_fractional_number_passes() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;

    let body = serde_json::json!({ "number": 12.5 }).to_string();

    let req = Request::post(format!("/books/{book_id}/chapters"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.router.clone().oneshot(req).await.unwrap();
    let id = assert_stored(resp).await;

    let sample = app.fetch_chapter_sample(id).await;

    assert_eq!(sample.number, Some(12.5));
}

#[tokio::test]
async fn store_chapter_with_zero_number_and_volume_passes() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;

    let body = serde_json::json!({ "number": 0, "volume": 0 }).to_string();

    let req = Request::post(format!("/books/{book_id}/chapters"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.router.clone().oneshot(req).await.unwrap();
    let id = assert_stored(resp).await;

    let got = app.fetch_chapter(id).await;

    assert_eq!(got.number, ChapterNumber::try_from(0.0).unwrap());
    assert_eq!(got.volume, Some(Volume::try_from(0).unwrap()));
}

#[tokio::test]
async fn store_chapter_with_duplicate_number_returns_409() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let mut chapter: Chapter = ChapterFaker::new(book_id).fake();
    chapter.number = ChapterNumber::try_from(7.0).unwrap();

    app.insert_chapter(&chapter).await;

    let body = serde_json::json!({ "number": 7 }).to_string();

    let req = Request::post(format!("/books/{book_id}/chapters"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();
    assert_error(resp, StatusCode::CONFLICT).await;
}

#[tokio::test]
async fn store_chapter_reuses_a_number_taken_in_another_book() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let other_book_id = app.insert_random_book().await;
    let mut chapter: Chapter = ChapterFaker::new(other_book_id).fake();
    chapter.number = ChapterNumber::try_from(7.0).unwrap();

    app.insert_chapter(&chapter).await;

    let body = serde_json::json!({ "number": 7 }).to_string();

    let req = Request::post(format!("/books/{book_id}/chapters"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.router.clone().oneshot(req).await.unwrap();
    let id = assert_stored(resp).await;

    let got = app.fetch_chapter(id).await;

    assert_eq!(got.book_id, book_id);
}

#[tokio::test]
async fn store_chapter_with_unknown_book_returns_404() {
    let app = TestApp::new().await;

    let body = serde_json::json!({ "number": 1 }).to_string();

    let req = Request::post(format!("/books/{}/chapters", uuid::Uuid::now_v7()))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();
    assert_error(resp, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn store_chapter_with_negative_number_returns_422() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;

    let body = serde_json::json!({ "number": -1 }).to_string();

    let req = Request::post(format!("/books/{book_id}/chapters"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();
    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;
}

#[tokio::test]
async fn store_chapter_above_the_number_limit_returns_422() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;

    let body = serde_json::json!({ "number": 100_000 }).to_string();

    let req = Request::post(format!("/books/{book_id}/chapters"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();
    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;
}

#[tokio::test]
async fn store_chapter_with_negative_volume_returns_422() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;

    let body = serde_json::json!({ "number": 1, "volume": -1 }).to_string();

    let req = Request::post(format!("/books/{book_id}/chapters"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.router.clone().oneshot(req).await.unwrap();
    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;

    let count = sqlx::query_scalar!(r#"select count(*) as "count!" from chapters"#)
        .fetch_one(&app.pool)
        .await
        .unwrap();

    assert_eq!(count, 0, "a rejected volume must not write a chapter");
}

#[tokio::test]
async fn store_chapter_with_repeated_localization_language_returns_422() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let language = LANGUAGES.first().expect("a seeded language must exist");

    let body = serde_json::json!({
        "number": 1,
        "localizations": [
            { "languageId": language.id, "name": "First" },
            { "languageId": language.id, "name": "Second" },
        ],
    })
    .to_string();

    let req = Request::post(format!("/books/{book_id}/chapters"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.router.clone().oneshot(req).await.unwrap();
    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;

    let count = sqlx::query_scalar!(r#"select count(*) as "count!" from chapters"#)
        .fetch_one(&app.pool)
        .await
        .unwrap();

    assert_eq!(count, 0, "failed write must roll back the chapter");
}

#[tokio::test]
async fn store_chapter_with_unknown_localization_language_returns_422() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;

    let body = serde_json::json!({
        "number": 1,
        "localizations": [{ "languageId": uuid::Uuid::now_v7(), "name": "First" }],
    })
    .to_string();

    let req = Request::post(format!("/books/{book_id}/chapters"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();
    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;
}

#[tokio::test]
async fn store_chapter_with_broken_json_returns_400() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;

    let req = Request::post(format!("/books/{book_id}/chapters"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{not json"))
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();
    assert_error(resp, StatusCode::BAD_REQUEST).await;
}

#[tokio::test]
async fn get_chapter_with_valid_id_embeds_its_localizations() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter: Chapter = ChapterFaker {
        book_id,
        localizations: 1..=3,
    }
    .fake();

    app.insert_chapter(&chapter).await;

    let req = Request::get(format!("/chapters/{}", chapter.id))
        .body(Body::empty())
        .unwrap();
    let resp = app.router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let wrapper: RespWrapper<Chapter> = serde_json::from_slice(&bytes).unwrap();
    let got = wrapper.data;

    assert_eq!(got.id, chapter.id);
    assert_eq!(got.book_id, book_id);
    assert_eq!(got.number, chapter.number);
    assert_eq!(got.name, chapter.name);
    assert_eq!(got.volume, chapter.volume);
    assert_eq!(
        localization_keys(&got.localizations),
        localization_keys(&chapter.localizations)
    );
    assert_eq!(got.updated_at, chapter.updated_at);
}

#[tokio::test]
async fn get_chapter_holds_at_most_one_localization_per_language() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter: Chapter = ChapterFaker {
        book_id,
        localizations: 3..=3,
    }
    .fake();

    app.insert_chapter(&chapter).await;

    let req = Request::get(format!("/chapters/{}", chapter.id))
        .body(Body::empty())
        .unwrap();
    let resp = app.router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let wrapper: RespWrapper<Chapter> = serde_json::from_slice(&bytes).unwrap();

    let mut language_ids: Vec<uuid::Uuid> = wrapper
        .data
        .localizations
        .iter()
        .map(|l| l.language_id)
        .collect();
    let total = language_ids.len();
    language_ids.sort();
    language_ids.dedup();

    assert_eq!(language_ids.len(), total);
}

#[tokio::test]
async fn get_chapter_with_unknown_id_returns_404() {
    let app = TestApp::new().await;

    let req = Request::get(format!("/chapters/{}", uuid::Uuid::now_v7()))
        .body(Body::empty())
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();
    assert_error(resp, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn get_chapter_resolves_without_a_book_scope() {
    let app = TestApp::new().await;
    app.insert_random_book().await;
    let owner_id = app.insert_random_book().await;
    let chapter: Chapter = ChapterFaker::new(owner_id).fake();

    app.insert_chapter(&chapter).await;

    let req = Request::get(format!("/chapters/{}", chapter.id))
        .body(Body::empty())
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let wrapper: RespWrapper<Chapter> = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(
        wrapper.data.book_id, owner_id,
        "the flat route must name the owning book in the payload"
    );
}

async fn insert_numbered_chapters(app: &TestApp, book_id: uuid::Uuid, numbers: &[f32]) {
    for number in numbers {
        let mut chapter: Chapter = ChapterFaker {
            book_id,
            localizations: 0..=0,
        }
        .fake();
        chapter.number = ChapterNumber::try_from(*number).unwrap();
        app.insert_chapter(&chapter).await;
    }
}

async fn list_chapters(app: &TestApp, path: String) -> RespWrapper<Vec<Chapter>> {
    let req = Request::get(path).body(Body::empty()).unwrap();
    let resp = app.router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn get_chapters_defaults_to_descending_number_order() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let numbers = [13.0, 0.0, 12.5, 12.0];

    insert_numbered_chapters(&app, book_id, &numbers).await;

    let listed = list_chapters(&app, format!("/books/{book_id}/chapters")).await;
    let got: Vec<f32> = listed.data.iter().map(|c| c.number.as_f32()).collect();

    assert_eq!(got, vec![13.0, 12.5, 12.0, 0.0]);
}

#[tokio::test]
async fn get_chapters_with_asc_order_reverses_the_page() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let numbers = [1.0, 2.0, 3.0];

    insert_numbered_chapters(&app, book_id, &numbers).await;

    let listed = list_chapters(&app, format!("/books/{book_id}/chapters?order=Asc")).await;
    let got: Vec<f32> = listed.data.iter().map(|c| c.number.as_f32()).collect();

    assert_eq!(got, vec![1.0, 2.0, 3.0]);
}

#[tokio::test]
async fn get_chapters_lists_only_its_own_books_chapters() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let other_book_id = app.insert_random_book().await;

    insert_numbered_chapters(&app, book_id, &[1.0]).await;
    insert_numbered_chapters(&app, other_book_id, &[1.0]).await;

    let listed = list_chapters(&app, format!("/books/{book_id}/chapters")).await;

    assert_eq!(listed.data.len(), 1);
    assert_eq!(listed.data[0].book_id, book_id);
}

#[tokio::test]
async fn get_chapters_returns_default_limit_and_next_cursor() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let numbers: Vec<f32> = (0..25).map(|n| n as f32).collect();

    insert_numbered_chapters(&app, book_id, &numbers).await;

    let listed = list_chapters(&app, format!("/books/{book_id}/chapters")).await;

    assert_eq!(listed.data.len(), 20);
    assert!(listed.next_cursor.is_some());
}

#[tokio::test]
async fn get_chapters_with_after_and_limit_3_returns_next_page() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let numbers: Vec<f32> = (0..6).map(|n| n as f32).collect();

    insert_numbered_chapters(&app, book_id, &numbers).await;

    let first = list_chapters(&app, format!("/books/{book_id}/chapters?limit=3")).await;
    let first_numbers: Vec<f32> = first.data.iter().map(|c| c.number.as_f32()).collect();
    let cursor = first.next_cursor.unwrap();

    assert_eq!(first_numbers, vec![5.0, 4.0, 3.0]);

    let second = list_chapters(
        &app,
        format!("/books/{book_id}/chapters?limit=3&after={cursor}"),
    )
    .await;
    let second_numbers: Vec<f32> = second.data.iter().map(|c| c.number.as_f32()).collect();

    assert_eq!(second_numbers, vec![2.0, 1.0, 0.0]);
    assert!(second.next_cursor.is_none());
}

#[tokio::test]
async fn get_chapters_with_after_walks_ascending_pages() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let numbers: Vec<f32> = (0..4).map(|n| n as f32).collect();

    insert_numbered_chapters(&app, book_id, &numbers).await;

    let first = list_chapters(&app, format!("/books/{book_id}/chapters?order=Asc&limit=2")).await;
    let cursor = first.next_cursor.unwrap();

    let second = list_chapters(
        &app,
        format!("/books/{book_id}/chapters?order=Asc&limit=2&after={cursor}"),
    )
    .await;
    let second_numbers: Vec<f32> = second.data.iter().map(|c| c.number.as_f32()).collect();

    assert_eq!(second_numbers, vec![2.0, 3.0]);
}

#[tokio::test]
async fn get_chapters_embeds_each_chapters_own_localizations() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;

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

    app.insert_chapter(&bare).await;
    app.insert_chapter(&full).await;

    let listed = list_chapters(&app, format!("/books/{book_id}/chapters")).await;

    let got_bare = listed.data.iter().find(|c| c.id == bare.id).unwrap();
    assert!(got_bare.localizations.is_empty());

    let got_full = listed.data.iter().find(|c| c.id == full.id).unwrap();
    assert_eq!(
        localization_keys(&got_full.localizations),
        localization_keys(&full.localizations)
    );
}

#[tokio::test]
async fn get_chapters_with_unknown_book_returns_404() {
    let app = TestApp::new().await;

    let req = Request::get(format!("/books/{}/chapters", uuid::Uuid::now_v7()))
        .body(Body::empty())
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();
    assert_error(resp, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn get_chapters_zero_limit_returns_422() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;

    let req = Request::get(format!("/books/{book_id}/chapters?limit=0"))
        .body(Body::empty())
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();
    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;
}

#[tokio::test]
async fn get_chapters_unknown_order_returns_400() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;

    let req = Request::get(format!("/books/{book_id}/chapters?order=sideways"))
        .body(Body::empty())
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();
    assert_error(resp, StatusCode::BAD_REQUEST).await;
}

#[tokio::test]
async fn update_chapter_with_valid_body_passes() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter: Chapter = ChapterFaker {
        book_id,
        localizations: 1..=3,
    }
    .fake();

    app.insert_chapter(&chapter).await;

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
    })
    .to_string();

    let req = Request::put(format!("/chapters/{}", chapter.id))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();
    let resp = app.router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let got = app.fetch_chapter(chapter.id).await;

    assert_eq!(got.number, updated.number);
    assert_eq!(got.name, updated.name);
    assert_eq!(got.volume, updated.volume);
    assert_eq!(
        localization_keys(&got.localizations),
        localization_keys(&updated.localizations),
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
    let book_id = app.insert_random_book().await;
    let chapter: Chapter = ChapterFaker {
        book_id,
        localizations: 1..=3,
    }
    .fake();

    app.insert_chapter(&chapter).await;

    let body = serde_json::json!({
        "number": chapter.number,
        "localizations": [],
    })
    .to_string();

    let req = Request::put(format!("/chapters/{}", chapter.id))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();
    let resp = app.router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let sample = app.fetch_chapter_sample(chapter.id).await;

    assert_eq!(sample.localizations, 0);
    assert!(sample.name.is_none(), "an omitted name must be cleared");
}

#[tokio::test]
async fn update_chapter_renumbers_without_touching_other_chapters() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let mut chapter: Chapter = ChapterFaker::new(book_id).fake();
    chapter.number = ChapterNumber::try_from(1.0).unwrap();
    let mut other: Chapter = ChapterFaker::new(book_id).fake();
    other.number = ChapterNumber::try_from(2.0).unwrap();

    app.insert_chapter(&chapter).await;
    app.insert_chapter(&other).await;

    let body = serde_json::json!({ "number": 1.5 }).to_string();

    let req = Request::put(format!("/chapters/{}", chapter.id))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();
    let resp = app.router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let sample = app.fetch_chapter_sample(chapter.id).await;
    assert_eq!(sample.number, Some(1.5));

    let other_sample = app.fetch_chapter_sample(other.id).await;
    assert_eq!(other_sample.number, Some(2.0));
    assert!(other_sample.updated_at.is_none());
    assert_eq!(other_sample.localizations, other.localizations.len() as i64);
}

#[tokio::test]
async fn update_chapter_to_a_taken_number_returns_409() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let mut chapter: Chapter = ChapterFaker::new(book_id).fake();
    chapter.number = ChapterNumber::try_from(1.0).unwrap();
    let mut other: Chapter = ChapterFaker::new(book_id).fake();
    other.number = ChapterNumber::try_from(2.0).unwrap();

    app.insert_chapter(&chapter).await;
    app.insert_chapter(&other).await;

    let body = serde_json::json!({ "number": 2 }).to_string();

    let req = Request::put(format!("/chapters/{}", chapter.id))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();
    assert_error(resp, StatusCode::CONFLICT).await;
}

#[tokio::test]
async fn update_chapter_with_unknown_id_returns_404() {
    let app = TestApp::new().await;

    let body = serde_json::json!({ "number": 1 }).to_string();

    let req = Request::put(format!("/chapters/{}", uuid::Uuid::now_v7()))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();
    assert_error(resp, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn update_chapter_leaves_the_owning_book_untouched() {
    let app = TestApp::new().await;
    app.insert_random_book().await;
    let owner_id = app.insert_random_book().await;
    let chapter: Chapter = ChapterFaker::new(owner_id).fake();

    app.insert_chapter(&chapter).await;

    let body = serde_json::json!({ "number": 1 }).to_string();

    let req = Request::put(format!("/chapters/{}", chapter.id))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let got = app.fetch_chapter(chapter.id).await;

    assert_eq!(got.number, ChapterNumber::try_from(1.0).unwrap());
    assert_eq!(
        got.book_id, owner_id,
        "a chapter never changes books on update"
    );
}

#[tokio::test]
async fn delete_chapter_with_valid_id_passes() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter: Chapter = ChapterFaker {
        book_id,
        localizations: 1..=3,
    }
    .fake();

    app.insert_chapter(&chapter).await;

    let req = Request::delete(format!("/chapters/{}", chapter.id))
        .body(Body::empty())
        .unwrap();

    let resp = app.router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let sample = app.fetch_chapter_sample(chapter.id).await;

    assert!(sample.number.is_none());
    assert_eq!(sample.localizations, 0);
}

#[tokio::test]
async fn delete_chapter_leaves_other_chapters_untouched() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let mut chapter: Chapter = ChapterFaker::new(book_id).fake();
    chapter.number = ChapterNumber::try_from(1.0).unwrap();
    let mut other: Chapter = ChapterFaker {
        book_id,
        localizations: 1..=3,
    }
    .fake();
    other.number = ChapterNumber::try_from(2.0).unwrap();

    app.insert_chapter(&chapter).await;
    app.insert_chapter(&other).await;

    let req = Request::delete(format!("/chapters/{}", chapter.id))
        .body(Body::empty())
        .unwrap();
    let resp = app.router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let sample = app.fetch_chapter_sample(other.id).await;

    assert_eq!(sample.number, Some(2.0));
    assert_eq!(sample.localizations, other.localizations.len() as i64);
}

#[tokio::test]
async fn delete_chapter_with_unknown_id_returns_404() {
    let app = TestApp::new().await;

    let req = Request::delete(format!("/chapters/{}", uuid::Uuid::now_v7()))
        .body(Body::empty())
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();
    assert_error(resp, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn delete_chapter_leaves_other_books_chapters_alone() {
    let app = TestApp::new().await;
    let bystander_id = app.insert_random_book().await;
    let owner_id = app.insert_random_book().await;
    let chapter: Chapter = ChapterFaker::new(owner_id).fake();
    let bystander: Chapter = ChapterFaker::new(bystander_id).fake();

    app.insert_chapter(&chapter).await;
    app.insert_chapter(&bystander).await;

    let req = Request::delete(format!("/chapters/{}", chapter.id))
        .body(Body::empty())
        .unwrap();

    let resp = app.router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let listed = list_chapters(&app, format!("/books/{bystander_id}/chapters")).await;

    assert_eq!(listed.data.len(), 1);
    assert_eq!(listed.data.first().map(|c| c.id), Some(bystander.id));
}

#[tokio::test]
async fn delete_book_cascades_its_chapters() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let chapter: Chapter = ChapterFaker {
        book_id,
        localizations: 1..=3,
    }
    .fake();

    app.insert_chapter(&chapter).await;

    let req = Request::delete(format!("/books/{book_id}"))
        .body(Body::empty())
        .unwrap();
    let resp = app.router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let sample = app.fetch_chapter_sample(chapter.id).await;

    assert!(sample.number.is_none());
    assert_eq!(sample.localizations, 0);
}

#[tokio::test]
async fn store_chapter_with_a_255_char_localization_name_passes() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let language = LANGUAGES.first().expect("a seeded language must exist");
    let name = "ё".repeat(255);

    let body = serde_json::json!({
        "number": 1,
        "localizations": [{ "languageId": language.id, "name": name }],
    })
    .to_string();

    let req = Request::post(format!("/books/{book_id}/chapters"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.router.clone().oneshot(req).await.unwrap();
    let id = assert_stored(resp).await;

    let got = app.fetch_chapter(id).await;

    assert_eq!(
        got.localizations.first().map(|l| l.name.as_ref()),
        Some(ChapterTitle::try_from(name).unwrap().as_ref())
    );
}

#[tokio::test]
async fn get_chapters_with_a_cursor_from_another_book_returns_an_empty_page() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;
    let other_book_id = app.insert_random_book().await;

    insert_numbered_chapters(&app, book_id, &[1.0, 2.0, 3.0]).await;
    insert_numbered_chapters(&app, other_book_id, &[0.5]).await;

    let other = list_chapters(&app, format!("/books/{other_book_id}/chapters")).await;
    let foreign_cursor = other.data.first().unwrap().id;

    let listed = list_chapters(
        &app,
        format!("/books/{book_id}/chapters?after={foreign_cursor}"),
    )
    .await;

    assert!(
        listed.data.is_empty(),
        "a cursor from another book must not offset this book's page"
    );
}

#[tokio::test]
async fn get_chapters_with_unknown_after_returns_an_empty_page() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;

    insert_numbered_chapters(&app, book_id, &[1.0, 2.0]).await;

    let listed = list_chapters(
        &app,
        format!("/books/{book_id}/chapters?after={}", uuid::Uuid::now_v7()),
    )
    .await;

    assert!(listed.data.is_empty());
    assert!(listed.next_cursor.is_none());
}

#[tokio::test]
async fn get_chapters_with_malformed_after_returns_400() {
    let app = TestApp::new().await;
    let book_id = app.insert_random_book().await;

    let req = Request::get(format!("/books/{book_id}/chapters?after=not-a-uuid"))
        .body(Body::empty())
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();
    assert_error(resp, StatusCode::BAD_REQUEST).await;
}
