use std::sync::LazyLock;
use std::time::Duration;

use aws_sdk_s3::primitives::ByteStream;
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::response::Response;
use axum_test::multipart::{MultipartForm, Part};
use fake::Fake;
use http_body_util::BodyExt;
use manga_theka::{
    config::{Config, Database},
    database,
    entity::{
        AlternativeTitle, Book, BookCoverQuery, BookLink, BookName, Chapter, ChapterLocalization,
        ChapterName, ChapterNumber, ChapterVolume, ContentRating, CoverUrl, Creator, Description,
        ImageExtension, Label, Language, LinkUrl, Name,
    },
    object_storage,
    startup::App,
    telemetry,
};
use serde::Deserialize;
use sqlx::{AssertSqlSafe, ConnectOptions, Connection, Executor, PgConnection, PgPool};
use tower::ServiceExt;
use tracing_log::log::LevelFilter;
use uuid::Uuid;

use crate::fakers::{BookFaker, ChapterFaker, LANGUAGES};

static TRACING: LazyLock<()> = LazyLock::new(|| {
    telemetry::init_subscriber("info");
});

#[derive(Debug)]
pub struct BookSample {
    pub name: Option<String>,
    pub updated_at: Option<time::OffsetDateTime>,
    pub labels: i64,
    pub links: i64,
    pub titles: i64,
}

#[derive(Debug)]
pub struct ChapterSample {
    pub number: Option<f32>,
    pub name: Option<String>,
    pub updated_at: Option<time::OffsetDateTime>,
    pub localizations: i64,
}

#[derive(Deserialize, Debug)]
pub struct RespWrapper<T> {
    pub data: T,
    #[serde(rename = "nextCursor", default)]
    pub next_cursor: Option<uuid::Uuid>,
}

pub async fn assert_error(resp: Response, expected: StatusCode) {
    assert_eq!(resp.status(), expected);
    assert!(
        !resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .is_empty(),
        "error response must carry a body"
    );
}

pub async fn assert_stored(resp: Response) -> Uuid {
    assert_eq!(resp.status(), StatusCode::CREATED);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let id = v["data"]["id"]
        .as_str()
        .expect("created response must carry data.id");

    Uuid::parse_str(id).expect("data.id must be a uuid")
}

pub async fn staged_ids(app: &TestApp, release_id: Uuid, parts: &[&[u8]]) -> Vec<Uuid> {
    let resp = app.post_upload_pages(release_id, parts).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let wrapper: RespWrapper<Vec<Uuid>> = serde_json::from_slice(&bytes).unwrap();

    wrapper.data
}

pub async fn page_ids_in_order(app: &TestApp, release_id: Uuid) -> Vec<Uuid> {
    let resp = app.get_pages(release_id).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let wrapper: RespWrapper<Vec<serde_json::Value>> = serde_json::from_slice(&bytes).unwrap();

    wrapper
        .data
        .iter()
        .map(|p| Uuid::parse_str(p["id"].as_str().unwrap()).unwrap())
        .collect()
}

pub async fn release_version(app: &TestApp, release_id: Uuid) -> i64 {
    let resp = app.get_release(release_id).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let wrapper: RespWrapper<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();

    wrapper.data["version"].as_i64().unwrap()
}

pub fn label_keys(labels: &[Label]) -> Vec<(Uuid, &str, &str)> {
    let mut keys: Vec<(Uuid, &str, &str)> = labels
        .iter()
        .map(|l| (l.id, l.name.as_str(), l.kind.as_ref()))
        .collect();
    keys.sort();

    keys
}

pub fn link_keys(links: &[BookLink]) -> Vec<(&str, &str)> {
    let mut keys: Vec<(&str, &str)> = links
        .iter()
        .map(|l| (l.kind.as_ref(), l.url.as_ref()))
        .collect();
    keys.sort();

    keys
}

pub fn title_keys(titles: &[AlternativeTitle]) -> Vec<(Uuid, &str)> {
    let mut keys: Vec<(Uuid, &str)> = titles
        .iter()
        .map(|t| (t.language_id, t.name.as_ref()))
        .collect();
    keys.sort();

    keys
}

