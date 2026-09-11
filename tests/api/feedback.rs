use axum::http::StatusCode;

use crate::helpers::fakers::FeedbackFaker;
use crate::helpers::{RespWrapper, TestApp, assert_error, assert_stored};
use fake::Fake;
use manga_theka::entity::{Feedback, FeedbackKind, FeedbackStatus, Role, Timestamp};

fn general_body() -> serde_json::Value {
    serde_json::json!({
        "kind": "General",
        "email": "reader@example.com",
        "note": "The search page is broken on mobile.",
    })
}

#[tokio::test]
async fn store_general_feedback_with_valid_body_passes() {
    let app = TestApp::new().await;

    let resp = app.post_feedback(general_body()).await;
    let id = assert_stored(resp).await;

    let stored = app.db_fetch_feedback(id).await;

    assert_eq!(stored.kind, FeedbackKind::General);
    assert_eq!(stored.status, FeedbackStatus::Open);
    assert_eq!(stored.email.as_ref(), "reader@example.com");
    assert_eq!(stored.book_id, None);
    assert_eq!(stored.updated_at, None);
    assert_ne!(stored.created_at, Timestamp::UNIX_EPOCH);
}

#[tokio::test]
async fn store_book_bound_feedback_with_valid_body_passes() {
    let app = TestApp::new().await;
    let book_id = app.db_insert_random_book().await;

    for (kind, expected) in [
        ("Report", FeedbackKind::Report),
        ("Correction", FeedbackKind::Correction),
    ] {
        let resp = app
            .post_feedback(serde_json::json!({
                "kind": kind,
                "email": "reader@example.com",
                "note": "This one breaks a rule.",
                "bookId": book_id,
            }))
            .await;
        let id = assert_stored(resp).await;

        let stored = app.db_fetch_feedback(id).await;

        assert_eq!(stored.kind, expected);
        assert_eq!(stored.book_id, Some(book_id));
        assert_eq!(stored.status, FeedbackStatus::Open);
    }
}

#[tokio::test]
async fn store_book_bound_feedback_without_a_book_returns_422() {
    let app = TestApp::new().await;

    for kind in ["Report", "Correction"] {
        let resp = app
            .post_feedback(serde_json::json!({
                "kind": kind,
                "email": "reader@example.com",
                "note": "No book named.",
            }))
            .await;

        assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;
    }

    assert_eq!(
        app.db_count_feedback().await,
        0,
        "failed write must roll back"
    );
}

#[tokio::test]
async fn store_feedback_with_unknown_book_returns_422() {
    let app = TestApp::new().await;

    let resp = app
        .post_feedback(serde_json::json!({
            "kind": "Report",
            "email": "reader@example.com",
            "note": "This one breaks a rule.",
            "bookId": uuid::Uuid::now_v7(),
        }))
        .await;

    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;
    assert_eq!(
        app.db_count_feedback().await,
        0,
        "failed write must roll back"
    );
}

#[tokio::test]
async fn store_general_feedback_naming_a_book_returns_422() {
    let app = TestApp::new().await;
    let book_id = app.db_insert_random_book().await;

    let resp = app
        .post_feedback(serde_json::json!({
            "kind": "General",
            "email": "reader@example.com",
            "note": "Site-wide, but naming a book anyway.",
            "bookId": book_id,
        }))
        .await;

    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;
    assert_eq!(
        app.db_count_feedback().await,
        0,
        "failed write must roll back"
    );
}

#[tokio::test]
async fn store_feedback_with_unknown_kind_returns_422() {
    let app = TestApp::new().await;

    let resp = app
        .post_feedback(serde_json::json!({
            "kind": "Complaint",
            "email": "reader@example.com",
            "note": "Wrong kind.",
        }))
        .await;

    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;
}

