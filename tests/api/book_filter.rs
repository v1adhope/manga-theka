use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

use crate::fakers::{
    ACTION, BookFaker, CONTENT_RATINGS, FANTASY, ISEKAI, LABELS, LANGUAGES, LONG_STRIP, MAFIA,
    ROMANCE, SCHOOL_LIFE, ZOMBIES,
};
use crate::helpers::{TestApp, day, ids, pick, rfc3339, sorted};
use fake::Fake;
use manga_theka::entity::{BookQuery, BookVisibility};

// A facet case is (query string, expected corpus indices). The query in the failure message is
// what makes one row of a table name itself when it breaks.
const LABEL_CASES: &[(&str, &[usize])] = &[
    ("?labels={A}", &[0, 1, 3, 9]),
    ("?labels={A}&labels={R}", &[0, 3]),
    ("?labels={A}&labels={R}&labelsMode=And", &[0, 3]),
    ("?labels={A}&labels={R}&labelsMode=Or", &[0, 1, 3, 5, 9]),
    ("?labels={F}&labels={L}", &[4]),
    ("?labels={A}&labels={A}", &[0, 1, 3, 9]),
    ("?excludedLabels={R}", &[1, 2, 4, 6, 7, 8, 9, 10, 11]),
    (
        "?excludedLabels={A}&excludedLabels={R}",
        &[2, 4, 6, 7, 8, 10, 11],
    ),
    ("?labels={F}&excludedLabels={I}", &[3, 4, 9]),
    (
        "?labels={M}&labels={Z}&labelsMode=Or&excludedLabels={S}",
        &[7, 11],
    ),
];

#[tokio::test]
async fn label_facets_narrow_the_catalog() {
    let app = TestApp::new().await;
    let books = app.seed_facet_corpus().await;

    for (template, expected) in LABEL_CASES {
        let query = template
            .replace("{A}", &ACTION.to_string())
            .replace("{R}", &ROMANCE.to_string())
            .replace("{F}", &FANTASY.to_string())
            .replace("{I}", &ISEKAI.to_string())
            .replace("{M}", &MAFIA.to_string())
            .replace("{Z}", &ZOMBIES.to_string())
            .replace("{S}", &SCHOOL_LIFE.to_string())
            .replace("{L}", &LONG_STRIP.to_string());

        let got = app.get_books_page(&format!("/books{query}")).await;

        assert_eq!(
            sorted(ids(&got.data)),
            sorted(pick(&books, expected)),
            "{template}"
        );
    }
}

#[tokio::test]
async fn get_books_with_an_unknown_label_id_returns_no_matches() {
    let app = TestApp::new().await;
    app.seed_facet_corpus().await;

    let got = app
        .get_books_page(&format!("/books?labels={}", uuid::Uuid::now_v7()))
        .await;

    assert!(
        got.data.is_empty(),
        "a stale picker must degrade to an empty list, not an error"
    );
    assert!(got.next_cursor.is_none());
}

#[tokio::test]
async fn enum_facets_or_within_and_and_between() {
    let app = TestApp::new().await;
    let books = app.seed_facet_corpus().await;

    let cases: Vec<(String, Vec<usize>)> = vec![
        ("".to_owned(), (0..12).collect()),
        ("?kind=Manhwa".to_owned(), vec![2, 3, 7, 10]),
        (
            "?kind=Manga&kind=Manhua".to_owned(),
            vec![0, 1, 4, 5, 6, 8, 9, 11],
        ),
        ("?status=Completed".to_owned(), vec![1, 3, 8, 11]),
        (
            "?status=Ongoing&status=Hiatus".to_owned(),
            vec![0, 2, 4, 6, 7, 10],
        ),
        ("?publicationDemographic=Seinen".to_owned(), vec![2, 6, 11]),
        (
            "?publicationDemographic=Shounen&publicationDemographic=Kids".to_owned(),
            vec![0, 1, 5, 9, 10],
        ),
        (
            format!("?contentRating={}", CONTENT_RATINGS[0].id),
            vec![0, 1, 8],
        ),
        (
            format!(
                "?contentRating={}&contentRating={}",
                CONTENT_RATINGS[0].id, CONTENT_RATINGS[3].id
            ),
            vec![0, 1, 6, 7, 8, 11],
        ),
        (
            format!("?publicationLanguage={}", LANGUAGES[0].id),
            vec![0, 1, 10],
        ),
        ("?kind=Manhwa&status=Completed".to_owned(), vec![3]),
        (
            "?kind=Manga&publicationDemographic=Shounen".to_owned(),
            vec![0, 1],
        ),
        (
            "?kind=Manhwa&kind=Manhua&status=Cancelled".to_owned(),
            vec![5],
        ),
        (
            "?kind=Manga&publicationDemographic=Josei".to_owned(),
            vec![],
        ),
        (
            format!("?kind=Manhwa&labels={ACTION}&publicationDemographic=Josei"),
            vec![3],
        ),
    ];

    for (query, expected) in cases {
        let got = app.get_books_page(&format!("/books{query}")).await;

        assert_eq!(
            sorted(ids(&got.data)),
            sorted(pick(&books, &expected)),
            "{query:?}"
        );
    }
}

