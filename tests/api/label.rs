use axum::{
    body::Body,
    http::{Request, StatusCode},
};

use crate::helpers::{TestApp, assert_error};

const SEEDED_LABELS: usize = 70;

const GENRE_NAMES: [&str; 5] = ["Action", "Murim", "Romance", "Wuxia", "Xianxia"];

const THEME_NAMES: [&str; 5] = ["Dungeons", "Mafia", "Regression", "School Life", "Zombies"];

const PRESENTATION_NAMES: [&str; 5] = [
    "Adaptation",
    "Full Color",
    "Long Strip",
    "Oneshot",
    "Self-Published",
];

const KIND_CASES: [(&str, [&str; 5], usize); 3] = [
    ("Genre", GENRE_NAMES, 27),
    ("Theme", THEME_NAMES, 38),
    ("Presentation", PRESENTATION_NAMES, 5),
];

#[tokio::test]
async fn get_labels_with_no_filter_returns_the_whole_curated_catalog() {
    let app = TestApp::new().await;

    let labels = app.get_labels("/labels").await;
    let names: Vec<&str> = labels.iter().map(|l| l.name.as_str()).collect();

    assert_eq!(labels.len(), SEEDED_LABELS);

    for (_, expected, _) in KIND_CASES {
        for name in expected {
            assert!(
                names.contains(&name),
                "expected label {name:?} to be present"
            );
        }
    }
}

#[tokio::test]
async fn get_labels_filtered_by_kind_returns_only_that_kind() {
    let app = TestApp::new().await;

    for (kind, expected, count) in KIND_CASES {
        let labels = app.get_labels(&format!("/labels?kind={kind}")).await;
        let names: Vec<&str> = labels.iter().map(|l| l.name.as_str()).collect();

        let all_carry_kind = labels.iter().all(|l| l.kind.as_ref() == kind);

        assert_eq!(labels.len(), count, "{kind}");
        assert!(all_carry_kind, "{kind} results must all carry that kind");

        for name in expected {
            assert!(
                names.contains(&name),
                "expected {kind} {name:?} to be present"
            );
        }
        for (other, unexpected, _) in KIND_CASES {
            if other == kind {
                continue;
            }
            for name in unexpected {
                assert!(
                    !names.contains(&name),
                    "did not expect {other} {name:?} among {kind} results"
                );
            }
        }
    }
}

#[tokio::test]
async fn get_labels_with_retired_tag_kind_returns_400() {
    let app = TestApp::new().await;

    let req = Request::get("/labels?kind=Tag")
        .body(Body::empty())
        .unwrap();
    let resp = app.send(req).await;
    assert_error(resp, StatusCode::BAD_REQUEST).await;
}

#[tokio::test]
async fn get_labels_with_invalid_kind_returns_400() {
    let app = TestApp::new().await;

    let req = Request::get("/labels?kind=NotAType")
        .body(Body::empty())
        .unwrap();
    let resp = app.send(req).await;
    assert_error(resp, StatusCode::BAD_REQUEST).await;
}
