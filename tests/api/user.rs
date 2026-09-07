use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use fake::Fake;
use http_body_util::BodyExt;
use manga_theka::entity::{Role, UserQuery};
use uuid::Uuid;

use crate::fakers::UserFaker;
use crate::helpers::{TestApp, assert_error, assert_stored};

async fn register(app: &TestApp, body: serde_json::Value) -> axum::response::Response {
    let req = Request::post("/users/register")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();

    app.send_raw(req).await
}

fn valid_body() -> serde_json::Value {
    serde_json::json!({
        "email": format!("{}@example.test", Uuid::now_v7().simple()),
        "username": format!("u{}", &Uuid::now_v7().simple().to_string()[..16]),
        "password": "correct horse battery staple",
    })
}

#[tokio::test]
async fn register_with_a_valid_body_returns_201_and_persists_a_hashed_reader() {
    let app = TestApp::new().await;
    let body = valid_body();

    let id = assert_stored(register(&app, body.clone()).await).await;

    let row = sqlx::query!(
        "select id, roles, password_hash from users where email = $1",
        body["email"].as_str().unwrap()
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert_eq!(row.id, id);
    assert_eq!(row.roles, vec!["Reader".to_owned()]);
    assert!(row.password_hash.starts_with("$argon2id$"));
}

#[tokio::test]
async fn register_with_a_taken_email_returns_409() {
    let app = TestApp::new().await;
    let existing: UserQuery = UserFaker::default().fake();
    app.insert_user(&existing).await;

    let mut body = valid_body();
    body["email"] = serde_json::json!(existing.email.as_ref());

    assert_error(register(&app, body).await, StatusCode::CONFLICT).await;
}

#[tokio::test]
async fn register_with_a_taken_username_returns_409() {
    let app = TestApp::new().await;
    let existing: UserQuery = UserFaker::default().fake();
    app.insert_user(&existing).await;

    let mut body = valid_body();
    body["username"] = serde_json::json!(existing.username.as_ref());

    assert_error(register(&app, body).await, StatusCode::CONFLICT).await;
}

#[tokio::test]
async fn register_with_semantically_invalid_fields_returns_422() {
    let app = TestApp::new().await;

    let cases = [
        ("bad email", serde_json::json!("not-an-email")),
        ("bad username", serde_json::json!("a")),
        ("short password", serde_json::json!("fifteen chars..")),
        ("long password", serde_json::json!("x".repeat(129))),
    ];

    for (label, override_value) in cases {
        let mut body = valid_body();
        let key = match label {
            "bad email" => "email",
            "bad username" => "username",
            _ => "password",
        };
        body[key] = override_value;

        assert_error(register(&app, body).await, StatusCode::UNPROCESSABLE_ENTITY).await;
    }
}

#[tokio::test]
async fn register_with_broken_json_returns_400() {
    let app = TestApp::new().await;
    let req = Request::post("/users/register")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{not json"))
        .unwrap();

    assert_error(app.send_raw(req).await, StatusCode::BAD_REQUEST).await;
}

#[tokio::test]
async fn get_me_returns_the_caller_without_the_hash() {
    let app = TestApp::new().await;
    let user: UserQuery = UserFaker {
        roles: vec![Role::Reader, Role::Uploader],
        verified: false,
    }
    .fake();
    app.insert_user(&user).await;

    let req = Request::get("/users/me")
        .header(
            header::AUTHORIZATION,
            app.bearer(user.id, Uuid::now_v7(), user.roles.as_slice()),
        )
        .body(Body::empty())
        .unwrap();
    let resp = app.send_raw(req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["data"]["id"], user.id.to_string());
    assert_eq!(v["data"]["email"], user.email.as_ref());
    assert_eq!(
        v["data"]["roles"],
        serde_json::json!(["Reader", "Uploader"])
    );
    assert!(v["data"].get("passwordHash").is_none());
}

#[tokio::test]
async fn get_me_without_a_token_returns_401() {
    let app = TestApp::new().await;
    let req = Request::get("/users/me").body(Body::empty()).unwrap();

    assert_error(app.send_raw(req).await, StatusCode::UNAUTHORIZED).await;
}

#[tokio::test]
async fn get_me_after_the_row_is_gone_returns_404() {
    let app = TestApp::new().await;
    let user: UserQuery = UserFaker::default().fake();
    app.insert_user(&user).await;

    sqlx::query!("delete from users where id = $1", user.id)
        .execute(&app.pool)
        .await
        .unwrap();

    let req = Request::get("/users/me")
        .header(
            header::AUTHORIZATION,
            app.bearer(user.id, Uuid::now_v7(), user.roles.as_slice()),
        )
        .body(Body::empty())
        .unwrap();

    assert_error(app.send_raw(req).await, StatusCode::NOT_FOUND).await;
}
