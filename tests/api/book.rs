use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use http_body_util::BodyExt;
use tower::ServiceExt;

use crate::helpers::{RespWrapper, TestApp};
use manga_theka::entity::Book;

#[tokio::test]
async fn store_book_with_valid_body_passes() {
    let app = TestApp::new().await;
    let refs = app.book_refs().await;

    let body = TestApp::book_body(&refs).to_string();
    let req = Request::post("/books")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let id = uuid::Uuid::parse_str(v["data"]["id"].as_str().unwrap()).unwrap();

    let row = sqlx::query!(
        r#"
select id, name, description, publication_year, content_rating, status, kind,
       publication_language, author, artist, updated_at, created_at
from books
        "#
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();

    assert_eq!(row.id, id);
    assert_eq!(row.name, "Berserk");
    assert_eq!(
        row.description,
        "A wandering swordsman and his enormous sword."
    );
    assert_eq!(row.publication_year, 1989);
    assert_eq!(row.content_rating, refs.content_rating);
    assert_eq!(row.status, "Ongoing");
    assert_eq!(row.kind, "Manga");
    assert_eq!(row.publication_language, refs.language);
    assert_eq!(row.author, refs.author);
    assert_eq!(row.artist, refs.artist);
    assert!(row.updated_at.is_none());
    assert_ne!(row.created_at, time::OffsetDateTime::UNIX_EPOCH);
}

#[tokio::test]
async fn store_book_attaches_labels_links_and_titles() {
    let app = TestApp::new().await;
    let refs = app.book_refs().await;
    let label_ids = app.label_ids(2).await;

    let mut body = TestApp::book_body(&refs);
    body["labelIds"] = serde_json::json!(label_ids);
    body["links"] = serde_json::json!([
        { "type": "WhereToRead", "url": "https://example.com/read" },
        { "type": "Track", "url": "https://example.com/track" },
    ]);
    body["titles"] = serde_json::json!([
        { "languageId": refs.language, "name": "ベルセルク" },
        { "languageId": refs.other_language, "name": "Berserk (KR)" },
    ]);

    let id = app.insert_book(&body).await;

    let labels = sqlx::query_scalar!(
        "select label_id from book_labels where book_id = $1 order by label_id",
        id
    )
    .fetch_all(&app.pool)
    .await
    .unwrap();
    let mut expected_labels = label_ids.clone();
    expected_labels.sort();
    assert_eq!(labels, expected_labels);

    let links = sqlx::query!(
        "select kind, url from book_links where book_id = $1 order by url",
        id
    )
    .fetch_all(&app.pool)
    .await
    .unwrap();
    assert_eq!(links.len(), 2);
    assert_eq!(links[0].kind, "WhereToRead");
    assert_eq!(links[0].url, "https://example.com/read");
    assert_eq!(links[1].kind, "Track");
    assert_eq!(links[1].url, "https://example.com/track");

    let titles = sqlx::query!(
        "select language_id, name from book_titles where book_id = $1 order by name",
        id
    )
    .fetch_all(&app.pool)
    .await
    .unwrap();
    assert_eq!(titles.len(), 2);
    assert_eq!(titles[0].name, "Berserk (KR)");
    assert_eq!(titles[0].language_id, refs.other_language);
    assert_eq!(titles[1].name, "ベルセルク");
    assert_eq!(titles[1].language_id, refs.language);
}

#[tokio::test]
async fn store_book_ignores_a_cover_image_field() {
    let app = TestApp::new().await;
    let refs = app.book_refs().await;

    let mut body = TestApp::book_body(&refs);
    body["cover"] = serde_json::json!("aGVsbG8=");

    let id = app.insert_book(&body).await;

    let count = sqlx::query_scalar!("select count(*) from books where id = $1", id)
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(count, Some(1));
}

#[tokio::test]
async fn get_book_embeds_labels_links_and_titles() {
    let app = TestApp::new().await;
    let refs = app.book_refs().await;
    let label_ids = app.label_ids(1).await;

    let mut body = TestApp::book_body(&refs);
    body["labelIds"] = serde_json::json!(label_ids);
    body["links"] = serde_json::json!([
        { "type": "WhereToBuy", "url": "https://example.com/buy" },
    ]);
    body["titles"] = serde_json::json!([
        { "languageId": refs.language, "name": "ベルセルク" },
    ]);

    let id = app.insert_book(&body).await;

    let req = Request::get(format!("/books/{id}"))
        .body(Body::empty())
        .unwrap();
    let resp = app.router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let data = &v["data"];

    assert_eq!(data["id"], id.to_string());
    assert_eq!(data["name"], "Berserk");
    assert_eq!(data["publicationYear"], 1989);
    assert_eq!(data["status"], "Ongoing");
    assert_eq!(data["type"], "Manga");
    assert_eq!(data["contentRating"], refs.content_rating.to_string());
    assert_eq!(data["publicationLanguage"], refs.language.to_string());
    assert_eq!(data["author"], refs.author.to_string());
    assert_eq!(data["artist"], refs.artist.to_string());
    assert!(data["updatedAt"].is_null());

    let labels = data["labels"].as_array().unwrap();
    assert_eq!(labels.len(), 1);
    assert_eq!(labels[0]["id"], label_ids[0].to_string());

    let links = data["links"].as_array().unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0]["type"], "WhereToBuy");
    assert_eq!(links[0]["url"], "https://example.com/buy");
    assert!(uuid::Uuid::parse_str(links[0]["id"].as_str().unwrap()).is_ok());

    let titles = data["titles"].as_array().unwrap();
    assert_eq!(titles.len(), 1);
    assert_eq!(titles[0]["name"], "ベルセルク");
    assert_eq!(titles[0]["languageId"], refs.language.to_string());
    assert!(uuid::Uuid::parse_str(titles[0]["id"].as_str().unwrap()).is_ok());
}

