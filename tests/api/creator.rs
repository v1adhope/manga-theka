use axum::http::StatusCode;
use time::OffsetDateTime;

use crate::helpers::fakers::CreatorFaker;
use crate::helpers::{RespWrapper, TestApp, assert_error, assert_stored};
use fake::Fake;
use manga_theka::entity::{Creator, CreatorQuery, CreatorRole};

#[tokio::test]
async fn store_creator_with_valid_body_passes() {
    let app = TestApp::new().await;

    let resp = app
        .post_json(
            "/creators",
            serde_json::json!({ "firstName": "John", "lastName": "Doe" }),
        )
        .await;
    let id = assert_stored(resp).await;

    let row = sqlx::query!("select id, first_name, last_name, created_at from creators",)
        .fetch_one(&app.pool)
        .await
        .unwrap();

    assert_eq!(row.id, id);
    assert_eq!(row.first_name, "John");
    assert_eq!(row.last_name, "Doe");
    assert_ne!(row.created_at, OffsetDateTime::UNIX_EPOCH)
}

#[tokio::test]
async fn store_creator_with_broken_json_returns_400() {
    let app = TestApp::new().await;

    let resp = app.post_raw("/creators", "{not json").await;

    assert_error(resp, StatusCode::BAD_REQUEST).await;
}

#[tokio::test]
async fn store_creator_with_missing_first_name_returns_422() {
    let app = TestApp::new().await;

    let resp = app
        .post_json("/creators", serde_json::json!({ "lastName": "Doe" }))
        .await;

    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;
}

