use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use fake::Fake;
use http_body_util::BodyExt;
use manga_theka::entity::{BookQuery, BookVisibility, Role};

use crate::helpers::fakers::{BookFaker, COVER_JPG, COVER_PNG, EVERY_VISIBILITY};
use crate::helpers::{RespWrapper, TestApp, assert_error};

#[tokio::test]
async fn store_book_starts_it_as_a_draft() {
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
    assert_eq!(resp.status(), StatusCode::CREATED);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let id = uuid::Uuid::parse_str(v["data"]["id"].as_str().unwrap()).unwrap();

    let state = app.fetch_book_visibility_state(id).await;

    assert_eq!(state.visibility, "Draft");
}

#[tokio::test]
async fn get_books_returns_listed_books_only() {
    let app = TestApp::new().await;
    let listed = app
        .insert_book_with_visibility(BookVisibility::Listed)
        .await;
    app.insert_book_with_visibility(BookVisibility::Draft).await;

    let resp = app.get_books("/books").await;
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let page: RespWrapper<Vec<BookQuery>> = serde_json::from_slice(&bytes).unwrap();
    let ids: Vec<uuid::Uuid> = page.data.iter().map(|b| b.id).collect();

    assert_eq!(ids, vec![listed]);
}

#[tokio::test]
async fn get_books_with_a_visibility_overrides_the_default_filter() {
    let app = TestApp::new().await;
    app.insert_book_with_visibility(BookVisibility::Listed)
        .await;
    let drafted = app.insert_book_with_visibility(BookVisibility::Draft).await;

    let resp = app.get_as_moderator("/books?visibility=Draft").await;
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let page: RespWrapper<Vec<BookQuery>> = serde_json::from_slice(&bytes).unwrap();
    let ids: Vec<uuid::Uuid> = page.data.iter().map(|b| b.id).collect();

    assert_eq!(ids, vec![drafted]);
}

#[tokio::test]
async fn get_books_with_an_unknown_visibility_returns_400() {
    let app = TestApp::new().await;

    let resp = app.get_books("/books?visibility=Published").await;

    assert_error(resp, StatusCode::BAD_REQUEST).await;
}

#[tokio::test]
async fn get_book_serves_a_listed_book_to_anyone() {
    let app = TestApp::new().await;
    let id = app
        .insert_book_with_visibility(BookVisibility::Listed)
        .await;

    let (status, _) = app.get_body(&format!("/books/{id}")).await;

    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn get_book_reads_a_book_in_every_visibility_for_a_moderator() {
    let app = TestApp::new().await;

    for visibility in EVERY_VISIBILITY {
        let id = app.insert_book_with_visibility(visibility).await;

        let resp = app.get_as_moderator(&format!("/books/{id}")).await;

        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "{visibility} must stay linkable for a moderator"
        );
    }
}

#[tokio::test]
async fn get_book_outside_listed_reads_as_missing_for_non_moderators() {
    let app = TestApp::new().await;
    let absent = uuid::Uuid::now_v7();

    for visibility in EVERY_VISIBILITY {
        if visibility == BookVisibility::Listed {
            continue;
        }

        let id = app.insert_book_with_visibility(visibility).await;

        let anon = app.get_body(&format!("/books/{id}")).await;
        let reader = app
            .get_body_as(&format!("/books/{id}"), &[Role::Reader, Role::Uploader])
            .await;
        let missing = app.get_body(&format!("/books/{absent}")).await;

        assert_eq!(anon.0, StatusCode::NOT_FOUND, "anon while {visibility}");
        assert_eq!(reader.0, StatusCode::NOT_FOUND, "reader while {visibility}");
        assert_eq!(anon, missing, "{visibility} must not reveal that it exists");
    }
}

#[tokio::test]
async fn get_book_carries_its_review_fields() {
    let app = TestApp::new().await;
    let id = app.insert_book_with_visibility(BookVisibility::Draft).await;

    let req = Request::get(format!("/books/{id}"))
        .body(Body::empty())
        .unwrap();
    let resp = app.send(req).await;
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(v["data"]["visibility"], "Draft");
    assert!(v["data"]["note"].is_null());
    assert!(v["data"]["submittedAt"].is_null());
}