/// Posts `body` and asserts it is rejected as semantically invalid.
async fn assert_store_book_returns_422(app: &TestApp, body: serde_json::Value) {
    let req = Request::post("/books")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();

    let resp = app.router.clone().oneshot(req).await.unwrap();
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

#[tokio::test]
async fn store_book_with_broken_json_returns_400() {
    let app = TestApp::new().await;

    let req = Request::post("/books")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{not json"))
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn store_book_with_missing_name_returns_422() {
    let app = TestApp::new().await;
    let refs = app.book_refs().await;

    let mut body = TestApp::book_body(&refs);
    body.as_object_mut().unwrap().remove("name");

    assert_store_book_returns_422(&app, body).await;
}

#[tokio::test]
async fn store_book_with_unknown_status_returns_422() {
    let app = TestApp::new().await;
    let refs = app.book_refs().await;

    let mut body = TestApp::book_body(&refs);
    body["status"] = serde_json::json!("abandoned");

    assert_store_book_returns_422(&app, body).await;
}

#[tokio::test]
async fn store_book_with_unknown_type_returns_422() {
    let app = TestApp::new().await;
    let refs = app.book_refs().await;

    let mut body = TestApp::book_body(&refs);
    body["type"] = serde_json::json!("webtoon");

    assert_store_book_returns_422(&app, body).await;
}

#[tokio::test]
async fn store_book_with_name_over_255_characters_returns_422() {
    let app = TestApp::new().await;
    let refs = app.book_refs().await;

    let mut body = TestApp::book_body(&refs);
    body["name"] = serde_json::json!("a".repeat(256));

    assert_store_book_returns_422(&app, body).await;
}

#[tokio::test]
async fn store_book_with_description_over_2000_characters_returns_422() {
    let app = TestApp::new().await;
    let refs = app.book_refs().await;

    let mut body = TestApp::book_body(&refs);
    body["description"] = serde_json::json!("a".repeat(2001));

    assert_store_book_returns_422(&app, body).await;
}

#[tokio::test]
async fn store_book_with_more_than_12_titles_returns_422() {
    let app = TestApp::new().await;
    let refs = app.book_refs().await;

    let titles: Vec<serde_json::Value> = (0..13)
        .map(|i| serde_json::json!({ "languageId": refs.language, "name": format!("Title {i}") }))
        .collect();

    let mut body = TestApp::book_body(&refs);
    body["titles"] = serde_json::json!(titles);

    assert_store_book_returns_422(&app, body).await;

    let count = sqlx::query_scalar!("select count(*) from books")
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(count, Some(0), "the book must not be written at all");
}

/// The trigger backstops any write path that skips request validation.
#[tokio::test]
async fn book_titles_trigger_rejects_a_thirteenth_row() {
    let app = TestApp::new().await;
    let refs = app.book_refs().await;
    let id = app.insert_book(&TestApp::book_body(&refs)).await;

    for i in 0..12 {
        sqlx::query!(
            "insert into book_titles(id, book_id, language_id, name) values($1, $2, $3, $4)",
            uuid::Uuid::now_v7(),
            id,
            refs.language,
            format!("Title {i}")
        )
        .execute(&app.pool)
        .await
        .expect("the first 12 titles must be accepted");
    }

    let res = sqlx::query!(
        "insert into book_titles(id, book_id, language_id, name) values($1, $2, $3, $4)",
        uuid::Uuid::now_v7(),
        id,
        refs.language,
        "Title 13"
    )
    .execute(&app.pool)
    .await;

    let err = res.expect_err("the 13th title must be rejected");
    assert_eq!(
        err.as_database_error().unwrap().constraint(),
        Some("check_count_book_titles_book_id")
    );
}

#[tokio::test]
async fn store_book_with_unknown_link_type_returns_422() {
    let app = TestApp::new().await;
    let refs = app.book_refs().await;

    let mut body = TestApp::book_body(&refs);
    body["links"] =
        serde_json::json!([{ "type": "where_to_borrow", "url": "https://example.com" }]);

    assert_store_book_returns_422(&app, body).await;
}

#[tokio::test]
async fn store_book_with_link_url_over_2048_characters_returns_422() {
    let app = TestApp::new().await;
    let refs = app.book_refs().await;

    let url = format!("https://a.co/{}", "b".repeat(2036));
    let mut body = TestApp::book_body(&refs);
    body["links"] = serde_json::json!([{ "type": "Track", "url": url }]);

    assert_store_book_returns_422(&app, body).await;
}

#[tokio::test]
async fn store_book_with_title_missing_language_id_returns_422() {
    let app = TestApp::new().await;
    let refs = app.book_refs().await;

    let mut body = TestApp::book_body(&refs);
    body["titles"] = serde_json::json!([{ "name": "ベルセルク" }]);

    assert_store_book_returns_422(&app, body).await;
}

#[tokio::test]
async fn store_book_with_unknown_content_rating_returns_422() {
    let app = TestApp::new().await;
    let refs = app.book_refs().await;

    let mut body = TestApp::book_body(&refs);
    body["contentRating"] = serde_json::json!(uuid::Uuid::now_v7());

    assert_store_book_returns_422(&app, body).await;
}

#[tokio::test]
async fn store_book_with_unknown_author_returns_422() {
    let app = TestApp::new().await;
    let refs = app.book_refs().await;

    let mut body = TestApp::book_body(&refs);
    body["author"] = serde_json::json!(uuid::Uuid::now_v7());

    assert_store_book_returns_422(&app, body).await;
}

#[tokio::test]
async fn store_book_with_unknown_label_id_returns_422() {
    let app = TestApp::new().await;
    let refs = app.book_refs().await;

    let mut body = TestApp::book_body(&refs);
    body["labelIds"] = serde_json::json!([uuid::Uuid::now_v7()]);

    assert_store_book_returns_422(&app, body).await;

    let count = sqlx::query_scalar!("select count(*) from books")
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(count, Some(0), "the failed write must roll back the book");
}

#[tokio::test]
async fn get_book_with_unknown_id_returns_404() {
    let app = TestApp::new().await;

    let req = Request::get(format!("/books/{}", uuid::Uuid::now_v7()))
        .body(Body::empty())
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
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

#[tokio::test]
async fn get_book_with_malformed_id_returns_400() {
    let app = TestApp::new().await;

    let req = Request::get("/books/not-a-uuid")
        .body(Body::empty())
        .unwrap();

    let resp = app.router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

async fn put_book(app: &TestApp, id: uuid::Uuid, body: &serde_json::Value) -> StatusCode {
    let req = Request::put(format!("/books/{id}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();

    app.router.clone().oneshot(req).await.unwrap().status()
}

#[tokio::test]
async fn update_book_with_valid_body_passes() {
    let app = TestApp::new().await;
    let refs = app.book_refs().await;
    let id = app.insert_book(&TestApp::book_body(&refs)).await;

    let created_at = sqlx::query_scalar!("select created_at from books where id = $1", id)
        .fetch_one(&app.pool)
        .await
        .unwrap();

    let mut body = TestApp::book_body(&refs);
    body["name"] = serde_json::json!("Berserk: Deluxe");
    body["status"] = serde_json::json!("Completed");
    body["type"] = serde_json::json!("Manhwa");
    body["publicationYear"] = serde_json::json!(1990);

    assert_eq!(put_book(&app, id, &body).await, StatusCode::NO_CONTENT);

    let row = sqlx::query!(
        "select name, status, kind, publication_year, updated_at, created_at from books where id = $1",
        id
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();

    assert_eq!(row.name, "Berserk: Deluxe");
    assert_eq!(row.status, "Completed");
    assert_eq!(row.kind, "Manhwa");
    assert_eq!(row.publication_year, 1990);
    assert!(row.updated_at.is_some());
    assert_eq!(
        row.created_at.unix_timestamp(),
        created_at.unix_timestamp(),
        "created_at must stay immutable"
    );
}

#[tokio::test]
async fn update_book_replaces_arrays_wholesale() {
    let app = TestApp::new().await;
    let refs = app.book_refs().await;
    let label_ids = app.label_ids(2).await;

    let mut body = TestApp::book_body(&refs);
    body["labelIds"] = serde_json::json!(label_ids);
    body["links"] = serde_json::json!([
        { "type": "WhereToRead", "url": "https://example.com/read" },
        { "type": "Track", "url": "https://example.com/track" },
    ]);
    body["titles"] = serde_json::json!([
        { "languageId": refs.language, "name": "First" },
        { "languageId": refs.language, "name": "Second" },
    ]);
    let id = app.insert_book(&body).await;

    // One of each replaces the two of each -- a merge would leave three.
    let mut replacement = TestApp::book_body(&refs);
    replacement["labelIds"] = serde_json::json!([label_ids[1]]);
    replacement["links"] = serde_json::json!([
        { "type": "WhereToBuy", "url": "https://example.com/buy" },
    ]);
    replacement["titles"] = serde_json::json!([
        { "languageId": refs.language, "name": "Only" },
    ]);

    assert_eq!(
        put_book(&app, id, &replacement).await,
        StatusCode::NO_CONTENT
    );

    let labels = sqlx::query_scalar!("select label_id from book_labels where book_id = $1", id)
        .fetch_all(&app.pool)
        .await
        .unwrap();
    assert_eq!(labels, vec![label_ids[1]]);

    let links = sqlx::query!("select kind, url from book_links where book_id = $1", id)
        .fetch_all(&app.pool)
        .await
        .unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].kind, "WhereToBuy");

    let titles = sqlx::query!("select name from book_titles where book_id = $1", id)
        .fetch_all(&app.pool)
        .await
        .unwrap();
    assert_eq!(titles.len(), 1);
    assert_eq!(titles[0].name, "Only");
}

#[tokio::test]
async fn update_book_with_empty_arrays_detaches_everything() {
    let app = TestApp::new().await;
    let refs = app.book_refs().await;
    let label_ids = app.label_ids(1).await;

    let mut body = TestApp::book_body(&refs);
    body["labelIds"] = serde_json::json!(label_ids);
    body["links"] = serde_json::json!([{ "type": "Track", "url": "https://example.com/t" }]);
    body["titles"] = serde_json::json!([{ "languageId": refs.language, "name": "Gone" }]);
    let id = app.insert_book(&body).await;

    assert_eq!(
        put_book(&app, id, &TestApp::book_body(&refs)).await,
        StatusCode::NO_CONTENT
    );

    assert_eq!(app.count_book_relations(id).await, (0, 0, 0));
}

#[tokio::test]
async fn update_book_with_unknown_id_returns_404() {
    let app = TestApp::new().await;
    let refs = app.book_refs().await;

    let status = put_book(&app, uuid::Uuid::now_v7(), &TestApp::book_body(&refs)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn update_book_leaves_other_books_untouched() {
    let app = TestApp::new().await;
    let refs = app.book_refs().await;
    let id = app.insert_book(&TestApp::book_body(&refs)).await;
    let other_id = app.insert_book(&TestApp::book_body(&refs)).await;

    let mut body = TestApp::book_body(&refs);
    body["name"] = serde_json::json!("Renamed");

    assert_eq!(put_book(&app, id, &body).await, StatusCode::NO_CONTENT);

    let row = sqlx::query!("select name, updated_at from books where id = $1", other_id)
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(row.name, "Berserk");
    assert!(row.updated_at.is_none());
}

#[tokio::test]
async fn update_book_with_more_than_12_titles_returns_422() {
    let app = TestApp::new().await;
    let refs = app.book_refs().await;
    let id = app.insert_book(&TestApp::book_body(&refs)).await;

    let titles: Vec<serde_json::Value> = (0..13)
        .map(|i| serde_json::json!({ "languageId": refs.language, "name": format!("Title {i}") }))
        .collect();
    let mut body = TestApp::book_body(&refs);
    body["titles"] = serde_json::json!(titles);

    assert_eq!(
        put_book(&app, id, &body).await,
        StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn get_books_returns_default_limit_and_next_cursor() {
    let app = TestApp::new().await;
    let refs = app.book_refs().await;

    for _ in 0..25 {
        app.insert_book(&TestApp::book_body(&refs)).await;
    }

    let req = Request::get("/books").body(Body::empty()).unwrap();
    let resp = app.router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let resp: RespWrapper<Vec<Book>> = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(resp.data.len(), 20);
    assert!(resp.next_cursor.is_some());
}

#[tokio::test]
async fn get_books_with_after_and_limit_3_returns_next_page() {
    let app = TestApp::new().await;
    let refs = app.book_refs().await;

    for _ in 0..6 {
        app.insert_book(&TestApp::book_body(&refs)).await;
    }

    let req = Request::get("/books?limit=3").body(Body::empty()).unwrap();
    let resp = app.router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let resp: RespWrapper<Vec<Book>> = serde_json::from_slice(&bytes).unwrap();
    let ids: Vec<uuid::Uuid> = resp.data.iter().map(|b| b.id).collect();
    let cursor = resp.next_cursor.unwrap();

    let second_req = Request::get(format!("/books?limit=3&after={cursor}"))
        .body(Body::empty())
        .unwrap();
    let second_resp = app.router.oneshot(second_req).await.unwrap();
    assert_eq!(second_resp.status(), StatusCode::OK);

    let second_bytes = second_resp.into_body().collect().await.unwrap().to_bytes();
    let second_resp: RespWrapper<Vec<Book>> = serde_json::from_slice(&second_bytes).unwrap();
    let second_ids: Vec<uuid::Uuid> = second_resp.data.iter().map(|b| b.id).collect();

    assert_eq!(second_ids.len(), 3);
    assert!(second_ids.iter().all(|id| !ids.contains(id)));
    assert!(second_resp.next_cursor.is_none());
}

#[tokio::test]
async fn get_books_desc_order_confirmed() {
    let app = TestApp::new().await;
    let refs = app.book_refs().await;

    let mut ids = Vec::with_capacity(3);
    for _ in 0..3 {
        ids.push(app.insert_book(&TestApp::book_body(&refs)).await);
    }
    ids.sort_by(|a, b| b.cmp(a));

    let req = Request::get("/books").body(Body::empty()).unwrap();
    let resp = app.router.oneshot(req).await.unwrap();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let resp: RespWrapper<Vec<Book>> = serde_json::from_slice(&bytes).unwrap();
    let returned_ids: Vec<uuid::Uuid> = resp.data.iter().map(|b| b.id).collect();

    assert_eq!(returned_ids, ids);
}

#[tokio::test]
async fn get_books_zero_limit_returns_422() {
    let app = TestApp::new().await;

    let req = Request::get("/books?limit=0").body(Body::empty()).unwrap();
    let resp = app.router.oneshot(req).await.unwrap();

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn get_books_invalid_after_returns_400() {
    let app = TestApp::new().await;

    let req = Request::get("/books?after=not-a-uuid")
        .body(Body::empty())
        .unwrap();
    let resp = app.router.oneshot(req).await.unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn delete_book_removes_its_attached_rows() {
    let app = TestApp::new().await;
    let refs = app.book_refs().await;
    let label_ids = app.label_ids(1).await;

    let mut body = TestApp::book_body(&refs);
    body["labelIds"] = serde_json::json!(label_ids);
    body["links"] = serde_json::json!([{ "type": "Track", "url": "https://example.com/t" }]);
    body["titles"] = serde_json::json!([{ "languageId": refs.language, "name": "Gone" }]);
    let id = app.insert_book(&body).await;

    let req = Request::delete(format!("/books/{id}"))
        .body(Body::empty())
        .unwrap();
    let resp = app.router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let count = sqlx::query_scalar!("select count(*) from books")
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(count, Some(0));

    assert_eq!(app.count_book_relations(id).await, (0, 0, 0));
}

#[tokio::test]
async fn delete_book_with_unknown_id_returns_404() {
    let app = TestApp::new().await;

    let req = Request::delete(format!("/books/{}", uuid::Uuid::now_v7()))
        .body(Body::empty())
        .unwrap();
    let resp = app.router.oneshot(req).await.unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
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

#[tokio::test]
async fn delete_book_leaves_other_books_untouched() {
    let app = TestApp::new().await;
    let refs = app.book_refs().await;
    let label_ids = app.label_ids(1).await;

    let mut body = TestApp::book_body(&refs);
    body["labelIds"] = serde_json::json!(label_ids);
    let id = app.insert_book(&body).await;
    let other_id = app.insert_book(&body).await;

    let req = Request::delete(format!("/books/{id}"))
        .body(Body::empty())
        .unwrap();
    let resp = app.router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let count = sqlx::query_scalar!("select count(*) from books where id = $1", other_id)
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(count, Some(1));

    let labels = sqlx::query_scalar!(
        "select label_id from book_labels where book_id = $1",
        other_id
    )
    .fetch_all(&app.pool)
    .await
    .unwrap();
    assert_eq!(labels, label_ids);
}
