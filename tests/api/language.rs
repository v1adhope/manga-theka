use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use tower::ServiceExt;

use crate::helpers::{RespWrapper, TestApp};
use manga_theka::entity::Language;

#[tokio::test]
async fn get_languages_returns_seeded_set() {
    let app = TestApp::new().await;

    let req = Request::get("/languages").body(Body::empty()).unwrap();
    let resp = app.router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let wrapper: RespWrapper<Vec<Language>> = serde_json::from_slice(&bytes).unwrap();

    let mut codes: Vec<String> = wrapper.data.iter().map(|l| l.code.clone()).collect();
    codes.sort();
    let mut expected = vec![
        "ja".to_string(),
        "ko".to_string(),
        "zh".to_string(),
        "en".to_string(),
        "ru".to_string(),
    ];
    expected.sort();
    assert_eq!(codes, expected);
}
