use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;

use crate::helpers::fakers::{
    ACTION, BookFaker, CONTENT_RATINGS, FANTASY, ISEKAI, LABELS, LANGUAGES, LONG_STRIP, MAFIA,
    ROMANCE, SCHOOL_LIFE, ZOMBIES,
};
use crate::helpers::{
    RespWrapper, TestApp, assert_error, creator_keys, day, ids, label_keys, link_keys, pick,
    rfc3339, sorted, title_keys,
};
use fake::Fake;
use manga_theka::entity::{BookQuery, BookVisibility};

const FACET_CORPUS_LABEL_CASES: &[(&str, &[usize])] = &[
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

const SORT_CORPUS_CASES: &[(&str, &[usize])] = &[
    ("", &[0, 2, 3, 1]),
    ("?sortField=CreatedAt&order=Asc", &[1, 3, 2, 0]),
    ("?sortField=CreatedAt&order=Desc", &[0, 2, 3, 1]),
    ("?sortField=Name&order=Asc", &[0, 1, 2, 3]),
    ("?sortField=Name&order=Desc", &[3, 2, 1, 0]),
    ("?sortField=PublicationYear&order=Asc", &[3, 1, 2, 0]),
    ("?sortField=PublicationYear&order=Desc", &[0, 2, 1, 3]),
];

#[tokio::test]
async fn label_facets_narrow_the_catalog() {
    let app = TestApp::new().await;
    let books = app.seed_facet_corpus().await;

    for (template, expected) in FACET_CORPUS_LABEL_CASES {
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
async fn enum_facets_or_within_a_facet_and_between_facets() {
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
    let japanese = LANGUAGES[0].id;

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
            format!("?availableTranslatedLanguage={english}&publicationLanguage={japanese}"),
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
async fn get_books_embeds_each_books_own_arrays() {
    let app = TestApp::new().await;
    let bare: BookQuery = BookFaker {
        labels: 0..=0,
        links: 0..=0,
        titles: 0..=0,
        creators: 0..=0,
        ..Default::default()
    }
    .fake();

    let full: BookQuery = BookFaker {
        labels: 1..=5,
        links: 1..=5,
        titles: 1..=5,
        creators: 1..=5,
        ..Default::default()
    }
    .fake();

    app.insert_book(&bare).await;
    app.insert_book(&full).await;

    let req = Request::get("/books").body(Body::empty()).unwrap();
    let resp = app.send(req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let listed: RespWrapper<Vec<BookQuery>> = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(listed.data.len(), 2);

    let got_full = listed.data.iter().find(|b| b.id == full.id).unwrap();
    assert_eq!(
        label_keys(got_full.labels.as_slice()),
        label_keys(full.labels.as_slice())
    );
    assert_eq!(
        link_keys(got_full.links.as_slice()),
        link_keys(full.links.as_slice())
    );
    assert_eq!(
        title_keys(got_full.titles.as_slice()),
        title_keys(full.titles.as_slice())
    );
    assert_eq!(
        creator_keys(got_full.creators.as_slice()),
        creator_keys(full.creators.as_slice())
    );

    let got_bare = listed.data.iter().find(|b| b.id == bare.id).unwrap();
    assert_eq!(got_bare.content_rating, bare.content_rating);
    assert!(got_bare.labels.is_empty());
    assert!(got_bare.links.is_empty());
    assert!(got_bare.titles.is_empty());
    assert!(got_bare.creators.is_empty());
}

#[tokio::test]
async fn get_books_returns_default_limit_and_next_cursor() {
    let app = TestApp::new().await;

    for _ in 0..25 {
        let book: BookQuery = BookFaker::default().fake();
        app.insert_book(&book).await;
    }

    let req = Request::get("/books").body(Body::empty()).unwrap();
    let resp = app.send(req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let listed: RespWrapper<Vec<BookQuery>, String> = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(listed.data.len(), 20);
    assert!(listed.next_cursor.is_some());
}

#[tokio::test]
async fn get_books_with_cursor_and_limit_3_returns_next_page() {
    let app = TestApp::new().await;

    for _ in 0..6 {
        let book: BookQuery = BookFaker::default().fake();
        app.insert_book(&book).await;
    }

    let first = app.get_books_page("/books?limit=3").await;
    let first_ids = ids(&first.data);
    let cursor = first.next_cursor.expect("a full page must carry a cursor");

    let second = app
        .get_books_page(&format!("/books?limit=3&cursor={cursor}"))
        .await;
    let second_ids = ids(&second.data);

    assert_eq!(second_ids.len(), 3);
    assert!(second_ids.iter().all(|id| !first_ids.contains(id)));
    assert!(second.next_cursor.is_none());
}

#[tokio::test]
async fn get_books_desc_order_confirmed() {
    let app = TestApp::new().await;

    let mut ids = Vec::with_capacity(3);
    for _ in 0..3 {
        let book: BookQuery = BookFaker::default().fake();
        app.insert_book(&book).await;
        ids.push(book.id);
    }
    ids.sort_by(|a, b| b.cmp(a));

    let req = Request::get("/books").body(Body::empty()).unwrap();
    let resp = app.send(req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let listed: RespWrapper<Vec<BookQuery>> = serde_json::from_slice(&bytes).unwrap();
    let returned_ids: Vec<uuid::Uuid> = listed.data.iter().map(|b| b.id).collect();

    assert_eq!(returned_ids, ids);
}

#[tokio::test]
async fn get_books_zero_limit_returns_422() {
    let app = TestApp::new().await;

    let req = Request::get("/books?limit=0").body(Body::empty()).unwrap();
    let resp = app.send(req).await;

    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;
}

#[tokio::test]
async fn get_books_invalid_cursor_returns_400() {
    let app = TestApp::new().await;

    let req = Request::get("/books?cursor=not-a-cursor!!")
        .body(Body::empty())
        .unwrap();
    let resp = app.send(req).await;

    assert_error(resp, StatusCode::BAD_REQUEST).await;
}

#[tokio::test]
async fn each_sort_orders_by_its_own_key() {
    let app = TestApp::new().await;
    let books = app.seed_sort_corpus().await;

    for (query, expected) in SORT_CORPUS_CASES {
        let got = app.get_books_page(&format!("/books{query}")).await;

        assert_eq!(ids(&got.data), pick(&books, expected), "{query:?}");
    }
}

#[tokio::test]
async fn paging_a_non_unique_sort_key_neither_skips_nor_repeats() {
    let app = TestApp::new().await;
    let books = app.seed_tie_corpus(6).await;
    let ascending = ids(&books);
    let descending: Vec<uuid::Uuid> = ascending.iter().rev().copied().collect();

    for sort in ["CreatedAt", "Name", "PublicationYear"] {
        for (order, expected) in [("Asc", &ascending), ("Desc", &descending)] {
            let base = format!("/books?sortField={sort}&order={order}&limit=2");
            let mut seen: Vec<uuid::Uuid> = Vec::new();
            let mut path = base.clone();

            for page in 0..3 {
                let got = app.get_books_page(&path).await;
                assert_eq!(got.data.len(), 2, "{sort} {order} page {page}");
                seen.extend(ids(&got.data));

                match got.next_cursor {
                    Some(cursor) => {
                        assert!(
                            page < 2,
                            "{sort} {order}: the last page must carry no cursor"
                        );
                        path = format!("{base}&cursor={cursor}");
                    }
                    None => assert_eq!(page, 2, "{sort} {order}: paging stopped early"),
                }
            }

            assert_eq!(
                &seen, expected,
                "{sort} {order}: books tied on the sort key must page in id order, each once"
            );
        }
    }
}

#[tokio::test]
async fn get_books_on_the_last_page_returns_a_null_cursor() {
    let app = TestApp::new().await;
    app.seed_tie_corpus(2).await;

    let (status, body) = app.get_body("/books?limit=5").await;
    assert_eq!(status, StatusCode::OK);

    let v: serde_json::Value = serde_json::from_str(&body).unwrap();

    assert!(
        v["nextCursor"].is_null(),
        "the key must be present and null, not absent"
    );
}

#[tokio::test]
async fn a_cursor_carries_its_filter_across_pages() {
    let app = TestApp::new().await;
    let books = app.seed_facet_corpus().await;

    let first = app
        .get_books_page(&format!("/books?labels={ACTION}&limit=2"))
        .await;
    let cursor = first.next_cursor.clone().expect("a full page has a cursor");

    let second = app
        .get_books_page(&format!("/books?labels={ACTION}&limit=2&cursor={cursor}"))
        .await;

    let mut seen = ids(&first.data);
    seen.extend(ids(&second.data));

    assert_eq!(
        sorted(seen),
        sorted(pick(&books, &[0, 1, 3, 9])),
        "page two of a filtered list must still be filtered"
    );
}

#[tokio::test]
async fn a_cursor_minted_under_another_filter_returns_400() {
    let app = TestApp::new().await;
    app.seed_facet_corpus().await;

    let first = app
        .get_books_page(&format!("/books?labels={ACTION}&limit=2"))
        .await;
    let cursor = first.next_cursor.expect("a full page has a cursor");

    for changed in [
        format!("/books?labels={ROMANCE}&limit=2&cursor={cursor}"),
        format!("/books?labels={ACTION}&limit=2&sortField=Name&cursor={cursor}"),
        format!("/books?labels={ACTION}&limit=2&order=Asc&cursor={cursor}"),
        format!("/books?labels={ACTION}&kind=Manga&limit=2&cursor={cursor}"),
        format!("/books?limit=2&cursor={cursor}"),
    ] {
        let req = Request::get(&changed).body(Body::empty()).unwrap();
        let resp = app.send(req).await;

        assert_error(resp, StatusCode::BAD_REQUEST).await;
    }
}

#[tokio::test]
async fn a_cursor_survives_a_changed_page_size() {
    let app = TestApp::new().await;
    app.seed_facet_corpus().await;

    let first = app.get_books_page("/books?limit=2").await;
    let cursor = first.next_cursor.expect("a full page has a cursor");

    let second = app
        .get_books_page(&format!("/books?limit=5&cursor={cursor}"))
        .await;

    assert_eq!(
        second.data.len(),
        5,
        "limit is deliberately outside the filter hash"
    );
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
        let resp = app.send(req).await;

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
    let resp = app.send(req).await;

    assert_eq!(resp.status(), StatusCode::OK);
}