#[tokio::test]
async fn store_feedback_with_invalid_field_values_returns_422() {
    let app = TestApp::new().await;
    let long_note = "e".repeat(2001);
    let long_email = format!("{}@b.co", "a".repeat(250));

    for (label, email, note) in [
        ("malformed email", "not-an-address", "A real note."),
        ("empty note", "reader@example.com", "   "),
        ("oversized note", "reader@example.com", long_note.as_str()),
        ("oversized email", long_email.as_str(), "A real note."),
    ] {
        let resp = app
            .post_feedback(serde_json::json!({
                "kind": "General",
                "email": email,
                "note": note,
            }))
            .await;

        assert_eq!(
            resp.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "{label} must be rejected"
        );
    }

    assert_eq!(
        app.db_count_feedback().await,
        0,
        "failed write must roll back"
    );
}

#[tokio::test]
async fn store_feedback_with_a_malformed_book_id_returns_422() {
    let app = TestApp::new().await;

    let resp = app
        .post_feedback(serde_json::json!({
            "kind": "Report",
            "email": "reader@example.com",
            "note": "This one breaks a rule.",
            "bookId": "not-a-uuid",
        }))
        .await;

    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;
}

#[tokio::test]
async fn store_feedback_with_broken_json_returns_400() {
    let app = TestApp::new().await;

    let resp = app.post_raw("/feedbacks", "{not json").await;

    assert_error(resp, StatusCode::BAD_REQUEST).await;
}

#[tokio::test]
async fn get_feedbacks_filters_by_kind() {
    let app = TestApp::new().await;
    let book_id = app.db_insert_random_book().await;

    for kind in [
        FeedbackKind::Report,
        FeedbackKind::Correction,
        FeedbackKind::General,
    ] {
        let book_id = match kind {
            FeedbackKind::General => None,
            _ => Some(book_id),
        };
        let feedback: Feedback = FeedbackFaker {
            kind,
            book_id,
            ..Default::default()
        }
        .fake();
        app.db_insert_feedback(&feedback).await;
    }

    let page: RespWrapper<Vec<Feedback>> = app
        .get_ok_json_as("/feedbacks?kind=Report", &[Role::Moderator])
        .await;

    assert_eq!(page.data.len(), 1);
    assert_eq!(page.data[0].kind, FeedbackKind::Report);
}

#[tokio::test]
async fn get_feedbacks_filters_by_status() {
    let app = TestApp::new().await;

    for status in [
        FeedbackStatus::Open,
        FeedbackStatus::Resolved,
        FeedbackStatus::Dismissed,
    ] {
        let feedback: Feedback = FeedbackFaker {
            status,
            ..Default::default()
        }
        .fake();
        app.db_insert_feedback(&feedback).await;
    }

    let page: RespWrapper<Vec<Feedback>> = app
        .get_ok_json_as("/feedbacks?status=Dismissed", &[Role::Moderator])
        .await;

    assert_eq!(page.data.len(), 1);
    assert_eq!(page.data[0].status, FeedbackStatus::Dismissed);
}

#[tokio::test]
async fn get_feedbacks_combines_kind_and_status_filters() {
    let app = TestApp::new().await;
    let book_id = app.db_insert_random_book().await;

    for (kind, status, book) in [
        (FeedbackKind::Report, FeedbackStatus::Open, Some(book_id)),
        (
            FeedbackKind::Report,
            FeedbackStatus::Resolved,
            Some(book_id),
        ),
        (FeedbackKind::General, FeedbackStatus::Open, None),
    ] {
        let feedback: Feedback = FeedbackFaker {
            kind,
            status,
            book_id: book,
        }
        .fake();
        app.db_insert_feedback(&feedback).await;
    }

    let page: RespWrapper<Vec<Feedback>> = app
        .get_ok_json_as("/feedbacks?kind=Report&status=Open", &[Role::Moderator])
        .await;

    assert_eq!(page.data.len(), 1);
    assert_eq!(page.data[0].kind, FeedbackKind::Report);
    assert_eq!(page.data[0].status, FeedbackStatus::Open);
}

