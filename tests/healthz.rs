mod helpers;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

use crate::helpers::spawn_app;

#[tokio::test]
async fn healthz_works() {
    let app = spawn_app().await;
    let req = Request::get("/healthz").body(Body::empty()).unwrap();

    let resp = app.router.oneshot(req).await.unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}