#[tokio::test]
async fn get_chapter_pages_of_an_unlisted_book_returns_404() {
    let app = TestApp::new().await;

    for visibility in EVERY_VISIBILITY {
        let book_id = app.insert_book_with_visibility(visibility).await;
        let chapter_id = app.insert_random_chapter(book_id).await;
        let release_id = app.insert_random_release(book_id, chapter_id).await;
        app.insert_page(release_id, Some(1), COVER_PNG).await;

        let req = Request::get(format!("/releases/{release_id}/pages"))
            .body(Body::empty())
            .unwrap();
        let resp = app.send_raw(req).await;

        let expected = match visibility {
            BookVisibility::Listed => StatusCode::OK,
            _ => StatusCode::NOT_FOUND,
        };
        assert_eq!(resp.status(), expected, "pages of a {visibility} book");
    }
}

#[tokio::test]
async fn get_chapter_page_of_an_unlisted_book_returns_404() {
    let app = TestApp::new().await;

    for visibility in EVERY_VISIBILITY {
        let book_id = app.insert_book_with_visibility(visibility).await;
        let chapter_id = app.insert_random_chapter(book_id).await;
        let release_id = app.insert_random_release(book_id, chapter_id).await;
        let page_id = app.insert_page(release_id, Some(1), COVER_PNG).await;

        let by_number = Request::get(format!("/releases/{release_id}/pages/1"))
            .body(Body::empty())
            .unwrap();
        let by_id = Request::get(format!("/releases/{release_id}/pages/{page_id}/image"))
            .body(Body::empty())
            .unwrap();

        let number_status = app.send_raw(by_number).await.status();
        let image_status = app.send_raw(by_id).await.status();

        let expected = match visibility {
            BookVisibility::Listed => StatusCode::FOUND,
            _ => StatusCode::NOT_FOUND,
        };
        assert_eq!(
            number_status, expected,
            "page by number of a {visibility} book"
        );
        assert_eq!(image_status, expected, "page image of a {visibility} book");
    }
}

#[tokio::test]
async fn get_cover_image_of_an_unlisted_book_returns_404() {
    let app = TestApp::new().await;

    for visibility in EVERY_VISIBILITY {
        let book_id = app.insert_book_with_visibility(visibility).await;
        let cover_id = app.insert_cover(book_id, COVER_PNG).await;

        let resp = app.get_cover_image(cover_id).await;

        let expected = match visibility {
            BookVisibility::Listed => StatusCode::FOUND,
            _ => StatusCode::NOT_FOUND,
        };
        assert_eq!(resp.status(), expected, "cover of a {visibility} book");
    }
}

#[tokio::test]
async fn a_moderator_reads_content_in_every_visibility() {
    let app = TestApp::new().await;

    for visibility in EVERY_VISIBILITY {
        let book_id = app.insert_book_with_visibility(visibility).await;
        let cover_id = app.insert_cover(book_id, COVER_PNG).await;
        let chapter_id = app.insert_random_chapter(book_id).await;
        let release_id = app.insert_random_release(book_id, chapter_id).await;
        let page_id = app.insert_page(release_id, Some(1), COVER_PNG).await;

        let cover = app
            .get_as_moderator(&format!("/covers/{cover_id}/image"))
            .await;
        let pages = app
            .get_as_moderator(&format!("/releases/{release_id}/pages"))
            .await;
        let by_number = app
            .get_as_moderator(&format!("/releases/{release_id}/pages/1"))
            .await;
        let image = app
            .get_as_moderator(&format!("/releases/{release_id}/pages/{page_id}/image"))
            .await;

        assert_eq!(
            cover.status(),
            StatusCode::FOUND,
            "cover while {visibility}"
        );
        assert_eq!(pages.status(), StatusCode::OK, "pages while {visibility}");
        assert_eq!(
            by_number.status(),
            StatusCode::FOUND,
            "page by number while {visibility}"
        );
        assert_eq!(
            image.status(),
            StatusCode::FOUND,
            "page image while {visibility}"
        );
    }
}

#[tokio::test]
async fn a_withheld_resource_is_indistinguishable_from_a_missing_one() {
    let app = TestApp::new().await;
    let book_id = app
        .insert_book_with_visibility(BookVisibility::Hidden)
        .await;
    let cover_id = app.insert_cover(book_id, COVER_PNG).await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;
    let page_id = app.insert_page(release_id, Some(1), COVER_PNG).await;
    let absent = uuid::Uuid::now_v7();

    for (withheld, missing) in [
        (
            format!("/covers/{cover_id}/image"),
            format!("/covers/{absent}/image"),
        ),
        (
            format!("/releases/{release_id}/pages"),
            format!("/releases/{absent}/pages"),
        ),
        (
            format!("/releases/{release_id}/pages/{page_id}/image"),
            format!("/releases/{release_id}/pages/{absent}/image"),
        ),
        (
            format!("/releases/{release_id}/pages/1"),
            format!("/releases/{release_id}/pages/9"),
        ),
    ] {
        let withheld_body = app.get_body(&withheld).await;
        let missing_body = app.get_body(&missing).await;

        assert_eq!(
            withheld_body, missing_body,
            "{withheld} must not reveal that it exists"
        );
    }
}

