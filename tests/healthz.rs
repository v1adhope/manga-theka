use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use manga_theka::startup::App;
use tower::ServiceExt;

#[tokio::test]
async fn healthz_works() {
    // TODO: config
    let addr = "0.0.0.0:3000";
    let pg_url = "postgres://postgres:postgres@localhost:5432/manga_theka";

    let app = App::build(addr, pg_url).await.unwrap();
    let router = app.router();
    let req = Request::get("/healthz").body(Body::empty()).unwrap();

    let resp = router.oneshot(req).await.unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}