pub fn localization_keys(localizations: &[ChapterLocalization]) -> Vec<(Uuid, &str)> {
    let mut keys: Vec<(Uuid, &str)> = localizations
        .iter()
        .map(|l| (l.language_id, l.name.as_ref()))
        .collect();
    keys.sort();

    keys
}

// TODO: migrate this to axum_test::TestServer/TestResponse.
pub struct TestApp {
    pub pool: PgPool,
    pub router: Router,
    pub s3: aws_sdk_s3::Client,
    pub covers_bucket: String,
    pub release_pages_bucket: String,
}

impl TestApp {
    pub async fn new() -> TestApp {
        LazyLock::force(&TRACING);

        let db_name = Uuid::now_v7().to_string();
        let covers_bucket = format!("covers-{}", Uuid::now_v7());
        let release_pages_bucket = format!("release-pages-{}", Uuid::now_v7());
        let cfg = Config::from_env_pairs([
            ("APP_DATABASE__USERNAME", "postgres"),
            ("APP_DATABASE__PASSWORD", "postgres"),
            ("APP_DATABASE__HOST", "localhost"),
            ("APP_DATABASE__PORT", "5432"),
            ("APP_DATABASE__DATABASE_NAME", db_name.as_str()),
            ("APP_OBJECT_STORAGE__ENDPOINT", "http://localhost:9000"),
            ("APP_OBJECT_STORAGE__REGION", "us-east-1"),
            ("APP_OBJECT_STORAGE__ACCESS_KEY", "rustfsadmin"),
            ("APP_OBJECT_STORAGE__SECRET_KEY", "rustfsadmin"),
            ("APP_OBJECT_STORAGE__COVERS_BUCKET", covers_bucket.as_str()),
            (
                "APP_OBJECT_STORAGE__RELEASE_PAGES_BUCKET",
                release_pages_bucket.as_str(),
            ),
            // Ignored pairs
            ("APP_ADDR", "0.0.0.0:0"),
            ("APP_LOG_LEVEL", "info"),
        ]);
        let pool = Self::configure_db(&cfg.database).await;
        let s3 = object_storage::client(&cfg.object_storage).await;
        let app = App::build(&cfg).await;

        TestApp {
            pool,
            router: app.router(),
            s3,
            covers_bucket,
            release_pages_bucket,
        }
    }

    async fn configure_db(cfg: &Database) -> PgPool {
        let conn_cfg = cfg
            .without_db()
            .log_slow_statements(LevelFilter::Warn, Duration::from_secs(10));
        let mut conn = PgConnection::connect_with(&conn_cfg)
            .await
            .expect("failed to connect to Postgres without database");

        conn.execute(AssertSqlSafe(format!(
            r#"CREATE DATABASE "{}""#,
            cfg.database_name
        )))
        .await
        .expect("failed to create database");

        database::pool(cfg).await
    }

    pub async fn object_exists(&self, bucket: &str, key: Uuid) -> bool {
        self.s3
            .head_object()
            .bucket(bucket)
            .key(key.to_string())
            .send()
            .await
            .is_ok()
    }

    pub async fn object_content_type(&self, bucket: &str, key: Uuid) -> String {
        self.s3
            .head_object()
            .bucket(bucket)
            .key(key.to_string())
            .send()
            .await
            .expect("stored object must exist")
            .content_type()
            .expect("stored object must carry a content type")
            .to_owned()
    }

    pub async fn objects_count(&self, bucket: &str) -> usize {
        self.s3
            .list_objects_v2()
            .bucket(bucket)
            .send()
            .await
            .expect("failed to list the bucket")
            .contents()
            .len()
    }

