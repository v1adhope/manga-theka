use crate::helpers::TestApp;

use axum::http::StatusCode;

#[tokio::test]
async fn healthz_works() {
    let app = TestApp::new().await;

    let resp = app.get_raw("/healthz").await;

    assert_eq!(resp.status(), StatusCode::OK);
}