#[tokio::test]
async fn available_translated_language_answers_what_can_be_read() {
    let app = TestApp::new().await;
    let books = app.seed_translated_corpus().await;
    let english = LANGUAGES[3].id;
    let russian = LANGUAGES[4].id;

    let cases: Vec<(String, Vec<usize>)> = vec![
        (
            format!("?availableTranslatedLanguage={english}"),
            vec![0, 1],
        ),
        (format!("?availableTranslatedLanguage={russian}"), vec![2]),
        (
            format!("?availableTranslatedLanguage={english}&availableTranslatedLanguage={russian}"),
            vec![0, 1, 2],
        ),
        (
            format!(
                "?availableTranslatedLanguage={english}&publicationLanguage={}",
                LANGUAGES[0].id
            ),
            vec![0, 1],
        ),
    ];

    for (query, expected) in cases {
        let got = app.get_books_page(&format!("/books{query}")).await;

        assert_eq!(
            sorted(ids(&got.data)),
            sorted(pick(&books, &expected)),
            "{query:?}"
        );
    }

    let got = app
        .get_books_page(&format!("/books?availableTranslatedLanguage={english}"))
        .await;

    assert_eq!(
        ids(&got.data)
            .iter()
            .filter(|id| **id == books[1].id)
            .count(),
        1,
        "two releases in one language must not multiply the book"
    );
    assert!(
        !ids(&got.data).contains(&books[3].id),
        "a matching publication language is not a translation"
    );
    assert!(
        !ids(&got.data).contains(&books[5].id),
        "a hidden book's releases must not make it discoverable"
    );
}

#[tokio::test]
async fn range_facets_respect_their_boundaries() {
    let app = TestApp::new().await;
    let books = app.seed_range_corpus().await;

    let cases: Vec<(String, Vec<usize>)> = vec![
        (
            "?publicationYearFrom=2012&publicationYearTo=2015".to_owned(),
            vec![1, 2],
        ),
        ("?publicationYearFrom=2015".to_owned(), vec![2, 3, 4]),
        ("?publicationYearTo=2012".to_owned(), vec![0, 1]),
        (
            "?publicationYearFrom=2012&publicationYearTo=2012".to_owned(),
            vec![1],
        ),
        (
            format!(
                "?createdAtFrom={}&createdAtTo={}",
                rfc3339(day(12)),
                rfc3339(day(18))
            ),
            vec![1, 2],
        ),
        (
            format!("?createdAtFrom={}", rfc3339(day(15))),
            vec![2, 3, 4],
        ),
        (format!("?createdAtTo={}", rfc3339(day(15))), vec![0, 1]),
        (
            format!(
                "?createdAtFrom={}&createdAtTo={}",
                rfc3339(day(15)),
                rfc3339(day(15))
            ),
            vec![],
        ),
    ];

    for (query, expected) in cases {
        let got = app.get_books_page(&format!("/books{query}")).await;

        assert_eq!(
            sorted(ids(&got.data)),
            sorted(pick(&books, &expected)),
            "{query:?}"
        );
    }
}

