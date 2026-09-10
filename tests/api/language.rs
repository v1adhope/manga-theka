use uuid::Uuid;

use crate::helpers::{RespWrapper, TestApp};
use manga_theka::entity::Language;

#[tokio::test]
async fn get_languages_returns_seeded_set() {
    let app = TestApp::new().await;

    let wrapper = app
        .get_ok_json::<RespWrapper<Vec<Language>>>("/languages")
        .await;

    let mut actual: Vec<(Uuid, String)> =
        wrapper.data.into_iter().map(|l| (l.id, l.code)).collect();
    actual.sort();

    let mut expected = vec![
        (
            Uuid::parse_str("019f12ac-d1fc-78f2-a4bc-d0827c0f1578").unwrap(),
            "ja".to_string(),
        ),
        (
            Uuid::parse_str("019f12ac-f9f3-7b8c-ba3f-97033810d391").unwrap(),
            "ko".to_string(),
        ),
        (
            Uuid::parse_str("019f12ad-0c41-7022-8877-50861d4ec2a4").unwrap(),
            "zh".to_string(),
        ),
        (
            Uuid::parse_str("019f12ad-1e26-7fdd-9318-b18ccd74e734").unwrap(),
            "en".to_string(),
        ),
        (
            Uuid::parse_str("019f12ad-2e9e-762d-999c-b4bc9bbdc964").unwrap(),
            "ru".to_string(),
        ),
    ];
    expected.sort();

    assert_eq!(actual, expected);
}
