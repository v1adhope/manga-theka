mod helpers;

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use http_body_util::BodyExt;
use tower::ServiceExt;

use crate::helpers::spawn_app;

#[tokio::test]
async fn store_creator_with_valid_body_returns_200() {
    let app = spawn_app().await;
    let body = serde_json::json!({
        "firstName": "John",
        "lastName": "Doe",
        "role": "artist",
    })
    .to_string();
    let req = Request::post("/creator")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = uuid::Uuid::parse_str(v["id"].as_str().unwrap()).unwrap();

    let row = sqlx::query!("select id, first_name, last_name, role from creators",)
        .fetch_one(&app.pool)
        .await
        .unwrap();

    assert_eq!(row.id, id);
    assert_eq!(row.first_name, "John");
    assert_eq!(row.last_name, "Doe");
    assert_eq!(row.role, "artist");
}

#[tokio::test]
async fn store_creator_with_broken_json_returns_400() {
    let app = spawn_app().await;
    let req = Request::post("/creator")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{not json"))
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn store_creator_with_missing_first_name_returns_422() {
    let app = spawn_app().await;
    let body = serde_json::json!({
        "lastName": "Doe",
        "role": "artist",
    })
    .to_string();
    let req = Request::post("/creator")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
