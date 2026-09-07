use crate::helpers::TestApp;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};

#[tokio::test]
async fn healthz_works() {
    let app = TestApp::new().await;
    let req = Request::get("/healthz").body(Body::empty()).unwrap();

    let resp = app.send(req).await;

    assert_eq!(resp.status(), StatusCode::OK);
}
