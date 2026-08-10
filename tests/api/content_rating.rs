use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use tower::ServiceExt;

use crate::helpers::{RespWrapper, TestApp};
use manga_theka::entity::ContentRating;

#[tokio::test]
async fn get_content_ratings_returns_seeded_set() {
    let app = TestApp::new().await;

    let req = Request::get("/content_ratings")
        .body(Body::empty())
        .unwrap();
    let resp = app.router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let wrapper: RespWrapper<Vec<ContentRating>> = serde_json::from_slice(&bytes).unwrap();

    let mut codes: Vec<String> = wrapper.data.iter().map(|c| c.code.clone()).collect();
    codes.sort();
    let mut expected = vec![
        "E".to_string(),
        "T".to_string(),
        "T+".to_string(),
        "M".to_string(),
    ];
    expected.sort();
    assert_eq!(codes, expected);
}