    pub async fn insert_book(&self, b: &Book) {
        sqlx::query!(
            r#"
insert into books(id, name, description, publication_year, content_rating, status, kind,
                  publication_language, updated_at, created_at)
values($1, $2, $3, $4, $5, $6, $7, $8, $9, $10);
        "#,
            b.id,
            b.name.as_ref(),
            b.description.as_ref(),
            b.publication_year,
            b.content_rating.id,
            b.status.as_ref() as _,
            b.kind.as_ref() as _,
            b.publication_language.id,
            b.updated_at,
            b.created_at
        )
        .execute(&self.pool)
        .await
        .expect("failed to insert factory book");

        let mut label_ids: Vec<Uuid> = Vec::with_capacity(b.labels.len());
        for l in &b.labels {
            label_ids.push(l.id);
        }
        sqlx::query!(
            r#"
insert into book_labels(book_id, label_id)
select $1, label_id
from unnest($2::uuid[]) as label_id;
        "#,
            b.id,
            &label_ids
        )
        .execute(&self.pool)
        .await
        .expect("failed to insert factory book labels");

        let mut link_kinds: Vec<String> = Vec::with_capacity(b.links.len());
        let mut link_urls: Vec<String> = Vec::with_capacity(b.links.len());
        for l in &b.links {
            link_kinds.push(l.kind.as_ref().to_owned());
            link_urls.push(l.url.as_ref().to_owned());
        }
        sqlx::query!(
            r#"
insert into book_links(book_id, kind, url)
select $1, link.kind, link.url
from unnest($2::text[], $3::text[]) as link(kind, url);
        "#,
            b.id,
            &link_kinds,
            &link_urls
        )
        .execute(&self.pool)
        .await
        .expect("failed to insert factory book links");

        let mut title_language_ids: Vec<Uuid> = Vec::with_capacity(b.titles.len());
        let mut title_names: Vec<String> = Vec::with_capacity(b.titles.len());
        for t in &b.titles {
            title_language_ids.push(t.language_id);
            title_names.push(t.name.as_ref().to_owned());
        }
        sqlx::query!(
            r#"
insert into book_titles(book_id, language_id, name)
select $1, title.language_id, title.name
from unnest($2::uuid[], $3::text[]) as title(language_id, name);
        "#,
            b.id,
            &title_language_ids,
            &title_names
        )
        .execute(&self.pool)
        .await
        .expect("failed to insert factory book titles");
    }

    pub async fn fetch_book_sample(&self, id: Uuid) -> BookSample {
        let row = sqlx::query!(
            r#"
select (select b.name from books b where b.id = $1) as "name?",
       (select b.updated_at from books b where b.id = $1) as "updated_at?",
       (select count(*) from book_labels bl where bl.book_id = $1) as "labels!",
       (select count(*) from book_links blk where blk.book_id = $1) as "links!",
       (select count(*) from book_titles bt where bt.book_id = $1) as "titles!";
        "#,
            id
        )
        .fetch_one(&self.pool)
        .await
        .expect("failed to read book sample");

        BookSample {
            name: row.name,
            updated_at: row.updated_at,
            labels: row.labels,
            links: row.links,
            titles: row.titles,
        }
    }