#[tokio::test]
async fn a_reader_role_does_not_unlock_unlisted_content() {
    let app = TestApp::new().await;
    let book_id = app
        .insert_book_with_visibility(BookVisibility::Hidden)
        .await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;
    app.insert_page(release_id, Some(1), COVER_PNG).await;

    let req = Request::get(format!("/releases/{release_id}/pages"))
        .header(
            header::AUTHORIZATION,
            app.bearer(
                uuid::Uuid::now_v7(),
                uuid::Uuid::now_v7(),
                &[Role::Reader, Role::Uploader],
            ),
        )
        .body(Body::empty())
        .unwrap();
    let resp = app.send_raw(req).await;

    assert_error(resp, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn a_broken_bearer_token_reads_as_anonymous_on_public_routes_and_401s_on_gated_ones() {
    let app = TestApp::new().await;
    let book_id = app
        .insert_book_with_visibility(BookVisibility::Listed)
        .await;
    let cover_id = app.insert_cover(book_id, COVER_PNG).await;
    let chapter_id = app.insert_random_chapter(book_id).await;
    let release_id = app.insert_random_release(book_id, chapter_id).await;
    let page_id = app.insert_page(release_id, Some(1), COVER_PNG).await;

    // A garbage or stale token never grants privilege, but on a public route it
    // must not lock the caller out either -- it reads exactly as anonymous.
    for (path, expected) in [
        (format!("/covers/{cover_id}/image"), StatusCode::FOUND),
        (format!("/releases/{release_id}/pages"), StatusCode::OK),
        (format!("/releases/{release_id}/pages/1"), StatusCode::FOUND),
        (
            format!("/releases/{release_id}/pages/{page_id}/image"),
            StatusCode::FOUND,
        ),
    ] {
        let req = Request::get(&path)
            .header(header::AUTHORIZATION, "Bearer not-a-real-jwt")
            .body(Body::empty())
            .unwrap();

        assert_eq!(
            app.send_raw(req).await.status(),
            expected,
            "{path} with a broken bearer token"
        );
    }

    // A gated route still refuses it.
    let gated = Request::get("/sessions/me")
        .header(header::AUTHORIZATION, "Bearer not-a-real-jwt")
        .body(Body::empty())
        .unwrap();
    assert_eq!(app.send_raw(gated).await.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn update_book_visibility_rejects_an_untabled_move() {
    let app = TestApp::new().await;
    let id = app
        .insert_book_with_visibility(BookVisibility::Listed)
        .await;

    let resp = app
        .put_visibility(id, "PendingReview", Some("because"))
        .await;

    assert_error(resp, StatusCode::CONFLICT).await;
}

#[tokio::test]
async fn update_book_visibility_stores_the_move() {
    let app = TestApp::new().await;
    let id = app.insert_book_with_visibility(BookVisibility::Draft).await;

    let resp = app.put_visibility(id, "Hidden", Some("parked")).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let state = app.fetch_book_visibility_state(id).await;

    assert_eq!(state.visibility, "Hidden");
    assert_eq!(state.note.as_deref(), Some("parked"));
    assert!(state.updated_at.is_some());
}

#[tokio::test]
async fn moves_that_explain_themselves_require_a_note() {
    let app = TestApp::new().await;

    for to in ["Draft", "Rejected", "Hidden"] {
        let id = app
            .insert_book_with_visibility(BookVisibility::PendingReview)
            .await;

        let missing = app.put_visibility(id, to, None).await;
        let blank = app.put_visibility(id, to, Some("   ")).await;

        assert_error(missing, StatusCode::UNPROCESSABLE_ENTITY).await;
        assert_error(blank, StatusCode::UNPROCESSABLE_ENTITY).await;
    }
}

#[tokio::test]
async fn update_book_visibility_with_an_unknown_value_returns_422() {
    let app = TestApp::new().await;
    let id = app
        .insert_book_with_visibility(BookVisibility::Listed)
        .await;

    let resp = app.put_visibility(id, "Published", Some("why")).await;

    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;
}

#[tokio::test]
async fn update_book_visibility_with_an_unknown_id_returns_404() {
    let app = TestApp::new().await;

    let resp = app
        .put_visibility(uuid::Uuid::now_v7(), "Hidden", Some("why"))
        .await;

    assert_error(resp, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn update_book_visibility_with_a_malformed_id_returns_400() {
    let app = TestApp::new().await;

    let req = Request::put("/books/not-a-uuid/visibility")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"visibility":"Hidden","note":"why"}"#))
        .unwrap();
    let resp = app.send(req).await;

    assert_error(resp, StatusCode::BAD_REQUEST).await;
}

#[tokio::test]
async fn book_writes_are_refused_unless_the_book_is_draft_or_listed() {
    let app = TestApp::new().await;

    for visibility in EVERY_VISIBILITY {
        let book: BookQuery = BookFaker {
            visibility,
            ..Default::default()
        }
        .fake();
        app.insert_book(&book).await;

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
        let req = Request::put(format!("/books/{}", book.id))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .unwrap();
        let update = app.send(req).await.status();

        let cover = app.post_cover(book.id, COVER_PNG).await.status();

        let expected = match visibility {
            BookVisibility::Draft | BookVisibility::Listed => {
                (StatusCode::NO_CONTENT, StatusCode::CREATED)
            }
            _ => (StatusCode::CONFLICT, StatusCode::CONFLICT),
        };
        assert_eq!((update, cover), expected, "book writes while {visibility}");
    }
}

#[tokio::test]
async fn cover_writes_are_refused_unless_the_book_is_draft_or_listed() {
    let app = TestApp::new().await;

    for visibility in EVERY_VISIBILITY {
        let book_id = app.insert_book_with_visibility(visibility).await;
        let cover_id = app.insert_cover(book_id, COVER_JPG).await;

        let promote = app.put_main_cover(book_id, cover_id).await;
        let delete = app.delete_cover(cover_id).await.status();

        let expected = match visibility {
            BookVisibility::Draft | BookVisibility::Listed => {
                (StatusCode::NO_CONTENT, StatusCode::NO_CONTENT)
            }
            _ => (StatusCode::CONFLICT, StatusCode::CONFLICT),
        };
        assert_eq!(
            (promote, delete),
            expected,
            "cover writes while {visibility}"
        );
    }
}

#[tokio::test]
async fn chapter_writes_are_refused_unless_the_book_is_listed() {
    let app = TestApp::new().await;

    for visibility in EVERY_VISIBILITY {
        let book_id = app.insert_book_with_visibility(visibility).await;
        let existing = app.insert_random_chapter(book_id).await;

        let created = app.post_chapter(book_id, 1.0).await.status();
        let deleted = app.delete_chapter(existing).await.status();

        let expected = match visibility {
            BookVisibility::Listed => (StatusCode::CREATED, StatusCode::NO_CONTENT),
            _ => (StatusCode::CONFLICT, StatusCode::CONFLICT),
        };
        assert_eq!(
            (created, deleted),
            expected,
            "chapter writes while {visibility}"
        );
    }
}

#[tokio::test]
async fn release_writes_are_refused_unless_the_book_is_listed() {
    let app = TestApp::new().await;

    for visibility in EVERY_VISIBILITY {
        let book_id = app.insert_book_with_visibility(visibility).await;
        let chapter_id = app.insert_random_chapter(book_id).await;
        let release_id = app.insert_random_release(book_id, chapter_id).await;

        let uploaded = app
            .post_upload_pages(release_id, &[COVER_PNG])
            .await
            .status();
        let deleted = app.delete_release(release_id).await.status();

        let expected = match visibility {
            BookVisibility::Listed => (StatusCode::CREATED, StatusCode::NO_CONTENT),
            _ => (StatusCode::CONFLICT, StatusCode::CONFLICT),
        };
        assert_eq!(
            (uploaded, deleted),
            expected,
            "release writes while {visibility}"
        );
    }
}

#[tokio::test]
async fn delete_book_is_allowed_in_every_visibility() {
    let app = TestApp::new().await;

    for visibility in EVERY_VISIBILITY {
        let id = app.insert_book_with_visibility(visibility).await;

        let resp = app.delete_book(id).await;

        assert_eq!(
            resp.status(),
            StatusCode::NO_CONTENT,
            "removal must never be blocked, not even while {visibility}"
        );
    }
}
