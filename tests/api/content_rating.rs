use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

use crate::helpers::{RespWrapper, TestApp};
use manga_theka::entity::ContentRating;

#[tokio::test]
async fn get_content_ratings_returns_seeded_set() {
    let app = TestApp::new().await;

    let req = Request::get("/content-ratings")
        .body(Body::empty())
        .unwrap();
    let resp = app.router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let wrapper: RespWrapper<Vec<ContentRating>> = serde_json::from_slice(&bytes).unwrap();

    let mut actual = wrapper.data;
    actual.sort();

    let mut expected = vec![
        ContentRating {
            id: Uuid::parse_str("019f124b-314f-73fc-8310-701df63eacea").unwrap(),
            name: "Everyone".to_string(),
            code: "E".to_string(),
        },
        ContentRating {
            id: Uuid::parse_str("019f125c-33ef-753d-984f-08781390d2f4").unwrap(),
            name: "Teen".to_string(),
            code: "T".to_string(),
        },
        ContentRating {
            id: Uuid::parse_str("019f125c-bf63-7578-a57d-16f2f593c365").unwrap(),
            name: "Teen Plus".to_string(),
            code: "T+".to_string(),
        },
        ContentRating {
            id: Uuid::parse_str("019f125d-2006-7a75-bcc2-4fc07e370d2d").unwrap(),
            name: "Mature".to_string(),
            code: "M".to_string(),
        },
    ];
    expected.sort();

    assert_eq!(actual, expected);
}