    pub async fn fetch_book(&self, id: Uuid) -> Book {
        let row = sqlx::query!(
            r#"
select b.name, b.description, b.publication_year, b.status, b.kind, b.updated_at, b.created_at,
       cr.id as content_rating_id, cr.name as content_rating_name, cr.code as content_rating_code,
       l.id as language_id, l.code as language_code, l.name as language_name
from books b
join content_ratings cr on cr.id = b.content_rating
join languages l on l.id = b.publication_language
where b.id = $1;
        "#,
            id
        )
        .fetch_one(&self.pool)
        .await
        .expect("failed to read book");

        let labels = sqlx::query!(
            r#"
select l.id, l.name, l.kind
from labels l
join book_labels bl on bl.label_id = l.id
where bl.book_id = $1
order by l.name;
        "#,
            id
        )
        .fetch_all(&self.pool)
        .await
        .expect("failed to read book labels")
        .into_iter()
        .map(|r| Label {
            id: r.id,
            name: r.name,
            kind: r.kind.parse().expect("stored label kind must be valid"),
        })
        .collect();

        let links = sqlx::query!(
            r#"
select kind, url
from book_links
where book_id = $1
order by kind, url;
        "#,
            id
        )
        .fetch_all(&self.pool)
        .await
        .expect("failed to read book links")
        .into_iter()
        .map(|r| BookLink {
            kind: r.kind.parse().expect("stored link kind must be valid"),
            url: LinkUrl::try_from(r.url).expect("stored link url must be valid"),
        })
        .collect();

        let titles = sqlx::query!(
            r#"
select language_id, name
from book_titles
where book_id = $1
order by language_id, name;
        "#,
            id
        )
        .fetch_all(&self.pool)
        .await
        .expect("failed to read book titles")
        .into_iter()
        .map(|r| AlternativeTitle {
            language_id: r.language_id,
            name: BookName::try_from(r.name).expect("stored title name must be valid"),
        })
        .collect();

        let creators = sqlx::query!(
            r#"
select c.id, c.first_name, c.last_name, c.role, c.created_at
from creators c
join book_creators bc on bc.creator_id = c.id
where bc.book_id = $1
order by c.id;
        "#,
            id
        )
        .fetch_all(&self.pool)
        .await
        .expect("failed to read book creators")
        .into_iter()
        .map(|r| Creator {
            id: r.id,
            first_name: Name::try_from(r.first_name).expect("stored first name must be valid"),
            last_name: Name::try_from(r.last_name).expect("stored last name must be valid"),
            role: r.role.parse().expect("stored creator role must be valid"),
            created_at: r.created_at,
        })
        .collect();

        Book {
            id,
            name: BookName::try_from(row.name).expect("stored name must be valid"),
            description: Description::try_from(row.description)
                .expect("stored description must be valid"),
            publication_year: row.publication_year,
            content_rating: ContentRating {
                id: row.content_rating_id,
                name: row.content_rating_name,
                code: row.content_rating_code,
            },
            status: row.status.parse().expect("stored status must be valid"),
            kind: row.kind.parse().expect("stored kind must be valid"),
            publication_language: Language {
                id: row.language_id,
                code: row.language_code,
                name: row.language_name,
            },
            labels,
            links,
            titles,
            creators,
            updated_at: row.updated_at,
            created_at: row.created_at,
        }
    }

    pub async fn insert_random_book(&self) -> Uuid {
        let book: Book = BookFaker::default().fake();
        self.insert_book(&book).await;

        book.id
    }

    pub async fn fetch_covers(&self, book_id: Uuid) -> Vec<BookCoverQuery> {
        sqlx::query!(
            r#"
select id, extension, is_main
from book_covers
where book_id = $1
order by id;
        "#,
            book_id
        )
        .fetch_all(&self.pool)
        .await
        .expect("failed to read book covers")
        .into_iter()
        .map(|r| BookCoverQuery {
            id: r.id,
            extension: r
                .extension
                .parse()
                .expect("stored cover extension must be valid"),
            is_main: r.is_main,
            url: CoverUrl::from((book_id, r.id)),
        })
        .collect()
    }

    pub async fn insert_cover(&self, book_id: Uuid, image: &'static [u8]) -> Uuid {
        let id = Uuid::now_v7();
        let extension =
            ImageExtension::try_from(image).expect("factory cover must be a supported image");

        sqlx::query!(
            r#"
insert into book_covers(id, book_id, extension, is_main)
values($1, $2, $3, false);
        "#,
            id,
            book_id,
            extension.as_ref()
        )
        .execute(&self.pool)
        .await
        .expect("failed to insert factory book cover");

        self.s3
            .put_object()
            .bucket(&self.covers_bucket)
            .key(id.to_string())
            .content_type(extension.content_type())
            .content_disposition(format!("inline; filename=\"{id}.{}\"", extension.as_ref()))
            .body(ByteStream::from_static(image))
            .send()
            .await
            .expect("failed to upload factory book cover");

        id
    }