#[tokio::test]
async fn get_books_with_an_unmatched_filter_returns_an_empty_page() {
    let app = TestApp::new().await;
    app.seed_facet_corpus().await;

    let (status, body) = app
        .get_body("/books?kind=Manga&publicationDemographic=Josei")
        .await;
    assert_eq!(status, StatusCode::OK);

    let v: serde_json::Value = serde_json::from_str(&body).unwrap();

    assert_eq!(v["data"].as_array().unwrap().len(), 0);
    assert!(v["nextCursor"].is_null());
}

#[tokio::test]
async fn filtering_stays_inside_the_default_visibility() {
    let app = TestApp::new().await;
    let hidden: BookQuery = BookFaker {
        exact_labels: Some(LABELS.to_vec()),
        visibility: BookVisibility::Hidden,
        publication_year: Some(2020),
        ..Default::default()
    }
    .fake();
    app.insert_book(&hidden).await;

    for query in [
        format!("?labels={ACTION}"),
        format!("?labels={ACTION}&labelsMode=Or"),
        format!("?excludedLabels={}", uuid::Uuid::now_v7()),
        "?publicationYearFrom=2020&publicationYearTo=2020".to_owned(),
        "?sortField=Name&order=Asc".to_owned(),
        format!("?kind={}", hidden.kind.as_ref()),
    ] {
        let got = app.get_books_page(&format!("/books{query}")).await;

        assert!(
            !ids(&got.data).contains(&hidden.id),
            "{query} must not surface a book outside Listed"
        );
    }
}

// 400 means the query string could not be deserialized; 422 means it parsed and then failed a
// domain rule. The split is the contract, so these assert status and nothing else.
#[tokio::test]
async fn get_books_rejects_malformed_and_out_of_range_queries() {
    let app = TestApp::new().await;

    let too_many: String = (0..=200)
        .map(|_| format!("&contentRating={}", uuid::Uuid::now_v7()))
        .collect();

    let cases: Vec<(String, StatusCode)> = vec![
        (
            "?publicationYearFrom=2020&publicationYearTo=2010".to_owned(),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            format!(
                "?createdAtFrom={}&createdAtTo={}",
                rfc3339(day(20)),
                rfc3339(day(10))
            ),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        ("?limit=0".to_owned(), StatusCode::UNPROCESSABLE_ENTITY),
        ("?limit=101".to_owned(), StatusCode::UNPROCESSABLE_ENTITY),
        (format!("?a=1{too_many}"), StatusCode::UNPROCESSABLE_ENTITY),
        (
            "?createdAtFrom=yesterday".to_owned(),
            StatusCode::BAD_REQUEST,
        ),
        (
            "?publicationYearFrom=recent".to_owned(),
            StatusCode::BAD_REQUEST,
        ),
        ("?kind=Manwha".to_owned(), StatusCode::BAD_REQUEST),
        ("?status=Abandoned".to_owned(), StatusCode::BAD_REQUEST),
        (
            "?publicationDemographic=Unknown".to_owned(),
            StatusCode::BAD_REQUEST,
        ),
        ("?sortField=rating".to_owned(), StatusCode::BAD_REQUEST),
        ("?order=Ascending".to_owned(), StatusCode::BAD_REQUEST),
        ("?labels=not-a-uuid".to_owned(), StatusCode::BAD_REQUEST),
        ("?labelsMode=BOTH".to_owned(), StatusCode::BAD_REQUEST),
        ("?cursor=not%20base64".to_owned(), StatusCode::BAD_REQUEST),
        ("?limit=abc".to_owned(), StatusCode::BAD_REQUEST),
    ];

    for (query, expected) in cases {
        let req = Request::get(format!("/books{query}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.router.clone().oneshot(req).await.unwrap();

        assert_eq!(resp.status(), expected, "{query:?}");
    }
}

#[tokio::test]
async fn get_books_accepts_duplicates_that_fit_after_deduping() {
    let app = TestApp::new().await;

    let repeated: String = (0..50).map(|_| "&kind=Manga".to_owned()).collect();

    let req = Request::get(format!("/books?a=1{repeated}"))
        .body(Body::empty())
        .unwrap();
    let resp = app.router.oneshot(req).await.unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}
