use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use http_body_util::BodyExt;
use time::OffsetDateTime;

use crate::helpers::fakers::CreatorFaker;
use crate::helpers::{RespWrapper, TestApp, assert_error, assert_stored};
use fake::Fake;
use manga_theka::entity::{Creator, CreatorQuery, CreatorRole};

#[tokio::test]
async fn store_creator_with_valid_body_passes() {
    let app = TestApp::new().await;
    let body = serde_json::json!({
        "firstName": "John",
        "lastName": "Doe",
    })
    .to_string();
    let req = Request::post("/creators")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.send(req).await;
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
    let req = Request::post("/creators")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{not json"))
        .unwrap();

    let resp = app.send(req).await;

    assert_error(resp, StatusCode::BAD_REQUEST).await;
}

#[tokio::test]
async fn store_creator_with_missing_first_name_returns_422() {
    let app = TestApp::new().await;
    let body = serde_json::json!({
        "lastName": "Doe",
    })
    .to_string();
    let req = Request::post("/creators")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.send(req).await;

    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;
}

#[tokio::test]
async fn update_creator_with_valid_body_passes() {
    let app = TestApp::new().await;
    let creator = CreatorFaker.fake();

    app.insert_creator(&creator).await;

    let body = serde_json::json!({
        "firstName": "Updated",
        "lastName": "Name",
    })
    .to_string();

    let req = Request::put(format!("/creators/{}", creator.id))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.send(req).await;
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

    app.insert_creator(&creator).await;
    app.insert_creator(&other_creator).await;

    let body = serde_json::json!({
        "firstName": "Updated",
        "lastName": "Name",
    })
    .to_string();
    let req = Request::put(format!("/creators/{}", creator.id))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.send(req).await;
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

    let body = serde_json::json!({
        "firstName": "Updated",
        "lastName": "Name",
    })
    .to_string();
    let req = Request::put(format!("/creators/{}", uuid::Uuid::now_v7()))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.send(req).await;
    assert_error(resp, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn update_creator_fullname_duplication_returns_409() {
    let app = TestApp::new().await;
    let creator = CreatorFaker.fake();
    let another_creator = CreatorFaker.fake();

    app.insert_creator(&creator).await;
    app.insert_creator(&another_creator).await;

    let body = serde_json::json!({
        "firstName": creator.first_name.as_ref(),
        "lastName": creator.last_name.as_ref(),
    })
    .to_string();
    let req = Request::put(format!("/creators/{}", another_creator.id))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.send(req).await;
    assert_error(resp, StatusCode::CONFLICT).await;
}

#[tokio::test]
async fn update_creator_name_validation_failure_returns_422() {
    let app = TestApp::new().await;
    let creator = CreatorFaker.fake();

    app.insert_creator(&creator).await;

    let body = serde_json::json!({
        "firstName": "John123",
        "lastName": "Doe",
    })
    .to_string();

    let req = Request::put(format!("/creators/{}", creator.id))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.send(req).await;
    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;
}

#[tokio::test]
async fn get_creator_with_valid_id_passes() {
    let app = TestApp::new().await;
    let creator = CreatorFaker.fake();

    app.insert_creator(&creator).await;

    let resp = app.get_creator(creator.id).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let wrapper: RespWrapper<CreatorQuery> = serde_json::from_slice(&bytes).unwrap();
    let got = wrapper.data;

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
    app.insert_creator(&creator).await;

    let book_a = app.insert_random_book().await;
    let book_b = app.insert_random_book().await;
    let book_c = app.insert_random_book().await;

    app.credit_creator(book_a, creator.id, CreatorRole::Author)
        .await;
    app.credit_creator(book_b, creator.id, CreatorRole::Artist)
        .await;
    app.credit_creator(book_c, creator.id, CreatorRole::Author)
        .await;

    let resp = app.get_creator(creator.id).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let wrapper: RespWrapper<CreatorQuery> = serde_json::from_slice(&bytes).unwrap();

    let mut roles: Vec<&str> = wrapper.data.roles.iter().map(AsRef::as_ref).collect();
    roles.sort();
    assert_eq!(roles, vec!["Artist", "Author"]);
}

#[tokio::test]
async fn get_creators_returns_default_limit_and_next_cursor() {
    let app = TestApp::new().await;

    for _ in 0..25 {
        let creator = CreatorFaker.fake();
        app.insert_creator(&creator).await;
    }

    let req = Request::get("/creators").body(Body::empty()).unwrap();
    let resp = app.send(req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let resp: RespWrapper<Vec<CreatorQuery>> = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(resp.data.len(), 20);
    assert!(resp.next_cursor.is_some());
}

#[tokio::test]
async fn get_creators_with_after_and_limit_3_returns_next_page() {
    let app = TestApp::new().await;

    for _ in 0..6 {
        let creator: Creator = CreatorFaker.fake();
        app.insert_creator(&creator).await;
    }

    let req = Request::get("/creators?limit=3")
        .body(Body::empty())
        .unwrap();
    let resp = app.send(req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let resp: RespWrapper<Vec<CreatorQuery>> = serde_json::from_slice(&body).unwrap();

    let ids: Vec<uuid::Uuid> = resp.data.iter().map(|c| c.id).collect();
    let cursor = resp.next_cursor.unwrap();

    let second_req = Request::get(format!("/creators?limit=3&after={cursor}"))
        .body(Body::empty())
        .unwrap();
    let second_resp = app.send(second_req).await;
    assert_eq!(second_resp.status(), StatusCode::OK);

    let second_body = second_resp.into_body().collect().await.unwrap().to_bytes();
    let second_resp: RespWrapper<Vec<CreatorQuery>> = serde_json::from_slice(&second_body).unwrap();

    let second_ids: Vec<uuid::Uuid> = second_resp.data.iter().map(|c| c.id).collect();

    assert_eq!(second_ids.len(), 3);
    assert!(second_ids.iter().all(|id| !ids.contains(id)));
    assert!(second_resp.next_cursor.is_none());
}

#[tokio::test]
async fn get_creators_desc_order_confirmed() {
    let app = TestApp::new().await;

    let mut ids = Vec::with_capacity(3);
    for _ in 0..3 {
        let creator: Creator = CreatorFaker.fake();
        ids.push(creator.id);
        app.insert_creator(&creator).await;
    }
    ids.sort_by(|a, b| b.cmp(a));

    let req = Request::get("/creators").body(Body::empty()).unwrap();
    let resp = app.send(req).await;
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let resp: RespWrapper<Vec<CreatorQuery>> = serde_json::from_slice(&bytes).unwrap();
    let returned_ids: Vec<uuid::Uuid> = resp.data.iter().map(|c| c.id).collect();
    assert_eq!(returned_ids, ids);
}

#[tokio::test]
async fn get_creators_next_cursor_null_on_last_page() {
    let app = TestApp::new().await;

    for _ in 0..10 {
        app.insert_creator(&CreatorFaker.fake()).await;
    }

    let req = Request::get("/creators").body(Body::empty()).unwrap();
    let resp = app.send(req).await;
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(v["nextCursor"].is_null());
}

#[tokio::test]
async fn get_creators_invalid_limit_returns_400() {
    let app = TestApp::new().await;

    let req = Request::get("/creators?limit=abc")
        .body(Body::empty())
        .unwrap();

    let resp = app.send(req).await;
    assert_error(resp, StatusCode::BAD_REQUEST).await;
}

#[tokio::test]
async fn get_creators_zero_limit_returns_422() {
    let app = TestApp::new().await;

    let req = Request::get("/creators?limit=0")
        .body(Body::empty())
        .unwrap();

    let resp = app.send(req).await;
    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;
}

#[tokio::test]
async fn get_creators_invalid_after_returns_400() {
    let app = TestApp::new().await;

    let req = Request::get("/creators?after=not-a-uuid")
        .body(Body::empty())
        .unwrap();

    let resp = app.send(req).await;
    assert_error(resp, StatusCode::BAD_REQUEST).await;
}

#[tokio::test]
async fn delete_creator_with_valid_id_passes() {
    let app = TestApp::new().await;
    let creator = CreatorFaker.fake();

    app.insert_creator(&creator).await;

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

    app.insert_creator(&creator).await;
    app.insert_creator(&other_creator).await;

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