    pub async fn insert_chapter(&self, c: &Chapter) {
        sqlx::query!(
            r#"
insert into chapters(id, book_id, number, name, volume, updated_at, created_at)
values($1, $2, $3, $4, $5, $6, $7);
        "#,
            c.id,
            c.book_id,
            c.number.as_f32(),
            c.name.as_ref().map(AsRef::as_ref),
            c.volume.map(ChapterVolume::as_i16),
            c.updated_at,
            c.created_at
        )
        .execute(&self.pool)
        .await
        .expect("failed to insert factory chapter");

        let mut language_ids: Vec<Uuid> = Vec::with_capacity(c.localizations.len());
        let mut names: Vec<String> = Vec::with_capacity(c.localizations.len());
        for l in &c.localizations {
            language_ids.push(l.language_id);
            names.push(l.name.as_ref().to_owned());
        }
        sqlx::query!(
            r#"
insert into chapter_localizations(chapter_id, language_id, name)
select $1, localization.language_id, localization.name
from unnest($2::uuid[], $3::text[]) as localization(language_id, name);
        "#,
            c.id,
            &language_ids,
            &names
        )
        .execute(&self.pool)
        .await
        .expect("failed to insert factory chapter localizations");
    }

    pub async fn insert_numbered_chapters(&self, book_id: Uuid, numbers: &[f32]) {
        for number in numbers {
            let mut chapter: Chapter = ChapterFaker {
                book_id,
                localizations: 0..=0,
            }
            .fake();
            chapter.number =
                ChapterNumber::try_from(*number).expect("factory number must be valid");

            self.insert_chapter(&chapter).await;
        }
    }

    pub async fn insert_random_chapter(&self, book_id: Uuid) -> Uuid {
        let chapter: Chapter = ChapterFaker::new(book_id).fake();
        self.insert_chapter(&chapter).await;

        chapter.id
    }

    pub async fn fetch_chapter(&self, id: Uuid) -> Chapter {
        let row = sqlx::query!(
            r#"
select book_id, number, name, volume, updated_at, created_at
from chapters
where id = $1;
        "#,
            id
        )
        .fetch_one(&self.pool)
        .await
        .expect("failed to read chapter");

        let localizations = sqlx::query!(
            r#"
select language_id, name
from chapter_localizations
where chapter_id = $1
order by language_id;
        "#,
            id
        )
        .fetch_all(&self.pool)
        .await
        .expect("failed to read chapter localizations")
        .into_iter()
        .map(|r| ChapterLocalization {
            language_id: r.language_id,
            name: ChapterName::try_from(r.name).expect("stored localization name must be valid"),
        })
        .collect();

        Chapter {
            id,
            book_id: row.book_id,
            number: ChapterNumber::try_from(row.number).expect("stored number must be valid"),
            name: row
                .name
                .map(|n| ChapterName::try_from(n).expect("stored name must be valid")),
            volume: row
                .volume
                .map(|v| ChapterVolume::try_from(v).expect("stored volume must be valid")),
            localizations,
            updated_at: row.updated_at,
            created_at: row.created_at,
        }
    }

    pub async fn fetch_chapter_sample(&self, id: Uuid) -> ChapterSample {
        let row = sqlx::query!(
            r#"
select (select c.number from chapters c where c.id = $1) as "number?",
       (select c.name from chapters c where c.id = $1) as "name?",
       (select c.updated_at from chapters c where c.id = $1) as "updated_at?",
       (select count(*) from chapter_localizations cl where cl.chapter_id = $1) as "localizations!";
        "#,
            id
        )
        .fetch_one(&self.pool)
        .await
        .expect("failed to read chapter sample");

        ChapterSample {
            number: row.number,
            name: row.name,
            updated_at: row.updated_at,
            localizations: row.localizations,
        }
    }

    pub async fn insert_creator(&self, c: &Creator) {
        sqlx::query!(
            r#"
insert into creators(id, first_name, last_name, role, created_at)
values($1, $2, $3, $4, $5);
        "#,
            c.id,
            c.first_name.as_ref(),
            c.last_name.as_ref(),
            c.role.as_ref() as _,
            c.created_at
        )
        .execute(&self.pool)
        .await
        .expect("failed to insert factory creator");
    }

    pub async fn post_cover(&self, book_id: Uuid, image: &[u8]) -> Response {
        let form = Self::multipart_body(&[image]);
        let req = Request::post(format!("/books/{book_id}/covers"))
            .header(header::CONTENT_TYPE, form.content_type())
            .body(Body::from(form))
            .unwrap();

        self.router.clone().oneshot(req).await.unwrap()
    }

