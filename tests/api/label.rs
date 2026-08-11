use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use tower::ServiceExt;

use crate::helpers::{RespWrapper, TestApp};
use manga_theka::entity::Label;

const GENRE_NAMES: [&str; 5] = ["Action", "Adventure", "Comedy", "Crime", "Drama"];

const TAG_NAMES: [&str; 5] = ["Mafia", "Music", "School Life", "Survival", "Time Travel"];

#[tokio::test]
async fn get_labels_with_no_filter_returns_seeded_labels() {
    let app = TestApp::new().await;

    let req = Request::get("/labels").body(Body::empty()).unwrap();
    let resp = app.router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let wrapper: RespWrapper<Vec<Label>> = serde_json::from_slice(&bytes).unwrap();

    let names: Vec<String> = wrapper.data.iter().map(|l| l.name.clone()).collect();

    for expected in GENRE_NAMES.iter().chain(TAG_NAMES.iter()) {
        assert!(
            names.contains(&expected.to_string()),
            "expected label {expected:?} to be present"
        );
    }
}

#[tokio::test]
async fn get_labels_filtered_by_genre_returns_only_genre_labels() {
    let app = TestApp::new().await;

    let req = Request::get("/labels?kind=Genre")
        .body(Body::empty())
        .unwrap();
    let resp = app.router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let wrapper: RespWrapper<Vec<Label>> = serde_json::from_slice(&bytes).unwrap();

    let names: Vec<String> = wrapper.data.iter().map(|l| l.name.clone()).collect();

    for expected in GENRE_NAMES {
        assert!(
            names.contains(&expected.to_string()),
            "expected genre {expected:?} to be present"
        );
    }
    for unexpected in TAG_NAMES {
        assert!(
            !names.contains(&unexpected.to_string()),
            "did not expect tag {unexpected:?} among genre results"
        );
    }
}

#[tokio::test]
async fn get_labels_filtered_by_tag_returns_only_tag_labels() {
    let app = TestApp::new().await;

    let req = Request::get("/labels?kind=Tag")
        .body(Body::empty())
        .unwrap();
    let resp = app.router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let wrapper: RespWrapper<Vec<Label>> = serde_json::from_slice(&bytes).unwrap();

    let names: Vec<String> = wrapper.data.iter().map(|l| l.name.clone()).collect();

    for expected in TAG_NAMES {
        assert!(
            names.contains(&expected.to_string()),
            "expected tag {expected:?} to be present"
        );
    }
    for unexpected in GENRE_NAMES {
        assert!(
            !names.contains(&unexpected.to_string()),
            "did not expect genre {unexpected:?} among tag results"
        );
    }
}

#[tokio::test]
async fn get_labels_with_invalid_kind_returns_400() {
    let app = TestApp::new().await;

    let req = Request::get("/labels?kind=NotAType")
        .body(Body::empty())
        .unwrap();
    let resp = app.router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    assert!(
        !resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .is_empty()
    );
}
