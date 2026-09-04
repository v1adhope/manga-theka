use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use http_body_util::BodyExt;
use tower::ServiceExt;

use crate::fakers::{ACTION, BookFaker, CONTENT_RATINGS, CreatorFaker, LabelFaker, ROMANCE};
use crate::helpers::{
    RespWrapper, TestApp, assert_error, assert_stored, creator_keys, ids, label_keys, link_keys,
    pick, sorted, title_keys,
};
use fake::Fake;
use manga_theka::entity::{BookQuery, Creator, Label};

#[tokio::test]
async fn store_book_with_valid_body_passes() {
    let app = TestApp::new().await;
    let book: BookQuery = BookFaker {
        labels: 1..=5,
        links: 1..=5,
        titles: 1..=5,
        ..Default::default()
    }
    .fake();
    let creator: Creator = CreatorFaker.fake();
    app.insert_creator(&creator).await;

    let body = serde_json::json!({
        "name": book.name.as_ref(),
        "description": book.description.as_ref(),
        "publicationYear": book.publication_year,
        "contentRatingId": book.content_rating.id,
        "status": book.status.as_ref(),
        "kind": book.kind.as_ref(),
        "publicationLanguageId": book.publication_language.id,
        "publicationDemographic": book.publication_demographic.as_ref(),
        "labelIds": book.labels.as_slice().iter().map(|l| l.id).collect::<Vec<_>>(),
        "links": &book.links,
        "titles": &book.titles,
        "creators": [{ "creatorId": creator.id, "role": "Author" }],
    })
    .to_string();

    let req = Request::post("/books")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.router.clone().oneshot(req).await.unwrap();
    let id = assert_stored(resp).await;

    let got = app.fetch_book(id).await;

    assert_eq!(got.name, book.name);
    assert_eq!(got.description, book.description);
    assert_eq!(got.publication_year, book.publication_year);
    assert_eq!(got.content_rating, book.content_rating);
    assert_eq!(got.status, book.status);
    assert_eq!(got.kind, book.kind);
    assert_eq!(got.publication_language.id, book.publication_language.id);
    assert_eq!(got.publication_demographic, book.publication_demographic);
    assert_eq!(
        label_keys(got.labels.as_slice()),
        label_keys(book.labels.as_slice())
    );
    assert_eq!(
        link_keys(got.links.as_slice()),
        link_keys(book.links.as_slice())
    );
    assert_eq!(
        title_keys(got.titles.as_slice()),
        title_keys(book.titles.as_slice())
    );
    assert_eq!(
        creator_keys(got.creators.as_slice()),
        vec![(
            creator.id,
            creator.first_name.as_ref(),
            creator.last_name.as_ref(),
            vec!["Author"]
        )]
    );
    assert!(got.updated_at.is_none());
    assert_ne!(got.created_at, time::OffsetDateTime::UNIX_EPOCH);
}

#[tokio::test]
async fn store_book_without_relations_passes() {
    let app = TestApp::new().await;
    let book: BookQuery = BookFaker::default().fake();

    let body = serde_json::json!({
        "name": book.name.as_ref(),
        "description": book.description.as_ref(),
        "publicationYear": book.publication_year,
        "contentRatingId": book.content_rating.id,
        "status": book.status.as_ref(),
        "kind": book.kind.as_ref(),
        "publicationLanguageId": book.publication_language.id,
        "publicationDemographic": book.publication_demographic.as_ref(),
    })
    .to_string();

    let req = Request::post("/books")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.router.clone().oneshot(req).await.unwrap();
    let id = assert_stored(resp).await;

    let sample = app.fetch_book_sample(id).await;

    assert_eq!(sample.name.as_deref(), Some(book.name.as_ref()));
    assert_eq!(sample.labels, 0);
    assert_eq!(sample.links, 0);
    assert_eq!(sample.titles, 0);
}