    pub async fn get_covers(&self, book_id: Uuid) -> Response {
        let req = Request::get(format!("/books/{book_id}/covers"))
            .body(Body::empty())
            .unwrap();

        self.router.clone().oneshot(req).await.unwrap()
    }

    pub async fn get_cover_image(&self, book_id: Uuid, cover_id: Uuid) -> Response {
        let req = Request::get(format!("/books/{book_id}/covers/{cover_id}/image"))
            .body(Body::empty())
            .unwrap();

        self.router.clone().oneshot(req).await.unwrap()
    }

    pub async fn put_main_cover(&self, book_id: Uuid, cover_id: Uuid) -> StatusCode {
        let body = serde_json::json!({ "coverId": cover_id }).to_string();
        let req = Request::put(format!("/books/{book_id}/main-cover"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .unwrap();

        self.router.clone().oneshot(req).await.unwrap().status()
    }

    pub async fn delete_cover(&self, book_id: Uuid, cover_id: Uuid) -> Response {
        let req = Request::delete(format!("/books/{book_id}/covers/{cover_id}"))
            .body(Body::empty())
            .unwrap();

        self.router.clone().oneshot(req).await.unwrap()
    }

    pub async fn get_creator(&self, id: Uuid) -> Response {
        let req = Request::get(format!("/creators/{id}"))
            .body(Body::empty())
            .unwrap();

        self.router.clone().oneshot(req).await.unwrap()
    }

    pub async fn delete_creator(&self, id: Uuid) -> Response {
        let req = Request::delete(format!("/creators/{id}"))
            .body(Body::empty())
            .unwrap();

        self.router.clone().oneshot(req).await.unwrap()
    }

    pub async fn delete_chapter(&self, id: Uuid) -> Response {
        let req = Request::delete(format!("/chapters/{id}"))
            .body(Body::empty())
            .unwrap();

        self.router.clone().oneshot(req).await.unwrap()
    }

    pub async fn get_chapters(&self, path: String) -> RespWrapper<Vec<Chapter>> {
        let req = Request::get(path).body(Body::empty()).unwrap();
        let resp = self.router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).expect("failed to parse chapter page")
    }

    fn multipart_body(parts: &[&[u8]]) -> MultipartForm {
        parts
            .iter()
            .enumerate()
            .fold(MultipartForm::new(), |form, (i, part)| {
                form.add_part(
                    format!("page{i}"),
                    Part::bytes(part.to_vec()).file_name(format!("page{i}")),
                )
            })
    }

    pub async fn non_publication_language(&self, book_id: Uuid) -> Uuid {
        let publication = sqlx::query_scalar!(
            r#"
select publication_language
from books
where id = $1;
        "#,
            book_id
        )
        .fetch_one(&self.pool)
        .await
        .expect("failed to read book publication language");

        LANGUAGES
            .iter()
            .map(|l| l.id)
            .find(|id| *id != publication)
            .expect("the language catalog must hold a translation language")
    }

    pub async fn insert_release(&self, chapter_id: Uuid, language_id: Uuid) -> Uuid {
        let id = Uuid::now_v7();

        sqlx::query!(
            r#"
insert into chapter_releases(id, chapter_id, language_id, version)
values($1, $2, $3, 0);
        "#,
            id,
            chapter_id,
            language_id
        )
        .execute(&self.pool)
        .await
        .expect("failed to insert factory chapter release");

        id
    }

    pub async fn insert_random_release(&self, book_id: Uuid, chapter_id: Uuid) -> Uuid {
        let language_id = self.non_publication_language(book_id).await;

        self.insert_release(chapter_id, language_id).await
    }

    pub async fn insert_page(
        &self,
        release_id: Uuid,
        sort_order: Option<i16>,
        image: &'static [u8],
    ) -> Uuid {
        let id = Uuid::now_v7();
        let extension =
            ImageExtension::try_from(image).expect("factory page must be a supported image");

        sqlx::query!(
            r#"
insert into chapter_pages(id, release_id, sort_order, extension)
values($1, $2, $3, $4);
        "#,
            id,
            release_id,
            sort_order,
            extension.as_ref()
        )
        .execute(&self.pool)
        .await
        .expect("failed to insert factory chapter page");

        self.s3
            .put_object()
            .bucket(&self.release_pages_bucket)
            .key(id.to_string())
            .content_type(extension.content_type())
            .content_disposition(format!("inline; filename=\"{id}.{}\"", extension.as_ref()))
            .body(ByteStream::from_static(image))
            .send()
            .await
            .expect("failed to upload factory chapter page");

        id
    }

    pub async fn fetch_release_version(&self, release_id: Uuid) -> i32 {
        sqlx::query_scalar!(
            r#"
select version
from chapter_releases
where id = $1;
        "#,
            release_id
        )
        .fetch_one(&self.pool)
        .await
        .expect("failed to read chapter release version")
    }

    pub async fn fetch_page_order(&self, release_id: Uuid) -> Vec<(Uuid, Option<i16>)> {
        sqlx::query!(
            r#"
select id, sort_order
from chapter_pages
where release_id = $1
order by sort_order nulls last, id;
        "#,
            release_id
        )
        .fetch_all(&self.pool)
        .await
        .expect("failed to read chapter pages")
        .into_iter()
        .map(|r| (r.id, r.sort_order))
        .collect()
    }

    pub async fn post_release(&self, chapter_id: Uuid, language_id: Uuid) -> Response {
        let body = serde_json::json!({ "languageId": language_id }).to_string();
        let req = Request::post(format!("/chapters/{chapter_id}/releases"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .unwrap();

        self.router.clone().oneshot(req).await.unwrap()
    }

    pub async fn get_releases(&self, chapter_id: Uuid) -> Response {
        let req = Request::get(format!("/chapters/{chapter_id}/releases"))
            .body(Body::empty())
            .unwrap();

        self.router.clone().oneshot(req).await.unwrap()
    }

    pub async fn get_release(&self, id: Uuid) -> Response {
        let req = Request::get(format!("/releases/{id}"))
            .body(Body::empty())
            .unwrap();

        self.router.clone().oneshot(req).await.unwrap()
    }

    pub async fn post_upload_pages(&self, release_id: Uuid, parts: &[&[u8]]) -> Response {
        let form = Self::multipart_body(parts);
        let req = Request::post(format!("/releases/{release_id}/upload"))
            .header(header::CONTENT_TYPE, form.content_type())
            .body(Body::from(form))
            .unwrap();

        self.router.clone().oneshot(req).await.unwrap()
    }

    pub async fn post_commit(&self, release_id: Uuid, page_order: &[Uuid]) -> Response {
        let body = serde_json::json!({ "pageOrder": page_order }).to_string();
        let req = Request::post(format!("/releases/{release_id}/commit"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .unwrap();

        self.router.clone().oneshot(req).await.unwrap()
    }

    pub async fn get_pages(&self, release_id: Uuid) -> Response {
        let req = Request::get(format!("/releases/{release_id}/pages"))
            .body(Body::empty())
            .unwrap();

        self.router.clone().oneshot(req).await.unwrap()
    }

    pub async fn get_staged(&self, release_id: Uuid) -> Response {
        let req = Request::get(format!("/releases/{release_id}/pages?status=staged"))
            .body(Body::empty())
            .unwrap();

        self.router.clone().oneshot(req).await.unwrap()
    }

    pub async fn get_page_image(&self, release_id: Uuid, page_id: Uuid) -> Response {
        let req = Request::get(format!("/releases/{release_id}/pages/{page_id}/image"))
            .body(Body::empty())
            .unwrap();

        self.router.clone().oneshot(req).await.unwrap()
    }

    pub async fn delete_book(&self, id: Uuid) -> Response {
        let req = Request::delete(format!("/books/{id}"))
            .body(Body::empty())
            .unwrap();

        self.router.clone().oneshot(req).await.unwrap()
    }

    pub async fn delete_release(&self, id: Uuid) -> Response {
        let req = Request::delete(format!("/releases/{id}"))
            .body(Body::empty())
            .unwrap();

        self.router.clone().oneshot(req).await.unwrap()
    }
}
