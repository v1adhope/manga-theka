use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use fake::Fake;
use http_body_util::BodyExt;
use manga_theka::entity::{Role, Session, UserQuery};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::fakers::UserFaker;
use crate::helpers::{KNOWN_PASSWORD, TestApp, assert_error};

async fn post_login(app: &TestApp, email: &str, password: &str) -> axum::response::Response {
    let body = serde_json::json!({ "email": email, "password": password }).to_string();
    let req = Request::post("/sessions/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    app.send_raw(req).await
}

fn refresh_cookie(resp: &axum::response::Response) -> String {
    resp.headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .map(|v| v.to_str().unwrap())
        .find(|v| v.starts_with("refresh_token="))
        .expect("a login response must set the refresh cookie")
        .to_owned()
}

fn cookie_pair(set_cookie: &str) -> String {
    set_cookie
        .split(';')
        .next()
        .expect("a Set-Cookie must carry a name=value pair")
        .to_owned()
}

async fn body_json(resp: axum::response::Response) -> serde_json::Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

async fn seeded_user(app: &TestApp) -> UserQuery {
    let user: UserQuery = UserFaker {
        roles: vec![Role::Reader],
        verified: true,
    }
    .fake();
    app.insert_user(&user).await;

    user
}

// -- POST /sessions/login --------------------------------------------------

#[tokio::test]
async fn login_with_valid_credentials_returns_201_a_token_and_a_scoped_cookie() {
    let app = TestApp::new().await;
    let user = seeded_user(&app).await;

    let resp = post_login(&app, user.email.as_ref(), KNOWN_PASSWORD).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let set_cookie = refresh_cookie(&resp);
    for attr in [
        "refresh_token=",
        "HttpOnly",
        "Secure",
        "SameSite=Strict",
        "Path=/sessions",
        "Max-Age=2592000",
    ] {
        assert!(
            set_cookie.contains(attr),
            "cookie missing {attr}: {set_cookie}"
        );
    }

    let v = body_json(resp).await;
    assert!(
        v["data"]["accessToken"]
            .as_str()
            .is_some_and(|t| !t.is_empty()),
        "login body must carry a non-empty access token"
    );
}

#[tokio::test]
async fn login_with_a_wrong_password_returns_401() {
    let app = TestApp::new().await;
    let user = seeded_user(&app).await;

    assert_error(
        post_login(&app, user.email.as_ref(), "the wrong passphrase!!").await,
        StatusCode::UNAUTHORIZED,
    )
    .await;
}

#[tokio::test]
async fn login_with_an_unknown_email_returns_401() {
    let app = TestApp::new().await;

    assert_error(
        post_login(&app, "nobody@example.test", KNOWN_PASSWORD).await,
        StatusCode::UNAUTHORIZED,
    )
    .await;
}

// -- POST /sessions/refresh ---------------------------------------------------

async fn login_and_get_cookie(app: &TestApp) -> String {
    let user = seeded_user(app).await;
    let resp = post_login(app, user.email.as_ref(), KNOWN_PASSWORD).await;
    cookie_pair(&refresh_cookie(&resp))
}

async fn post_refresh(app: &TestApp, cookie: &str) -> axum::response::Response {
    let req = Request::post("/sessions/refresh")
        .header(header::COOKIE, cookie)
        .body(Body::empty())
        .unwrap();

    app.send_raw(req).await
}

#[tokio::test]
async fn refresh_succeeds_even_with_a_stale_access_token_still_attached() {
    let app = TestApp::new().await;
    let cookie = login_and_get_cookie(&app).await;

    // The SPA pattern keeps the access token on every request; an expired one
    // riding along on the refresh call must not block the renewal.
    let stale = app
        .jwt
        .issue_access(
            Uuid::now_v7(),
            Uuid::now_v7(),
            &[Role::Reader],
            OffsetDateTime::now_utc() - Duration::hours(1),
        )
        .unwrap();

    let req = Request::post("/sessions/refresh")
        .header(header::COOKIE, cookie)
        .header(header::AUTHORIZATION, format!("Bearer {stale}"))
        .body(Body::empty())
        .unwrap();

    assert_eq!(app.send_raw(req).await.status(), StatusCode::OK);
}

#[tokio::test]
async fn refresh_rotates_the_cookie_and_invalidates_the_presented_one() {
    let app = TestApp::new().await;
    let first = login_and_get_cookie(&app).await;

    let rotated = post_refresh(&app, &first).await;
    assert_eq!(rotated.status(), StatusCode::OK);
    let second = cookie_pair(&refresh_cookie(&rotated));
    assert_ne!(first, second, "refresh must mint a fresh cookie value");

    // The replayed original now fails the stored-jti comparison.
    assert_error(post_refresh(&app, &first).await, StatusCode::UNAUTHORIZED).await;

    // The rotated cookie still works.
    assert_eq!(post_refresh(&app, &second).await.status(), StatusCode::OK);
}

#[tokio::test]
async fn refresh_with_an_expired_token_returns_401() {
    let app = TestApp::new().await;
    let sub = Uuid::now_v7();
    let sid = Uuid::now_v7();
    let jti = Uuid::now_v7();
    let long_ago = OffsetDateTime::now_utc() - Duration::days(40);

    app.memory
        .put_session(
            sub,
            Session {
                sid,
                jti: app.hasher.keyed_jti_hash(jti).unwrap(),
                ua: None,
                ip: None,
                created_at: long_ago.into(),
                updated_at: long_ago.into(),
            },
        )
        .await
        .unwrap();

    let token = app.jwt.issue_refresh(sub, sid, jti, long_ago).unwrap();

    assert_error(
        post_refresh(&app, &format!("refresh_token={token}")).await,
        StatusCode::UNAUTHORIZED,
    )
    .await;
}

#[tokio::test]
async fn an_access_token_presented_as_a_refresh_token_is_rejected() {
    let app = TestApp::new().await;
    let access = app.access_token(Uuid::now_v7(), Uuid::now_v7(), &[Role::Reader]);

    assert_error(
        post_refresh(&app, &format!("refresh_token={access}")).await,
        StatusCode::UNAUTHORIZED,
    )
    .await;
}

#[tokio::test]
async fn a_refresh_token_presented_as_an_access_token_is_rejected() {
    let app = TestApp::new().await;
    let refresh = app
        .jwt
        .issue_refresh(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            OffsetDateTime::now_utc(),
        )
        .unwrap();

    let req = Request::get("/users/me")
        .header(header::AUTHORIZATION, format!("Bearer {refresh}"))
        .body(Body::empty())
        .unwrap();

    assert_error(app.send_raw(req).await, StatusCode::UNAUTHORIZED).await;
}

// -- GET /sessions/me -------------------------------------------------------

#[tokio::test]
async fn list_my_sessions_returns_every_live_session_without_the_jti() {
    let app = TestApp::new().await;
    let sub = Uuid::now_v7();
    let current = Uuid::now_v7();
    let other = Uuid::now_v7();
    let now = OffsetDateTime::now_utc();

    app.insert_session(sub, current, now).await;
    app.insert_session(sub, other, now).await;

    let req = Request::get("/sessions/me")
        .header(
            header::AUTHORIZATION,
            app.bearer(sub, current, &[Role::Reader]),
        )
        .body(Body::empty())
        .unwrap();
    let resp = app.send_raw(req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let v = body_json(resp).await;
    let rows = v["data"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    for row in rows {
        assert!(row.get("sid").is_some());
        assert!(row.get("createdAt").is_some());
        assert!(row.get("jti").is_none(), "the read shape must scrub jti");
    }
}

// -- DELETE /sessions/me/* ------------------------------------------------------

async fn delete(app: &TestApp, path: &str, sub: Uuid, sid: Uuid) -> axum::response::Response {
    let req = Request::delete(path)
        .header(header::AUTHORIZATION, app.bearer(sub, sid, &[Role::Reader]))
        .body(Body::empty())
        .unwrap();

    app.send_raw(req).await
}

#[tokio::test]
async fn deleting_the_current_session_bypasses_the_24h_rule() {
    let app = TestApp::new().await;
    let sub = Uuid::now_v7();
    let sid = Uuid::now_v7();
    app.insert_session(sub, sid, OffsetDateTime::now_utc())
        .await;

    let resp = delete(&app, "/sessions/me/current", sub, sid).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    assert!(app.memory.get_session(sub, sid).await.unwrap().is_none());
}

#[tokio::test]
async fn deleting_another_session_that_is_not_mine_returns_404() {
    let app = TestApp::new().await;
    let sub = Uuid::now_v7();
    let current = Uuid::now_v7();
    app.insert_session(
        sub,
        current,
        OffsetDateTime::now_utc() - Duration::hours(48),
    )
    .await;

    let resp = delete(
        &app,
        &format!("/sessions/me/{}", Uuid::now_v7()),
        sub,
        current,
    )
    .await;
    assert_error(resp, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn deleting_another_session_from_a_fresh_session_returns_409() {
    let app = TestApp::new().await;
    let sub = Uuid::now_v7();
    let current = Uuid::now_v7();
    let target = Uuid::now_v7();
    app.insert_session(sub, current, OffsetDateTime::now_utc())
        .await;
    app.insert_session(sub, target, OffsetDateTime::now_utc())
        .await;

    let resp = delete(&app, &format!("/sessions/me/{target}"), sub, current).await;
    assert_error(resp, StatusCode::CONFLICT).await;
    assert!(
        app.memory.get_session(sub, target).await.unwrap().is_some(),
        "a blocked revoke must leave the target alive"
    );
}

#[tokio::test]
async fn deleting_another_session_from_an_aged_session_passes() {
    let app = TestApp::new().await;
    let sub = Uuid::now_v7();
    let current = Uuid::now_v7();
    let target = Uuid::now_v7();
    app.insert_session(
        sub,
        current,
        OffsetDateTime::now_utc() - Duration::hours(48),
    )
    .await;
    app.insert_session(sub, target, OffsetDateTime::now_utc())
        .await;

    let resp = delete(&app, &format!("/sessions/me/{target}"), sub, current).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    assert!(app.memory.get_session(sub, target).await.unwrap().is_none());
}

#[tokio::test]
async fn deleting_all_sessions_from_an_aged_session_passes_and_from_a_fresh_one_409s() {
    let app = TestApp::new().await;

    let aged_sub = Uuid::now_v7();
    let aged_sid = Uuid::now_v7();
    app.insert_session(
        aged_sub,
        aged_sid,
        OffsetDateTime::now_utc() - Duration::hours(48),
    )
    .await;
    app.insert_session(aged_sub, Uuid::now_v7(), OffsetDateTime::now_utc())
        .await;

    let resp = delete(&app, "/sessions/me/all", aged_sub, aged_sid).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    assert!(app.memory.list_sessions(aged_sub).await.unwrap().is_empty());

    let fresh_sub = Uuid::now_v7();
    let fresh_sid = Uuid::now_v7();
    app.insert_session(fresh_sub, fresh_sid, OffsetDateTime::now_utc())
        .await;

    assert_error(
        delete(&app, "/sessions/me/all", fresh_sub, fresh_sid).await,
        StatusCode::CONFLICT,
    )
    .await;
}

// -- Role gate ------------------------------------------------------------------

fn creator_body() -> String {
    serde_json::json!({ "firstName": "Gate", "lastName": "Probe" }).to_string()
}

#[tokio::test]
async fn a_gated_route_without_a_token_returns_401() {
    let app = TestApp::new().await;
    let req = Request::post("/creators")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(creator_body()))
        .unwrap();

    assert_error(app.send_raw(req).await, StatusCode::UNAUTHORIZED).await;
}

#[tokio::test]
async fn a_gated_route_with_a_role_below_the_gate_returns_403() {
    let app = TestApp::new().await;
    let req = Request::post("/creators")
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            header::AUTHORIZATION,
            app.bearer(Uuid::now_v7(), Uuid::now_v7(), &[Role::Reader]),
        )
        .body(Body::from(creator_body()))
        .unwrap();

    assert_error(app.send_raw(req).await, StatusCode::FORBIDDEN).await;
}

#[tokio::test]
async fn a_gated_route_with_a_role_at_the_gate_passes() {
    let app = TestApp::new().await;
    let req = Request::post("/creators")
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            header::AUTHORIZATION,
            app.bearer(Uuid::now_v7(), Uuid::now_v7(), &[Role::Uploader]),
        )
        .body(Body::from(creator_body()))
        .unwrap();

    assert_eq!(app.send_raw(req).await.status(), StatusCode::CREATED);
}
