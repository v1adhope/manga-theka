use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use fake::Fake;
use http_body_util::BodyExt;
use manga_theka::entity::{BookQuery, BookVisibility, Role};

use crate::helpers::fakers::{BookFaker, COVER_JPG, COVER_PNG, EVERY_VISIBILITY};
use crate::helpers::{RespWrapper, TestApp, assert_error, assert_stored};

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
    });

    let resp = app.post_json("/books", body).await;
    let id = assert_stored(resp).await;

    let state = app.db_fetch_book_visibility_state(id).await;

    assert_eq!(state.visibility, "Draft");
}

#[tokio::test]
async fn get_books_returns_listed_books_only() {
    let app = TestApp::new().await;
    let listed = app
        .db_insert_book_with_visibility(BookVisibility::Listed)
        .await;
    app.db_insert_book_with_visibility(BookVisibility::Draft)
        .await;

    let page = app.get_books_page("/books").await;
    let ids: Vec<uuid::Uuid> = page.data.iter().map(|b| b.id).collect();

    assert_eq!(ids, vec![listed]);
}

#[tokio::test]
async fn get_books_with_a_visibility_overrides_the_default_filter() {
    let app = TestApp::new().await;
    app.db_insert_book_with_visibility(BookVisibility::Listed)
        .await;
    let drafted = app
        .db_insert_book_with_visibility(BookVisibility::Draft)
        .await;

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

    let resp = app.get_raw("/books?visibility=Published").await;

    assert_error(resp, StatusCode::BAD_REQUEST).await;
}

#[tokio::test]
async fn get_book_serves_a_listed_book_to_anyone() {
    let app = TestApp::new().await;
    let id = app
        .db_insert_book_with_visibility(BookVisibility::Listed)
        .await;

    let (status, _) = app.get_body(&format!("/books/{id}")).await;

    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn get_book_reads_a_book_in_every_visibility_for_a_moderator() {
    let app = TestApp::new().await;

    for visibility in EVERY_VISIBILITY {
        let id = app.db_insert_book_with_visibility(visibility).await;

        let resp = app.get_as_moderator(&format!("/books/{id}")).await;

        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "{visibility} must stay linkable for a moderator"
        );
    }
}