#[tokio::test]
async fn get_feedbacks_orders_by_submission_date() {
    let app = TestApp::new().await;
    let mut ids = Vec::new();

    for _ in 0..4 {
        let feedback: Feedback = FeedbackFaker::default().fake();
        ids.push(feedback.id);
        app.db_insert_feedback(&feedback).await;
    }

    let asc: RespWrapper<Vec<Feedback>> = app
        .get_ok_json_as("/feedbacks?order=Asc", &[Role::Moderator])
        .await;
    let asc_ids: Vec<uuid::Uuid> = asc.data.iter().map(|f| f.id).collect();

    let desc: RespWrapper<Vec<Feedback>> = app
        .get_ok_json_as("/feedbacks?order=Desc", &[Role::Moderator])
        .await;
    let desc_ids: Vec<uuid::Uuid> = desc.data.iter().map(|f| f.id).collect();

    ids.sort();
    assert_eq!(asc_ids, ids);

    ids.reverse();
    assert_eq!(desc_ids, ids);
}

#[tokio::test]
async fn get_feedbacks_returns_default_limit_and_next_cursor() {
    let app = TestApp::new().await;

    for _ in 0..25 {
        let feedback: Feedback = FeedbackFaker::default().fake();
        app.db_insert_feedback(&feedback).await;
    }

    let page: RespWrapper<Vec<Feedback>> =
        app.get_ok_json_as("/feedbacks", &[Role::Moderator]).await;

    assert_eq!(page.data.len(), 20);
    assert!(page.next_cursor.is_some());
}

#[tokio::test]
async fn get_feedbacks_with_after_and_limit_3_returns_next_page() {
    let app = TestApp::new().await;

    for _ in 0..6 {
        let feedback: Feedback = FeedbackFaker::default().fake();
        app.db_insert_feedback(&feedback).await;
    }

    let first: RespWrapper<Vec<Feedback>> = app
        .get_ok_json_as("/feedbacks?limit=3", &[Role::Moderator])
        .await;

    let ids: Vec<uuid::Uuid> = first.data.iter().map(|f| f.id).collect();
    let cursor = first.next_cursor.unwrap();

    let second: RespWrapper<Vec<Feedback>> = app
        .get_ok_json_as(
            &format!("/feedbacks?limit=3&after={cursor}"),
            &[Role::Moderator],
        )
        .await;

    let second_ids: Vec<uuid::Uuid> = second.data.iter().map(|f| f.id).collect();

    assert_eq!(second_ids.len(), 3);
    assert!(second_ids.iter().all(|id| !ids.contains(id)));
    assert!(second.next_cursor.is_none());
}

#[tokio::test]
async fn get_feedbacks_zero_limit_returns_422() {
    let app = TestApp::new().await;

    let resp = app.get_feedbacks("/feedbacks?limit=0").await;

    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;
}

#[tokio::test]
async fn get_feedbacks_invalid_after_returns_400() {
    let app = TestApp::new().await;

    let resp = app.get_feedbacks("/feedbacks?after=not-a-uuid").await;

    assert_error(resp, StatusCode::BAD_REQUEST).await;
}

#[tokio::test]
async fn get_feedbacks_unknown_filter_values_return_400() {
    let app = TestApp::new().await;

    for query in ["/feedbacks?kind=Complaint", "/feedbacks?status=Closed"] {
        let resp = app.get_feedbacks(query).await;

        assert_error(resp, StatusCode::BAD_REQUEST).await;
    }
}

#[tokio::test]
async fn get_feedback_with_valid_id_passes() {
    let app = TestApp::new().await;

    for status in [FeedbackStatus::Open, FeedbackStatus::Dismissed] {
        let feedback: Feedback = FeedbackFaker {
            status,
            ..Default::default()
        }
        .fake();
        app.db_insert_feedback(&feedback).await;

        let got: RespWrapper<Feedback> = app
            .get_ok_json_as(&format!("/feedbacks/{}", feedback.id), &[Role::Moderator])
            .await;

        assert_eq!(got.data.id, feedback.id);
        assert_eq!(got.data.email.as_ref(), feedback.email.as_ref());
        assert_eq!(got.data.note.as_ref(), feedback.note.as_ref());
    }
}

