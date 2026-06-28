use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use manga_theka::startup::App;
use tower::ServiceExt;

#[tokio::test]
async fn healthz_works() {
    let app = App::build("0.0.0.0:3000").await.unwrap();
    let router = app.router();
    let req = Request::get("/healthz").body(Body::empty()).unwrap();

    let resp = router.oneshot(req).await.unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}
