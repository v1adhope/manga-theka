use std::net::IpAddr;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use fake::Fake;
use fake::faker::internet::en::{IP, UserAgent};
use http_body_util::BodyExt;
use manga_theka::entity::{Role, Session, SessionQuery, ShortText};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::helpers::{
    AccessTokenBody, KNOWN_PASSWORD, RespWrapper, TestApp, assert_error, cookie_pair,
    refresh_cookie,
};

async fn login_and_get_cookie(app: &TestApp) -> String {
    let user = app.db_seed_reader().await;
    let resp = app.post_login(user.email.as_ref(), KNOWN_PASSWORD).await;
    cookie_pair(&refresh_cookie(&resp))
}

fn creator_body() -> serde_json::Value {
    serde_json::json!({ "firstName": "Gate", "lastName": "Probe" })
}

#[tokio::test]
async fn login_with_valid_credentials_returns_201_a_token_and_a_scoped_cookie() {
    let app = TestApp::new().await;
    let user = app.db_seed_reader().await;

    let resp = app.post_login(user.email.as_ref(), KNOWN_PASSWORD).await;
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

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let body: RespWrapper<AccessTokenBody> = serde_json::from_slice(&bytes).unwrap();
    assert!(
        !body.data.access_token.is_empty(),
        "login body must carry a non-empty access token"
    );
    assert!(
        body.data.expires_in > 0,
        "login body must carry a positive access-token lifetime"
    );
}

#[tokio::test]
async fn login_with_a_wrong_password_returns_401() {
    let app = TestApp::new().await;
    let user = app.db_seed_reader().await;

    assert_error(
        app.post_login(user.email.as_ref(), "the wrong passphrase!!")
            .await,
        StatusCode::UNAUTHORIZED,
    )
    .await;
}

#[tokio::test]
async fn login_with_an_unknown_email_returns_401() {
    let app = TestApp::new().await;

    assert_error(
        app.post_login("nobody@example.test", KNOWN_PASSWORD).await,
        StatusCode::UNAUTHORIZED,
    )
    .await;
}

#[tokio::test]
async fn refresh_succeeds_even_with_a_stale_access_token_still_attached() {
    let app = TestApp::new().await;
    let cookie = login_and_get_cookie(&app).await;

    let stale = app
        .jwt
        .issue_access(
            Uuid::now_v7(),
            Uuid::now_v7(),
            &[Role::Reader],
            OffsetDateTime::now_utc() - Duration::hours(1),
        )
        .unwrap()
        .value;

    let req = Request::post("/sessions/refresh")
        .header(header::COOKIE, cookie)
        .header(header::AUTHORIZATION, format!("Bearer {stale}"))
        .body(Body::empty())
        .unwrap();

    let refreshed = app.send_raw(req).await;
    assert_eq!(refreshed.status(), StatusCode::OK);
}

#[tokio::test]
async fn refresh_rotates_the_cookie_and_invalidates_the_presented_one() {
    let app = TestApp::new().await;
    let first = login_and_get_cookie(&app).await;

    let rotated = app.post_refresh(&first).await;
    assert_eq!(rotated.status(), StatusCode::OK);
    let second = cookie_pair(&refresh_cookie(&rotated));
    assert_ne!(first, second, "refresh must mint a fresh cookie value");

    let replayed_first = app.post_refresh(&first).await;
    assert_error(replayed_first, StatusCode::UNAUTHORIZED).await;

    let replayed_second = app.post_refresh(&second).await;
    assert_eq!(replayed_second.status(), StatusCode::OK);
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
                jti: app.hasher.compute_keyed_hex_hash(jti).unwrap(),
                ua: None,
                ip: None,
                created_at: long_ago.into(),
                updated_at: long_ago.into(),
            },
        )
        .await
        .unwrap();

    let token = app
        .jwt
        .issue_refresh(sub, sid, jti, long_ago)
        .unwrap()
        .value;

    assert_error(
        app.post_refresh(&format!("refresh_token={token}")).await,
        StatusCode::UNAUTHORIZED,
    )
    .await;
}