#[tokio::test]
async fn store_book_with_broken_json_returns_400() {
    let app = TestApp::new().await;

    let req = Request::post("/books")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{not json"))
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();
    assert_error(resp, StatusCode::BAD_REQUEST).await;
}

#[tokio::test]
async fn store_book_with_missing_name_returns_422() {
    let app = TestApp::new().await;
    let book: BookQuery = BookFaker::default().fake();

    let body = serde_json::json!({
        "description": book.description.as_ref(),
        "publicationYear": book.publication_year,
        "contentRatingId": book.content_rating.id,
        "status": book.status.as_ref(),
        "kind": book.kind.as_ref(),
        "publicationLanguageId": book.publication_language.id,
        "publicationDemographic": book.publication_demographic.as_ref(),
    })
    .to_string();

    let req = Request::post("/books")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();
    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;
}

#[tokio::test]
async fn store_book_with_unknown_status_returns_422() {
    let app = TestApp::new().await;
    let book: BookQuery = BookFaker::default().fake();

    let body = serde_json::json!({
        "name": book.name.as_ref(),
        "description": book.description.as_ref(),
        "publicationYear": book.publication_year,
        "contentRatingId": book.content_rating.id,
        "status": "abandoned",
        "kind": book.kind.as_ref(),
        "publicationLanguageId": book.publication_language.id,
        "publicationDemographic": book.publication_demographic.as_ref(),
    })
    .to_string();

    let req = Request::post("/books")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();
    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;
}

#[tokio::test]
async fn store_book_with_unknown_publication_demographic_returns_422() {
    let app = TestApp::new().await;
    let book: BookQuery = BookFaker::default().fake();

    let body = serde_json::json!({
        "name": book.name.as_ref(),
        "description": book.description.as_ref(),
        "publicationYear": book.publication_year,
        "contentRatingId": book.content_rating.id,
        "status": book.status.as_ref(),
        "kind": book.kind.as_ref(),
        "publicationLanguageId": book.publication_language.id,
        "publicationDemographic": "Unknown",
    })
    .to_string();

    let req = Request::post("/books")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();
    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;
}

#[tokio::test]
async fn store_book_without_publication_demographic_returns_422() {
    let app = TestApp::new().await;
    let book: BookQuery = BookFaker::default().fake();

    let body = serde_json::json!({
        "name": book.name.as_ref(),
        "description": book.description.as_ref(),
        "publicationYear": book.publication_year,
        "contentRatingId": book.content_rating.id,
        "status": book.status.as_ref(),
        "kind": book.kind.as_ref(),
        "publicationLanguageId": book.publication_language.id,
    })
    .to_string();

    let req = Request::post("/books")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();
    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;
}

#[tokio::test]
async fn store_book_with_unknown_label_id_returns_422() {
    let app = TestApp::new().await;
    let book: BookQuery = BookFaker::default().fake();

    let body = serde_json::json!({
        "name": book.name.as_ref(),
        "description": book.description.as_ref(),
        "publicationYear": book.publication_year,
        "contentRatingId": book.content_rating.id,
        "status": book.status.as_ref(),
        "kind": book.kind.as_ref(),
        "publicationLanguageId": book.publication_language.id,
        "publicationDemographic": book.publication_demographic.as_ref(),
        "labelIds": [uuid::Uuid::now_v7()],
    })
    .to_string();

    let req = Request::post("/books")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();
    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;

    let count = sqlx::query_scalar!(r#"select count(*) as "count!" from books"#)
        .fetch_one(&app.pool)
        .await
        .unwrap();

    assert_eq!(count, 0, "failed write must roll back book");
}

#[tokio::test]
async fn store_book_with_unknown_creator_id_returns_422() {
    let app = TestApp::new().await;
    let book: BookQuery = BookFaker::default().fake();

    let body = serde_json::json!({
        "name": book.name.as_ref(),
        "description": book.description.as_ref(),
        "publicationYear": book.publication_year,
        "contentRatingId": book.content_rating.id,
        "status": book.status.as_ref(),
        "kind": book.kind.as_ref(),
        "publicationLanguageId": book.publication_language.id,
        "publicationDemographic": book.publication_demographic.as_ref(),
        "creators": [{ "creatorId": uuid::Uuid::now_v7(), "role": "Author" }],
    })
    .to_string();

    let req = Request::post("/books")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();
    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;

    let count = sqlx::query_scalar!(r#"select count(*) as "count!" from books"#)
        .fetch_one(&app.pool)
        .await
        .unwrap();

    assert_eq!(count, 0, "failed write must roll back book");
}

#[tokio::test]
async fn store_book_with_dual_role_creator_merges_into_one_credit() {
    let app = TestApp::new().await;
    let book: BookQuery = BookFaker::default().fake();
    let creator: Creator = CreatorFaker.fake();
    app.insert_creator(&creator).await;

    let body = serde_json::json!({
        "name": book.name.as_ref(),
        "description": book.description.as_ref(),
        "publicationYear": book.publication_year,
        "contentRatingId": book.content_rating.id,
        "status": book.status.as_ref(),
        "kind": book.kind.as_ref(),
        "publicationLanguageId": book.publication_language.id,
        "publicationDemographic": book.publication_demographic.as_ref(),
        "creators": [
            { "creatorId": creator.id, "role": "Author" },
            { "creatorId": creator.id, "role": "Artist" },
        ],
    })
    .to_string();

    let req = Request::post("/books")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.router.clone().oneshot(req).await.unwrap();
    let id = assert_stored(resp).await;

    let got = app.fetch_book(id).await;

    assert_eq!(
        creator_keys(got.creators.as_slice()),
        vec![(
            creator.id,
            creator.first_name.as_ref(),
            creator.last_name.as_ref(),
            vec!["Artist", "Author"]
        )],
        "a creator credited as both roles on one book must merge into a single entry"
    );
}

#[tokio::test]
async fn store_book_with_duplicate_creator_role_returns_422() {
    let app = TestApp::new().await;
    let book: BookQuery = BookFaker::default().fake();
    let creator: Creator = CreatorFaker.fake();
    app.insert_creator(&creator).await;

    let body = serde_json::json!({
        "name": book.name.as_ref(),
        "description": book.description.as_ref(),
        "publicationYear": book.publication_year,
        "contentRatingId": book.content_rating.id,
        "status": book.status.as_ref(),
        "kind": book.kind.as_ref(),
        "publicationLanguageId": book.publication_language.id,
        "publicationDemographic": book.publication_demographic.as_ref(),
        "creators": [
            { "creatorId": creator.id, "role": "Author" },
            { "creatorId": creator.id, "role": "Author" },
        ],
    })
    .to_string();

    let req = Request::post("/books")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();
    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;
}

#[tokio::test]
async fn get_book_with_valid_id_passes() {
    let app = TestApp::new().await;
    let book: BookQuery = BookFaker {
        labels: 1..=5,
        links: 1..=5,
        titles: 1..=5,
        creators: 1..=5,
        ..Default::default()
    }
    .fake();

    app.insert_book(&book).await;

    let req = Request::get(format!("/books/{}", book.id))
        .body(Body::empty())
        .unwrap();
    let resp = app.router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let wrapper: RespWrapper<BookQuery> = serde_json::from_slice(&bytes).unwrap();
    let got = wrapper.data;

    assert_eq!(got.id, book.id);
    assert_eq!(got.name, book.name);
    assert_eq!(got.description, book.description);
    assert_eq!(got.publication_year, book.publication_year);
    assert_eq!(got.content_rating, book.content_rating,);
    assert_eq!(got.status, book.status);
    assert_eq!(got.kind, book.kind);
    assert_eq!(got.publication_language.id, book.publication_language.id);
    assert_eq!(
        got.publication_language.code,
        book.publication_language.code,
    );
    assert_eq!(
        got.publication_language.name,
        book.publication_language.name
    );
    assert_eq!(got.publication_demographic, book.publication_demographic);
    assert_eq!(
        label_keys(got.labels.as_slice()),
        label_keys(book.labels.as_slice())
    );
    assert_eq!(
        link_keys(got.links.as_slice()),
        link_keys(book.links.as_slice())
    );
    assert_eq!(
        title_keys(got.titles.as_slice()),
        title_keys(book.titles.as_slice())
    );
    assert_eq!(
        creator_keys(got.creators.as_slice()),
        creator_keys(book.creators.as_slice())
    );
    assert_eq!(got.updated_at, book.updated_at);
    assert_eq!(
        got.created_at.unix_timestamp(),
        book.created_at.unix_timestamp()
    );
}

#[tokio::test]
async fn get_book_with_unknown_id_returns_404() {
    let app = TestApp::new().await;

    let req = Request::get(format!("/books/{}", uuid::Uuid::now_v7()))
        .body(Body::empty())
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();

    assert_error(resp, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn get_book_with_malformed_id_returns_400() {
    let app = TestApp::new().await;

    let req = Request::get("/books/not-a-uuid")
        .body(Body::empty())
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();
    assert_error(resp, StatusCode::BAD_REQUEST).await;
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
    let resp = app.router.oneshot(req).await.unwrap();
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
    let resp = app.router.oneshot(req).await.unwrap();
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
    let resp = app.router.oneshot(req).await.unwrap();
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
    let resp = app.router.oneshot(req).await.unwrap();

    assert_error(resp, StatusCode::UNPROCESSABLE_ENTITY).await;
}

#[tokio::test]
async fn get_books_invalid_cursor_returns_400() {
    let app = TestApp::new().await;

    let req = Request::get("/books?cursor=not-a-cursor!!")
        .body(Body::empty())
        .unwrap();
    let resp = app.router.oneshot(req).await.unwrap();

    assert_error(resp, StatusCode::BAD_REQUEST).await;
}

// Order is the assertion here, so these compare the exact sequence rather than a set.
const SORT_CASES: &[(&str, &[usize])] = &[
    ("", &[0, 2, 3, 1]),
    ("?sortField=CreatedAt&order=Asc", &[1, 3, 2, 0]),
    ("?sortField=CreatedAt&order=Desc", &[0, 2, 3, 1]),
    ("?sortField=Name&order=Asc", &[0, 1, 2, 3]),
    ("?sortField=Name&order=Desc", &[3, 2, 1, 0]),
    ("?sortField=PublicationYear&order=Asc", &[3, 1, 2, 0]),
    ("?sortField=PublicationYear&order=Desc", &[0, 2, 1, 3]),
];

#[tokio::test]
async fn each_sort_orders_by_its_own_key() {
    let app = TestApp::new().await;
    let books = app.seed_sort_corpus().await;

    for (query, expected) in SORT_CASES {
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
        let resp = app.router.clone().oneshot(req).await.unwrap();

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
async fn update_book_with_valid_body_passes() {
    let app = TestApp::new().await;
    let book: BookQuery = BookFaker {
        labels: 1..=5,
        links: 1..=5,
        titles: 1..=5,
        ..Default::default()
    }
    .fake();

    app.insert_book(&book).await;

    let updated: BookQuery = BookFaker {
        labels: 1..=5,
        links: 1..=5,
        titles: 1..=5,
        ..Default::default()
    }
    .fake();

    let body = serde_json::json!({
        "name": updated.name.as_ref(),
        "description": updated.description.as_ref(),
        "publicationYear": updated.publication_year,
        "contentRatingId": updated.content_rating.id,
        "status": updated.status.as_ref(),
        "kind": updated.kind.as_ref(),
        "publicationLanguageId": updated.publication_language.id,
        "publicationDemographic": updated.publication_demographic.as_ref(),
        "labelIds": updated.labels.as_slice().iter().map(|l| l.id).collect::<Vec<_>>(),
        "links": &updated.links,
        "titles": &updated.titles,
    })
    .to_string();

    let req = Request::put(format!("/books/{}", book.id))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();
    let resp = app.router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let got = app.fetch_book(book.id).await;

    assert_eq!(got.name, updated.name);
    assert_eq!(got.description, updated.description);
    assert_eq!(got.publication_year, updated.publication_year);
    assert_eq!(got.content_rating, updated.content_rating);
    assert_eq!(got.status, updated.status);
    assert_eq!(got.kind, updated.kind);
    assert_eq!(got.publication_language.id, updated.publication_language.id);
    assert_eq!(got.publication_demographic, updated.publication_demographic);

    assert_eq!(
        label_keys(got.labels.as_slice()),
        label_keys(updated.labels.as_slice()),
        "arrays must be replaced wholesale"
    );
    assert_eq!(
        link_keys(got.links.as_slice()),
        link_keys(updated.links.as_slice()),
        "arrays must be replaced wholesale"
    );
    assert_eq!(
        title_keys(got.titles.as_slice()),
        title_keys(updated.titles.as_slice()),
        "arrays must be replaced wholesale"
    );
    assert!(got.updated_at.is_some());
    assert_eq!(
        got.created_at.unix_timestamp(),
        book.created_at.unix_timestamp(),
        "created_at must stay immutable"
    );
}

#[tokio::test]
async fn update_book_swaps_the_content_rating() {
    let app = TestApp::new().await;
    let mut book: BookQuery = BookFaker::default().fake();
    book.content_rating = CONTENT_RATINGS
        .first()
        .expect("content rating to swap from must exist")
        .clone();

    app.insert_book(&book).await;

    let other = CONTENT_RATINGS
        .get(1)
        .expect("content rating to swap to must exist")
        .clone();

    let updated_body = serde_json::json!({
        "name": book.name.as_ref(),
        "description": book.description.as_ref(),
        "publicationYear": book.publication_year,
        "contentRatingId": other.id,
        "status": book.status.as_ref(),
        "kind": book.kind.as_ref(),
        "publicationLanguageId": book.publication_language.id,
        "publicationDemographic": book.publication_demographic.as_ref(),
    })
    .to_string();

    let req = Request::put(format!("/books/{}", book.id))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(updated_body))
        .unwrap();
    let resp = app.router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let got = app.fetch_book(book.id).await;

    assert_eq!(got.content_rating, other);
}

#[tokio::test]
async fn update_book_with_empty_arrays_detaches_everything() {
    let app = TestApp::new().await;
    let book: BookQuery = BookFaker {
        labels: 1..=5,
        links: 1..=5,
        titles: 1..=5,
        ..Default::default()
    }
    .fake();

    app.insert_book(&book).await;

    let detached = serde_json::json!({
        "name": book.name.as_ref(),
        "description": book.description.as_ref(),
        "publicationYear": book.publication_year,
        "contentRatingId": book.content_rating.id,
        "status": book.status.as_ref(),
        "kind": book.kind.as_ref(),
        "publicationLanguageId": book.publication_language.id,
        "publicationDemographic": book.publication_demographic.as_ref(),
        "labelIds": [],
        "links": [],
        "titles": [],
    })
    .to_string();

    let req = Request::put(format!("/books/{}", book.id))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(detached))
        .unwrap();
    let resp = app.router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let sample = app.fetch_book_sample(book.id).await;

    assert_eq!(sample.labels, 0);
    assert_eq!(sample.links, 0);
    assert_eq!(sample.titles, 0);
}

#[tokio::test]
async fn update_book_leaves_other_books_untouched() {
    let app = TestApp::new().await;
    let book: BookQuery = BookFaker::default().fake();
    let other: BookQuery = BookFaker::default().fake();

    app.insert_book(&book).await;
    app.insert_book(&other).await;

    let renamed: BookQuery = BookFaker::default().fake();
    let updated_body = serde_json::json!({
        "name": renamed.name.as_ref(),
        "description": book.description.as_ref(),
        "publicationYear": book.publication_year,
        "contentRatingId": book.content_rating.id,
        "status": book.status.as_ref(),
        "kind": book.kind.as_ref(),
        "publicationLanguageId": book.publication_language.id,
        "publicationDemographic": book.publication_demographic.as_ref(),
    })
    .to_string();

    let req = Request::put(format!("/books/{}", book.id))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(updated_body))
        .unwrap();
    let resp = app.router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let sample = app.fetch_book_sample(other.id).await;

    assert_eq!(sample.name.as_deref(), Some(other.name.as_ref()));
    assert!(sample.updated_at.is_none());
    assert_eq!(sample.labels, other.labels.len() as i64);
    assert_eq!(sample.links, other.links.len() as i64);
    assert_eq!(sample.titles, other.titles.len() as i64);
}

#[tokio::test]
async fn update_book_with_unknown_id_returns_404() {
    let app = TestApp::new().await;
    let book: BookQuery = BookFaker::default().fake();

    let body = serde_json::json!({
        "name": book.name.as_ref(),
        "description": book.description.as_ref(),
        "publicationYear": book.publication_year,
        "contentRatingId": book.content_rating.id,
        "status": book.status.as_ref(),
        "kind": book.kind.as_ref(),
        "publicationLanguageId": book.publication_language.id,
        "publicationDemographic": book.publication_demographic.as_ref(),
    })
    .to_string();

    let req = Request::put(format!("/books/{}", uuid::Uuid::now_v7()))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();
    assert_error(resp, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn delete_book_with_valid_id_passes() {
    let app = TestApp::new().await;
    let book: BookQuery = BookFaker {
        labels: 1..=5,
        links: 1..=5,
        titles: 1..=5,
        ..Default::default()
    }
    .fake();

    app.insert_book(&book).await;

    let req = Request::delete(format!("/books/{}", book.id))
        .body(Body::empty())
        .unwrap();

    let resp = app.router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let sample = app.fetch_book_sample(book.id).await;

    assert!(sample.name.is_none());
    assert_eq!(sample.labels, 0);
    assert_eq!(sample.links, 0);
    assert_eq!(sample.titles, 0);
}

#[tokio::test]
async fn delete_book_leaves_other_books_untouched() {
    let app = TestApp::new().await;
    let labels: Label = LabelFaker.fake();

    let mut book: BookQuery = BookFaker {
        labels: 0..=0,
        ..Default::default()
    }
    .fake();
    book.labels = vec![labels.clone()].try_into().unwrap();

    let mut other: BookQuery = BookFaker {
        labels: 0..=0,
        ..Default::default()
    }
    .fake();
    other.labels = vec![labels].try_into().unwrap();

    app.insert_book(&book).await;
    app.insert_book(&other).await;

    let req = Request::delete(format!("/books/{}", book.id))
        .body(Body::empty())
        .unwrap();
    let resp = app.router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let sample = app.fetch_book_sample(other.id).await;

    assert_eq!(sample.name.as_deref(), Some(other.name.as_ref()));
    assert_eq!(sample.labels, other.labels.len() as i64);
    assert_eq!(sample.links, other.links.len() as i64);
    assert_eq!(sample.titles, other.titles.len() as i64);
}

#[tokio::test]
async fn delete_book_with_unknown_id_returns_404() {
    let app = TestApp::new().await;

    let req = Request::delete(format!("/books/{}", uuid::Uuid::now_v7()))
        .body(Body::empty())
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();

    assert_error(resp, StatusCode::NOT_FOUND).await;
}