#[tokio::test]
async fn get_feedback_with_unknown_id_returns_404() {
    let app = TestApp::new().await;

    let resp = app.get_feedback(uuid::Uuid::now_v7()).await;

    assert_error(resp, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn get_feedback_with_malformed_id_returns_400() {
    let app = TestApp::new().await;

    let resp = app.get_feedbacks("/feedbacks/not-a-uuid").await;

    assert_error(resp, StatusCode::BAD_REQUEST).await;
}

#[tokio::test]
async fn update_feedback_status_closes_it_and_stamps_updated_at() {
    let app = TestApp::new().await;

    for target in ["Resolved", "Dismissed"] {
        let feedback: Feedback = FeedbackFaker::default().fake();
        app.db_insert_feedback(&feedback).await;

        let status = app.put_feedback_status(feedback.id, target).await;
        assert_eq!(status, StatusCode::NO_CONTENT);

        let stored = app.db_fetch_feedback(feedback.id).await;

        assert_eq!(stored.status.as_ref(), target);
        assert!(
            stored.updated_at.is_some(),
            "{target} must stamp updated_at"
        );
    }
}

#[tokio::test]
async fn update_feedback_status_can_reopen_a_closed_one() {
    let app = TestApp::new().await;
    let feedback: Feedback = FeedbackFaker {
        status: FeedbackStatus::Resolved,
        ..Default::default()
    }
    .fake();
    app.db_insert_feedback(&feedback).await;

    let status = app.put_feedback_status(feedback.id, "Open").await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let stored = app.db_fetch_feedback(feedback.id).await;

    assert_eq!(stored.status, FeedbackStatus::Open);
}

#[tokio::test]
async fn update_feedback_status_to_its_current_value_is_idempotent() {
    let app = TestApp::new().await;
    let feedback: Feedback = FeedbackFaker::default().fake();
    app.db_insert_feedback(&feedback).await;

    let first = app.put_feedback_status(feedback.id, "Resolved").await;
    let second = app.put_feedback_status(feedback.id, "Resolved").await;

    let stored = app.db_fetch_feedback(feedback.id).await;

    assert_eq!(first, StatusCode::NO_CONTENT);
    assert_eq!(second, StatusCode::NO_CONTENT);
    assert_eq!(stored.status, FeedbackStatus::Resolved);
}

#[tokio::test]
async fn update_feedback_status_with_unknown_status_returns_422() {
    let app = TestApp::new().await;
    let feedback: Feedback = FeedbackFaker::default().fake();
    app.db_insert_feedback(&feedback).await;

    let status = app.put_feedback_status(feedback.id, "Closed").await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn update_feedback_status_with_unknown_id_returns_404() {
    let app = TestApp::new().await;

    let status = app
        .put_feedback_status(uuid::Uuid::now_v7(), "Resolved")
        .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn deleting_a_book_keeps_its_feedback_and_nulls_the_book_id() {
    let app = TestApp::new().await;
    let book_id = app.db_insert_random_book().await;
    let feedback: Feedback = FeedbackFaker {
        kind: FeedbackKind::Report,
        book_id: Some(book_id),
        ..Default::default()
    }
    .fake();
    app.db_insert_feedback(&feedback).await;

    let resp = app.delete_book(book_id).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let stored = app.db_fetch_feedback(feedback.id).await;

    assert_eq!(stored.kind, FeedbackKind::Report);
    assert_eq!(stored.book_id, None);
    assert_eq!(app.db_count_feedback().await, 1);
}

#[tokio::test]
async fn feedback_has_no_delete_route() {
    let app = TestApp::new().await;
    let feedback: Feedback = FeedbackFaker::default().fake();
    app.db_insert_feedback(&feedback).await;

    let resp = app
        .delete_authed(&format!("/feedbacks/{}", feedback.id))
        .await;

    assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(app.db_count_feedback().await, 1);
}