#[tokio::test]
async fn get_book_serves_a_hidden_book_to_anyone() {
    let app = TestApp::new().await;
    let id = app
        .db_insert_book_with_visibility(BookVisibility::Hidden)
        .await;

    let (status, _) = app.get_body(&format!("/books/{id}")).await;

    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn get_book_in_a_private_state_is_forbidden_for_non_owners() {
    let app = TestApp::new().await;

    for visibility in [
        BookVisibility::Draft,
        BookVisibility::PendingReview,
        BookVisibility::Rejected,
    ] {
        let id = app.db_insert_book_with_visibility(visibility).await;

        let anon = app.get_body(&format!("/books/{id}")).await;
        let reader = app
            .get_body_as(&format!("/books/{id}"), &[Role::Reader, Role::Uploader])
            .await;

        assert_eq!(anon.0, StatusCode::FORBIDDEN, "anon while {visibility}");
        assert_eq!(reader.0, StatusCode::FORBIDDEN, "reader while {visibility}");
        assert!(
            anon.1.contains(visibility.as_ref()),
            "the {visibility} refusal body must name the blocking state, got {:?}",
            anon.1
        );
    }
}

#[tokio::test]
async fn get_book_in_a_private_state_serves_its_creator() {
    let app = TestApp::new().await;
    let creator = app.db_seed_user(&[Role::Reader]).await;

    for visibility in [
        BookVisibility::Draft,
        BookVisibility::PendingReview,
        BookVisibility::Rejected,
    ] {
        let id = app.db_insert_book_as(visibility, creator.id).await;

        let stray = app
            .get_body_as(&format!("/books/{id}"), &[Role::Reader])
            .await;
        assert_eq!(
            stray.0,
            StatusCode::FORBIDDEN,
            "a stray reader is still refused a {visibility} book"
        );

        let owned = app
            .get_as_user(&format!("/books/{id}"), creator.id, &[Role::Reader])
            .await;
        assert_eq!(
            owned.status(),
            StatusCode::OK,
            "the creator reads their own {visibility} book"
        );
    }
}

#[tokio::test]
async fn get_book_carries_its_review_fields() {
    let app = TestApp::new().await;
    let id = app
        .db_insert_book_with_visibility(BookVisibility::Draft)
        .await;

    let req = Request::get(format!("/books/{id}"))
        .body(Body::empty())
        .unwrap();
    let resp = app.send_authed(req).await;
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(v["data"]["visibility"], "Draft");
    assert!(v["data"]["note"].is_null());
    assert!(v["data"]["submittedAt"].is_null());
}

#[tokio::test]
async fn get_chapter_pages_are_public_only_while_the_book_is_listed() {
    let app = TestApp::new().await;

    for visibility in EVERY_VISIBILITY {
        let book_id = app.db_insert_book_with_visibility(visibility).await;
        let chapter_id = app.db_insert_random_chapter(book_id).await;
        let release_id = app.db_insert_random_release(book_id, chapter_id).await;
        app.fixture_insert_page(release_id, Some(1), COVER_PNG)
            .await;

        let resp = app.get_pages(release_id).await;

        let expected = match visibility {
            BookVisibility::Listed => StatusCode::OK,
            _ => StatusCode::FORBIDDEN,
        };
        assert_eq!(resp.status(), expected, "pages of a {visibility} book");
    }
}

#[tokio::test]
async fn get_chapter_page_is_public_only_while_the_book_is_listed() {
    let app = TestApp::new().await;

    for visibility in EVERY_VISIBILITY {
        let book_id = app.db_insert_book_with_visibility(visibility).await;
        let chapter_id = app.db_insert_random_chapter(book_id).await;
        let release_id = app.db_insert_random_release(book_id, chapter_id).await;
        let page_id = app
            .fixture_insert_page(release_id, Some(1), COVER_PNG)
            .await;

        let number_status = app.get_page(release_id, 1).await.status();
        let image_status = app.get_page_image(release_id, page_id).await.status();

        let expected = match visibility {
            BookVisibility::Listed => StatusCode::FOUND,
            _ => StatusCode::FORBIDDEN,
        };
        assert_eq!(
            number_status, expected,
            "page by number of a {visibility} book"
        );
        assert_eq!(image_status, expected, "page image of a {visibility} book");
    }
}

#[tokio::test]
async fn get_cover_image_follows_the_record_tier() {
    let app = TestApp::new().await;

    for visibility in EVERY_VISIBILITY {
        let book_id = app.db_insert_book_with_visibility(visibility).await;
        let cover_id = app.fixture_insert_cover(book_id, COVER_PNG).await;

        let resp = app.get_cover_image(cover_id).await;

        let expected = if visibility.is_publicly_listable() {
            StatusCode::FOUND
        } else {
            StatusCode::FORBIDDEN
        };
        assert_eq!(resp.status(), expected, "cover of a {visibility} book");
    }
}

#[tokio::test]
async fn a_moderator_reads_content_in_every_visibility() {
    let app = TestApp::new().await;

    for visibility in EVERY_VISIBILITY {
        let book_id = app.db_insert_book_with_visibility(visibility).await;
        let cover_id = app.fixture_insert_cover(book_id, COVER_PNG).await;
        let chapter_id = app.db_insert_random_chapter(book_id).await;
        let release_id = app.db_insert_random_release(book_id, chapter_id).await;
        let page_id = app
            .fixture_insert_page(release_id, Some(1), COVER_PNG)
            .await;

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
async fn a_withheld_page_resource_answers_403_while_a_missing_one_answers_404() {
    let app = TestApp::new().await;
    let book_id = app
        .db_insert_book_with_visibility(BookVisibility::Hidden)
        .await;
    let chapter_id = app.db_insert_random_chapter(book_id).await;
    let release_id = app.db_insert_random_release(book_id, chapter_id).await;
    let page_id = app
        .fixture_insert_page(release_id, Some(1), COVER_PNG)
        .await;
    let absent = uuid::Uuid::now_v7();

    // A hidden book withholds its page content: 403, naming the state.
    for withheld in [
        format!("/releases/{release_id}/pages"),
        format!("/releases/{release_id}/pages/1"),
        format!("/releases/{release_id}/pages/{page_id}/image"),
    ] {
        let (status, body) = app.get_body(&withheld).await;

        assert_eq!(status, StatusCode::FORBIDDEN, "{withheld}");
        assert!(
            body.contains("Hidden"),
            "{withheld} must name the blocking state, got {body:?}"
        );
    }

    // Standing is resolved before sub-resource existence: an absent page id or
    // number behind a withheld release is still 403, not 404.
    for still_withheld in [
        format!("/releases/{release_id}/pages/9"),
        format!("/releases/{release_id}/pages/{absent}/image"),
    ] {
        let status = app.get_raw(&still_withheld).await.status();
        assert_eq!(status, StatusCode::FORBIDDEN, "{still_withheld}");
    }

    // An entirely absent release is a genuine 404.
    let missing = app
        .get_raw(&format!("/releases/{absent}/pages"))
        .await
        .status();
    assert_eq!(missing, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_moderator_reaches_a_missing_page_behind_a_withheld_release_as_404() {
    let app = TestApp::new().await;
    let book_id = app
        .db_insert_book_with_visibility(BookVisibility::Hidden)
        .await;
    let chapter_id = app.db_insert_random_chapter(book_id).await;
    let release_id = app.db_insert_random_release(book_id, chapter_id).await;
    app.fixture_insert_page(release_id, Some(1), COVER_PNG)
        .await;
    let absent = uuid::Uuid::now_v7();

    let by_number = app
        .get_as_moderator(&format!("/releases/{release_id}/pages/9"))
        .await
        .status();
    let by_id = app
        .get_as_moderator(&format!("/releases/{release_id}/pages/{absent}/image"))
        .await
        .status();

    assert_eq!(by_number, StatusCode::NOT_FOUND);
    assert_eq!(by_id, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_reader_role_does_not_unlock_unlisted_content() {
    let app = TestApp::new().await;
    let book_id = app
        .db_insert_book_with_visibility(BookVisibility::Hidden)
        .await;
    let chapter_id = app.db_insert_random_chapter(book_id).await;
    let release_id = app.db_insert_random_release(book_id, chapter_id).await;
    app.fixture_insert_page(release_id, Some(1), COVER_PNG)
        .await;

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

    assert_error(resp, StatusCode::FORBIDDEN).await;
}

#[tokio::test]
async fn a_broken_bearer_token_reads_as_anonymous_on_public_routes_and_401s_on_gated_ones() {
    let app = TestApp::new().await;
    let book_id = app
        .db_insert_book_with_visibility(BookVisibility::Listed)
        .await;
    let cover_id = app.fixture_insert_cover(book_id, COVER_PNG).await;
    let chapter_id = app.db_insert_random_chapter(book_id).await;
    let release_id = app.db_insert_random_release(book_id, chapter_id).await;
    let page_id = app
        .fixture_insert_page(release_id, Some(1), COVER_PNG)
        .await;

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

    let gated = Request::get("/sessions/me")
        .header(header::AUTHORIZATION, "Bearer not-a-real-jwt")
        .body(Body::empty())
        .unwrap();
    assert_eq!(app.send_raw(gated).await.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn update_book_visibility_rejects_an_untabled_move() {
    let app = TestApp::new().await;
    let creator = app.db_seed_user(&[Role::Reader]).await;
    let id = app
        .db_insert_book_as(BookVisibility::Listed, creator.id)
        .await;

    // The creator clears the standing check, so the untabled Listed -> PendingReview
    // move is what the domain guard refuses.
    let resp = app
        .json_as_user(
            axum::http::Method::PUT,
            &format!("/books/{id}/visibility"),
            serde_json::json!({ "visibility": "PendingReview", "note": "because" }),
            creator.id,
            &[Role::Reader],
        )
        .await;

    assert_error(resp, StatusCode::CONFLICT).await;
}

#[tokio::test]
async fn update_book_visibility_stores_the_move() {
    let app = TestApp::new().await;
    let id = app
        .db_insert_book_with_visibility(BookVisibility::Draft)
        .await;

    let resp = app.put_visibility(id, "Hidden", Some("parked")).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let state = app.db_fetch_book_visibility_state(id).await;

    assert_eq!(state.visibility, "Hidden");
    assert_eq!(state.note.as_deref(), Some("parked"));
    assert!(state.updated_at.is_some());
}

#[tokio::test]
async fn moves_that_explain_themselves_require_a_note() {
    let app = TestApp::new().await;

    for to in ["Draft", "Rejected", "Hidden"] {
        let id = app
            .db_insert_book_with_visibility(BookVisibility::PendingReview)
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
        .db_insert_book_with_visibility(BookVisibility::Listed)
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

    let resp = app
        .put_json(
            "/books/not-a-uuid/visibility",
            serde_json::json!({ "visibility": "Hidden", "note": "why" }),
        )
        .await;

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
        app.fixture_insert_book(&book).await;

        let body = serde_json::json!({
            "name": book.name.as_ref(),
            "description": book.description.as_ref(),
            "publicationYear": book.publication_year,
            "contentRatingId": book.content_rating.id,
            "status": book.status.as_ref(),
            "kind": book.kind.as_ref(),
            "publicationLanguageId": book.publication_language.id,
            "publicationDemographic": book.publication_demographic.as_ref(),
        });
        let update = app
            .put_json(&format!("/books/{}", book.id), body)
            .await
            .status();

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
        let book_id = app.db_insert_book_with_visibility(visibility).await;
        let cover_id = app.fixture_insert_cover(book_id, COVER_JPG).await;

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
        let book_id = app.db_insert_book_with_visibility(visibility).await;
        let existing = app.db_insert_random_chapter(book_id).await;

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
        let book_id = app.db_insert_book_with_visibility(visibility).await;
        let chapter_id = app.db_insert_random_chapter(book_id).await;
        let release_id = app.db_insert_random_release(book_id, chapter_id).await;

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
        let id = app.db_insert_book_with_visibility(visibility).await;

        let resp = app.delete_book(id).await;

        assert_eq!(
            resp.status(),
            StatusCode::NO_CONTENT,
            "removal must never be blocked, not even while {visibility}"
        );
    }
}

fn book_body(book: &BookQuery) -> serde_json::Value {
    serde_json::json!({
        "name": book.name.as_ref(),
        "description": book.description.as_ref(),
        "publicationYear": book.publication_year,
        "contentRatingId": book.content_rating.id,
        "status": book.status.as_ref(),
        "kind": book.kind.as_ref(),
        "publicationLanguageId": book.publication_language.id,
        "publicationDemographic": book.publication_demographic.as_ref(),
    })
}

#[tokio::test]
async fn get_books_with_visibility_hidden_is_public() {
    let app = TestApp::new().await;
    let hidden = app
        .db_insert_book_with_visibility(BookVisibility::Hidden)
        .await;
    app.db_insert_book_with_visibility(BookVisibility::Listed)
        .await;

    let page = app.get_books_page("/books?visibility=Hidden").await;
    let ids: Vec<uuid::Uuid> = page.data.iter().map(|b| b.id).collect();

    assert_eq!(ids, vec![hidden]);
}

#[tokio::test]
async fn get_books_with_a_restricted_visibility_is_forbidden_for_a_guest() {
    let app = TestApp::new().await;

    for visibility in ["Draft", "PendingReview", "Rejected"] {
        let resp = app
            .get_raw(&format!("/books?visibility={visibility}"))
            .await;

        assert_error(resp, StatusCode::FORBIDDEN).await;
    }
}

#[tokio::test]
async fn the_book_creator_reads_the_record_and_its_children_in_a_private_state() {
    let app = TestApp::new().await;
    let book_creator = app.db_seed_user(&[Role::Reader]).await;
    let release_creator = app.db_seed_user(&[Role::Uploader]).await;

    let book_id = app
        .db_insert_book_as(BookVisibility::Draft, book_creator.id)
        .await;
    let cover_id = app.fixture_insert_cover(book_id, COVER_PNG).await;
    let chapter_id = app.db_insert_random_chapter(book_id).await;
    let language_id = app.db_non_publication_language(book_id).await;
    let release_id = app
        .db_insert_release_as(chapter_id, language_id, release_creator.id)
        .await;

    let cid = book_creator.id;
    let readable = [
        (format!("/books/{book_id}"), StatusCode::OK),
        (format!("/books/{book_id}/covers"), StatusCode::OK),
        (format!("/covers/{cover_id}/image"), StatusCode::FOUND),
        (format!("/books/{book_id}/chapters"), StatusCode::OK),
        (format!("/chapters/{chapter_id}"), StatusCode::OK),
        (format!("/chapters/{chapter_id}/releases"), StatusCode::OK),
        // ...but not a release they did not create, nor its page content.
        (format!("/releases/{release_id}"), StatusCode::FORBIDDEN),
        (
            format!("/releases/{release_id}/pages"),
            StatusCode::FORBIDDEN,
        ),
    ];

    for (path, expected) in readable {
        let status = app.get_as_user(&path, cid, &[Role::Reader]).await.status();
        assert_eq!(status, expected, "book creator on {path}");
    }
}

#[tokio::test]
async fn the_release_creator_reads_their_release_and_its_pages_while_hidden() {
    let app = TestApp::new().await;
    let release_creator = app.db_seed_user(&[Role::Uploader]).await;

    let book_id = app
        .db_insert_book_with_visibility(BookVisibility::Hidden)
        .await;
    let chapter_id = app.db_insert_random_chapter(book_id).await;
    let language_id = app.db_non_publication_language(book_id).await;
    let release_id = app
        .db_insert_release_as(chapter_id, language_id, release_creator.id)
        .await;
    let page_id = app
        .fixture_insert_page(release_id, Some(1), COVER_PNG)
        .await;

    let record = app
        .get_as_user(
            &format!("/releases/{release_id}"),
            release_creator.id,
            &[Role::Uploader],
        )
        .await
        .status();
    let pages = app
        .get_as_user(
            &format!("/releases/{release_id}/pages"),
            release_creator.id,
            &[Role::Uploader],
        )
        .await
        .status();
    let image = app
        .get_as_user(
            &format!("/releases/{release_id}/pages/{page_id}/image"),
            release_creator.id,
            &[Role::Uploader],
        )
        .await
        .status();
    let staged = app
        .get_as_user(
            &format!("/releases/{release_id}/pages?status=Staged"),
            release_creator.id,
            &[Role::Uploader],
        )
        .await
        .status();

    assert_eq!(record, StatusCode::OK);
    assert_eq!(pages, StatusCode::OK);
    assert_eq!(image, StatusCode::FOUND);
    assert_eq!(staged, StatusCode::OK);
}

#[tokio::test]
async fn the_staged_page_list_is_forbidden_to_a_guest_even_while_listed() {
    let app = TestApp::new().await;
    let book_id = app
        .db_insert_book_with_visibility(BookVisibility::Listed)
        .await;
    let chapter_id = app.db_insert_random_chapter(book_id).await;
    let release_id = app.db_insert_random_release(book_id, chapter_id).await;
    app.fixture_insert_page(release_id, None, COVER_PNG).await;

    let resp = app.get_staged(release_id).await;

    assert_error(resp, StatusCode::FORBIDDEN).await;
}

#[tokio::test]
async fn submitting_a_book_for_review_is_the_creators_alone() {
    let app = TestApp::new().await;
    let creator = app.db_seed_user(&[Role::Reader]).await;
    let book_id = app
        .db_insert_book_as(BookVisibility::Draft, creator.id)
        .await;

    let by_stranger = app
        .json_as_user(
            axum::http::Method::PUT,
            &format!("/books/{book_id}/visibility"),
            serde_json::json!({ "visibility": "PendingReview" }),
            uuid::Uuid::now_v7(),
            &[Role::Moderator],
        )
        .await;
    assert_error(by_stranger, StatusCode::FORBIDDEN).await;

    let by_creator = app
        .json_as_user(
            axum::http::Method::PUT,
            &format!("/books/{book_id}/visibility"),
            serde_json::json!({ "visibility": "PendingReview" }),
            creator.id,
            &[Role::Reader],
        )
        .await;
    assert_eq!(by_creator.status(), StatusCode::NO_CONTENT);

    let state = app.db_fetch_book_visibility_state(book_id).await;
    assert_eq!(state.visibility, "PendingReview");
}

#[tokio::test]
async fn a_draft_book_edit_is_refused_for_an_uploader_who_is_not_its_creator() {
    let app = TestApp::new().await;
    let creator = app.db_seed_user(&[Role::Uploader]).await;
    let book: BookQuery = BookFaker {
        visibility: BookVisibility::Draft,
        created_by: Some(creator.id),
        ..Default::default()
    }
    .fake();
    app.db_insert_book(&book).await;

    let by_stranger = app
        .json_as_user(
            axum::http::Method::PUT,
            &format!("/books/{}", book.id),
            book_body(&book),
            uuid::Uuid::now_v7(),
            &[Role::Uploader],
        )
        .await;
    assert_error(by_stranger, StatusCode::FORBIDDEN).await;

    let by_creator = app
        .json_as_user(
            axum::http::Method::PUT,
            &format!("/books/{}", book.id),
            book_body(&book),
            creator.id,
            &[Role::Uploader],
        )
        .await;
    assert_eq!(by_creator.status(), StatusCode::NO_CONTENT);
}
