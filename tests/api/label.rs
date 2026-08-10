use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

use crate::helpers::{RespWrapper, TestApp};
use manga_theka::entity::{Label, LabelType};

#[tokio::test]
async fn get_labels_with_no_filter_returns_all_labels() {
    let app = TestApp::new().await;

    app.insert_label(&Label {
        id: Uuid::now_v7(),
        name: "Isekai".to_owned(),
        r#type: LabelType::Genre,
    })
    .await;
    app.insert_label(&Label {
        id: Uuid::now_v7(),
        name: "Completed Translation".to_owned(),
        r#type: LabelType::Tag,
    })
    .await;

    let req = Request::get("/labels").body(Body::empty()).unwrap();
    let resp = app.router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let wrapper: RespWrapper<Vec<Label>> = serde_json::from_slice(&bytes).unwrap();

    let mut names: Vec<String> = wrapper.data.iter().map(|l| l.name.clone()).collect();
    names.sort();
    let mut expected = vec!["Isekai".to_owned(), "Completed Translation".to_owned()];
    expected.sort();
    assert_eq!(names, expected);
}

#[tokio::test]
async fn get_labels_filtered_by_genre_returns_only_genre_labels() {
    let app = TestApp::new().await;

    app.insert_label(&Label {
        id: Uuid::now_v7(),
        name: "Isekai".to_owned(),
        r#type: LabelType::Genre,
    })
    .await;
    app.insert_label(&Label {
        id: Uuid::now_v7(),
        name: "Completed Translation".to_owned(),
        r#type: LabelType::Tag,
    })
    .await;

    let req = Request::get("/labels?type=Genre")
        .body(Body::empty())
        .unwrap();
    let resp = app.router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let wrapper: RespWrapper<Vec<Label>> = serde_json::from_slice(&bytes).unwrap();

    let names: Vec<String> = wrapper.data.iter().map(|l| l.name.clone()).collect();
    assert_eq!(names, vec!["Isekai".to_owned()]);
}

#[tokio::test]
async fn get_labels_filtered_by_tag_returns_only_tag_labels() {
    let app = TestApp::new().await;

    app.insert_label(&Label {
        id: Uuid::now_v7(),
        name: "Isekai".to_owned(),
        r#type: LabelType::Genre,
    })
    .await;
    app.insert_label(&Label {
        id: Uuid::now_v7(),
        name: "Completed Translation".to_owned(),
        r#type: LabelType::Tag,
    })
    .await;

    let req = Request::get("/labels?type=Tag")
        .body(Body::empty())
        .unwrap();
    let resp = app.router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let wrapper: RespWrapper<Vec<Label>> = serde_json::from_slice(&bytes).unwrap();

    let names: Vec<String> = wrapper.data.iter().map(|l| l.name.clone()).collect();
    assert_eq!(names, vec!["Completed Translation".to_owned()]);
}

#[tokio::test]
async fn get_labels_with_invalid_type_returns_422() {
    let app = TestApp::new().await;

    let req = Request::get("/labels?type=NotAType")
        .body(Body::empty())
        .unwrap();
    let resp = app.router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
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