#[tokio::test]
async fn an_access_token_presented_as_a_refresh_token_is_rejected() {
    let app = TestApp::new().await;
    let access = app.access_token(Uuid::now_v7(), Uuid::now_v7(), &[Role::Reader]);

    assert_error(
        app.post_refresh(&format!("refresh_token={access}")).await,
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
        .unwrap()
        .value;

    let req = Request::get("/users/me")
        .header(header::AUTHORIZATION, format!("Bearer {refresh}"))
        .body(Body::empty())
        .unwrap();

    assert_error(app.send_raw(req).await, StatusCode::UNAUTHORIZED).await;
}

#[tokio::test]
async fn list_my_sessions_returns_every_live_session_with_the_full_read_shape() {
    let app = TestApp::new().await;
    let sub = Uuid::now_v7();
    let current = Uuid::now_v7();
    let bare = Uuid::now_v7();
    let expected_ua: String = UserAgent().fake();
    let expected_ip: IpAddr = IP().fake();
    let created_at = OffsetDateTime::now_utc() - Duration::hours(2);
    let updated_at = OffsetDateTime::now_utc() - Duration::minutes(5);

    app.memory
        .put_session(
            sub,
            Session {
                sid: current,
                jti: app.hasher.compute_keyed_hex_hash(Uuid::now_v7()).unwrap(),
                ua: Some(ShortText::try_from(expected_ua.clone()).unwrap()),
                ip: Some(expected_ip),
                created_at: created_at.into(),
                updated_at: updated_at.into(),
            },
        )
        .await
        .unwrap();
    app.memory_insert_session(sub, bare, OffsetDateTime::now_utc())
        .await;

    let req = Request::get("/sessions/me")
        .header(
            header::AUTHORIZATION,
            app.bearer(sub, current, &[Role::Reader]),
        )
        .body(Body::empty())
        .unwrap();
    let resp = app.send_raw(req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let body: RespWrapper<Vec<SessionQuery>> = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body.data.len(), 2);

    let row = body
        .data
        .iter()
        .find(|s| s.sid == current)
        .expect("the populated session must be listed");
    assert_eq!(
        row.ua.as_ref().map(|ua| ua.as_ref()),
        Some(expected_ua.as_str())
    );
    assert_eq!(row.ip, Some(expected_ip));
    assert_eq!(row.created_at.into_inner(), created_at);
    assert_eq!(row.updated_at.into_inner(), updated_at);
}

#[tokio::test]
async fn deleting_the_current_session_bypasses_the_24h_rule() {
    let app = TestApp::new().await;
    let sub = Uuid::now_v7();
    let sid = Uuid::now_v7();
    app.memory_insert_session(sub, sid, OffsetDateTime::now_utc())
        .await;

    let resp = app
        .delete_authed_as("/sessions/me/current", sub, sid, &[Role::Reader])
        .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    let revoked = app.memory.get_session(sub, sid).await.unwrap();
    assert!(revoked.is_none());
}

#[tokio::test]
async fn deleting_a_session_owned_by_another_user_returns_404() {
    let app = TestApp::new().await;
    let sub = Uuid::now_v7();
    let current = Uuid::now_v7();
    app.memory_insert_session(
        sub,
        current,
        OffsetDateTime::now_utc() - Duration::hours(48),
    )
    .await;

    let other_sub = Uuid::now_v7();
    let other_sid = Uuid::now_v7();
    app.memory_insert_session(other_sub, other_sid, OffsetDateTime::now_utc())
        .await;

    let resp = app
        .delete_authed_as(
            &format!("/sessions/me/{other_sid}"),
            sub,
            current,
            &[Role::Reader],
        )
        .await;
    assert_error(resp, StatusCode::NOT_FOUND).await;

    let survivor = app.memory.get_session(other_sub, other_sid).await.unwrap();
    assert!(
        survivor.is_some(),
        "another user's session must be untouched"
    );
}

#[tokio::test]
async fn deleting_another_session_from_a_fresh_session_returns_409() {
    let app = TestApp::new().await;
    let sub = Uuid::now_v7();
    let current = Uuid::now_v7();
    let target = Uuid::now_v7();
    app.memory_insert_session(sub, current, OffsetDateTime::now_utc())
        .await;
    app.memory_insert_session(sub, target, OffsetDateTime::now_utc())
        .await;

    let resp = app
        .delete_authed_as(
            &format!("/sessions/me/{target}"),
            sub,
            current,
            &[Role::Reader],
        )
        .await;
    assert_error(resp, StatusCode::CONFLICT).await;
    let target_session = app.memory.get_session(sub, target).await.unwrap();
    assert!(
        target_session.is_some(),
        "a blocked revoke must leave the target alive"
    );
}

#[tokio::test]
async fn deleting_another_session_from_an_aged_session_passes() {
    let app = TestApp::new().await;
    let sub = Uuid::now_v7();
    let current = Uuid::now_v7();
    let target = Uuid::now_v7();
    app.memory_insert_session(
        sub,
        current,
        OffsetDateTime::now_utc() - Duration::hours(48),
    )
    .await;
    app.memory_insert_session(sub, target, OffsetDateTime::now_utc())
        .await;

    let resp = app
        .delete_authed_as(
            &format!("/sessions/me/{target}"),
            sub,
            current,
            &[Role::Reader],
        )
        .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    let target_session = app.memory.get_session(sub, target).await.unwrap();
    assert!(target_session.is_none());
}

#[tokio::test]
async fn deleting_all_sessions_from_a_fresh_session_returns_409() {
    let app = TestApp::new().await;
    let sub = Uuid::now_v7();
    let sid = Uuid::now_v7();
    app.memory_insert_session(sub, sid, OffsetDateTime::now_utc())
        .await;

    let resp = app
        .delete_authed_as("/sessions/me/all", sub, sid, &[Role::Reader])
        .await;
    assert_error(resp, StatusCode::CONFLICT).await;
    let remaining = app.memory.list_sessions(sub).await.unwrap();
    assert!(
        !remaining.is_empty(),
        "a blocked revoke-all must leave the sessions alive"
    );
}

#[tokio::test]
async fn deleting_all_sessions_from_an_aged_session_passes() {
    let app = TestApp::new().await;
    let sub = Uuid::now_v7();
    let sid = Uuid::now_v7();
    app.memory_insert_session(sub, sid, OffsetDateTime::now_utc() - Duration::hours(48))
        .await;
    app.memory_insert_session(sub, Uuid::now_v7(), OffsetDateTime::now_utc())
        .await;

    let resp = app
        .delete_authed_as("/sessions/me/all", sub, sid, &[Role::Reader])
        .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    let remaining = app.memory.list_sessions(sub).await.unwrap();
    assert!(remaining.is_empty());
}

#[tokio::test]
async fn a_gated_route_without_a_token_returns_401() {
    let app = TestApp::new().await;
    let req = Request::post("/creators")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(creator_body().to_string()))
        .unwrap();

    assert_error(app.send_raw(req).await, StatusCode::UNAUTHORIZED).await;
}

#[tokio::test]
async fn a_gated_route_with_a_role_below_the_gate_returns_403() {
    let app = TestApp::new().await;

    assert_error(
        app.post_json_as("/creators", creator_body(), &[Role::Reader])
            .await,
        StatusCode::FORBIDDEN,
    )
    .await;
}

#[tokio::test]
async fn a_gated_route_with_a_role_at_the_gate_passes() {
    let app = TestApp::new().await;

    let resp = app
        .post_json_as("/creators", creator_body(), &[Role::Uploader])
        .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
}