#[tokio::test]
async fn update_creator_with_valid_body_passes() {
    let app = TestApp::new().await;
    let creator = CreatorFaker.fake();

    app.db_insert_creator(&creator).await;

    let resp = app
        .put_json(
            &format!("/creators/{}", creator.id),
            serde_json::json!({ "firstName": "Updated", "lastName": "Name" }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let row = sqlx::query!("select first_name, last_name, created_at from creators")
        .fetch_one(&app.pool)
        .await
        .unwrap();

    assert_eq!(row.first_name, "Updated");
    assert_eq!(row.last_name, "Name");
    assert_eq!(
        row.created_at.unix_timestamp(),
        creator.created_at.unix_timestamp()
    );
}

#[tokio::test]
async fn update_creator_leaves_other_creators_untouched() {
    let app = TestApp::new().await;
    let creator = CreatorFaker.fake();
    let other_creator = CreatorFaker.fake();

    app.db_insert_creator(&creator).await;
    app.db_insert_creator(&other_creator).await;

    let resp = app
        .put_json(
            &format!("/creators/{}", creator.id),
            serde_json::json!({ "firstName": "Updated", "lastName": "Name" }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let row = sqlx::query!(
        "select first_name, last_name, created_at from creators where id = $1",
        other_creator.id,
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();

    assert_eq!(row.first_name, other_creator.first_name.as_ref());
    assert_eq!(row.last_name, other_creator.last_name.as_ref());
    assert_eq!(
        row.created_at.unix_timestamp(),
        other_creator.created_at.unix_timestamp()
    );
}

#[tokio::test]
async fn update_creator_with_unknown_id_returns_404() {
    let app = TestApp::new().await;

    let resp = app
        .put_json(
            &format!("/creators/{}", uuid::Uuid::now_v7()),
            serde_json::json!({ "firstName": "Updated", "lastName": "Name" }),
        )
        .await;
    assert_error(resp, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn update_creator_fullname_duplication_returns_409() {
    let app = TestApp::new().await;
    let creator = CreatorFaker.fake();
    let another_creator = CreatorFaker.fake();

    app.db_insert_creator(&creator).await;
    app.db_insert_creator(&another_creator).await;

    let resp = app
        .put_json(
            &format!("/creators/{}", another_creator.id),
            serde_json::json!({
                "firstName": creator.first_name.as_ref(),
                "lastName": creator.last_name.as_ref(),
            }),
        )
        .await;
    assert_error(resp, StatusCode::CONFLICT).await;
}

#[tokio::test]
async fn update_creator_name_validation_failure_returns_422() {
    let app = TestApp::new().await;
    let creator = CreatorFaker.fake();

    app.db_insert_creator(&creator).await;

    let resp = app
        .put_json(
            &format!("/creators/{}", creator.id),
            serde_json::json!({ "firstName": "John123", "lastName": "Doe" }),
        )
        .await;
    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;
}

#[tokio::test]
async fn get_creator_with_valid_id_passes() {
    let app = TestApp::new().await;
    let creator = CreatorFaker.fake();

    app.db_insert_creator(&creator).await;

    let got = app
        .get_ok_json::<RespWrapper<CreatorQuery>>(&format!("/creators/{}", creator.id))
        .await
        .data;

    assert_eq!(got.id, creator.id);
    assert_eq!(got.first_name, creator.first_name);
    assert_eq!(got.last_name, creator.last_name);
    assert!(got.roles.is_empty());
    assert_eq!(
        got.created_at.unix_timestamp(),
        creator.created_at.unix_timestamp()
    );
}

#[tokio::test]
async fn get_creator_with_unknown_id_returns_404() {
    let app = TestApp::new().await;
    let resp = app.get_creator(uuid::Uuid::now_v7()).await;
    assert_error(resp, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn get_creator_returns_distinct_roles_credited_across_books() {
    let app = TestApp::new().await;
    let creator = CreatorFaker.fake();
    app.db_insert_creator(&creator).await;

    let book_a = app.db_insert_random_book().await;
    let book_b = app.db_insert_random_book().await;
    let book_c = app.db_insert_random_book().await;

    app.db_credit_creator(book_a, creator.id, CreatorRole::Author)
        .await;
    app.db_credit_creator(book_b, creator.id, CreatorRole::Artist)
        .await;
    app.db_credit_creator(book_c, creator.id, CreatorRole::Author)
        .await;

    let data = app
        .get_ok_json::<RespWrapper<CreatorQuery>>(&format!("/creators/{}", creator.id))
        .await
        .data;

    let mut roles: Vec<&str> = data.roles.iter().map(AsRef::as_ref).collect();
    roles.sort();
    assert_eq!(roles, vec!["Artist", "Author"]);
}

#[tokio::test]
async fn get_creators_returns_default_limit_and_next_cursor() {
    let app = TestApp::new().await;

    for _ in 0..25 {
        let creator = CreatorFaker.fake();
        app.db_insert_creator(&creator).await;
    }

    let page = app
        .get_ok_json::<RespWrapper<Vec<CreatorQuery>>>("/creators")
        .await;

    assert_eq!(page.data.len(), 20);
    assert!(page.next_cursor.is_some());
}

#[tokio::test]
async fn get_creators_with_after_and_limit_3_returns_next_page() {
    let app = TestApp::new().await;

    for _ in 0..6 {
        let creator: Creator = CreatorFaker.fake();
        app.db_insert_creator(&creator).await;
    }

    let first = app
        .get_ok_json::<RespWrapper<Vec<CreatorQuery>>>("/creators?limit=3")
        .await;

    let ids: Vec<uuid::Uuid> = first.data.iter().map(|c| c.id).collect();
    let cursor = first.next_cursor.unwrap();

    let second = app
        .get_ok_json::<RespWrapper<Vec<CreatorQuery>>>(&format!("/creators?limit=3&after={cursor}"))
        .await;

    let second_ids: Vec<uuid::Uuid> = second.data.iter().map(|c| c.id).collect();

    assert_eq!(second_ids.len(), 3);
    assert!(second_ids.iter().all(|id| !ids.contains(id)));
    assert!(second.next_cursor.is_none());
}

#[tokio::test]
async fn get_creators_desc_order_confirmed() {
    let app = TestApp::new().await;

    let mut ids = Vec::with_capacity(3);
    for _ in 0..3 {
        let creator: Creator = CreatorFaker.fake();
        ids.push(creator.id);
        app.db_insert_creator(&creator).await;
    }
    ids.sort_by(|a, b| b.cmp(a));

    let page = app
        .get_ok_json::<RespWrapper<Vec<CreatorQuery>>>("/creators")
        .await;
    let returned_ids: Vec<uuid::Uuid> = page.data.iter().map(|c| c.id).collect();
    assert_eq!(returned_ids, ids);
}

#[tokio::test]
async fn get_creators_next_cursor_null_on_last_page() {
    let app = TestApp::new().await;

    for _ in 0..10 {
        app.db_insert_creator(&CreatorFaker.fake()).await;
    }

    let v = app.get_ok_json::<serde_json::Value>("/creators").await;
    assert!(v["nextCursor"].is_null());
}

#[tokio::test]
async fn get_creators_invalid_limit_returns_400() {
    let app = TestApp::new().await;

    let resp = app.get_raw("/creators?limit=abc").await;
    assert_error(resp, StatusCode::BAD_REQUEST).await;
}

#[tokio::test]
async fn get_creators_zero_limit_returns_422() {
    let app = TestApp::new().await;

    let resp = app.get_raw("/creators?limit=0").await;
    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;
}

#[tokio::test]
async fn get_creators_invalid_after_returns_400() {
    let app = TestApp::new().await;

    let resp = app.get_raw("/creators?after=not-a-uuid").await;
    assert_error(resp, StatusCode::BAD_REQUEST).await;
}

#[tokio::test]
async fn delete_creator_with_valid_id_passes() {
    let app = TestApp::new().await;
    let creator = CreatorFaker.fake();

    app.db_insert_creator(&creator).await;

    let resp = app.delete_creator(creator.id).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let count = sqlx::query_scalar!("select count(*) from creators")
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(count, Some(0));
}

#[tokio::test]
async fn delete_creator_with_unknown_id_returns_404() {
    let app = TestApp::new().await;

    let resp = app.delete_creator(uuid::Uuid::now_v7()).await;
    assert_error(resp, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn delete_creator_leaves_other_creators_untouched() {
    let app = TestApp::new().await;
    let creator = CreatorFaker.fake();
    let other_creator = CreatorFaker.fake();

    app.db_insert_creator(&creator).await;
    app.db_insert_creator(&other_creator).await;

    let resp = app.delete_creator(creator.id).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let row = sqlx::query!(
        "select first_name, last_name, created_at from creators where id = $1",
        other_creator.id,
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();

    assert_eq!(row.first_name, other_creator.first_name.as_ref());
    assert_eq!(row.last_name, other_creator.last_name.as_ref());
    assert_eq!(
        row.created_at.unix_timestamp(),
        other_creator.created_at.unix_timestamp()
    );
}
