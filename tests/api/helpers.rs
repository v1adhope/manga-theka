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
        AlternativeTitle, BookCoverQuery, BookKind, BookLink, BookName, BookQuery, BookStatus,
        BookVisibility, Chapter, ChapterLocalization, ChapterName, ChapterNumber, ChapterVolume,
        ContentRating, CoverUrl, Creator, CreatorQuery, CreatorRole, Email, Feedback,
        ImageExtension, Label, Language, LinkUrl, Name, PublicationDemographic, Role, Session,
        Text, Timestamp, UserQuery,
    },
    hasher::Hasher,
    jwt::Jwt,
    memory_storage::{self, MemoryStore},
    object_storage,
    startup::App,
    telemetry,
};
use serde::Deserialize;
use sqlx::{AssertSqlSafe, ConnectOptions, Connection, Executor, PgConnection, PgPool};
use tower::ServiceExt;
use tracing_log::log::LevelFilter;
use uuid::Uuid;

use crate::fakers::{
    ACTION, BookFaker, CONTENT_RATINGS, ChapterFaker, FANTASY, ISEKAI, LABELS, LANGUAGES,
    LONG_STRIP, MAFIA, ROMANCE, SCHOOL_LIFE, ZOMBIES,
};

static TRACING: LazyLock<()> = LazyLock::new(|| {
    telemetry::init_subscriber("info");
});

/// Four Ed25519 PEM files (two keypairs) written once per test process into a
/// temp dir, derived from fixed literal seeds so no key material is committed
/// and CI needs no openssl.
pub static KEYS: LazyLock<TestKeys> = LazyLock::new(TestKeys::generate);

pub struct TestKeys {
    pub access_private: std::path::PathBuf,
    pub access_public: std::path::PathBuf,
    pub refresh_private: std::path::PathBuf,
    pub refresh_public: std::path::PathBuf,
}

impl TestKeys {
    fn generate() -> Self {
        use ed25519_dalek::SigningKey;
        use ed25519_dalek::pkcs8::{EncodePrivateKey, EncodePublicKey, spki::der::pem::LineEnding};

        let dir =
            std::env::temp_dir().join(format!("manga-theka-test-keys-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("failed to create the test key dir");

        let write = |name: &str, seed: [u8; 32], private: bool| {
            let signing = SigningKey::from_bytes(&seed);
            let pem = if private {
                signing
                    .to_pkcs8_pem(LineEnding::LF)
                    .expect("failed to encode the private key")
                    .to_string()
            } else {
                signing
                    .verifying_key()
                    .to_public_key_pem(LineEnding::LF)
                    .expect("failed to encode the public key")
            };
            let path = dir.join(name);
            std::fs::write(&path, pem).expect("failed to write a test key");
            path
        };

        TestKeys {
            access_private: write("access_private.pem", [7u8; 32], true),
            access_public: write("access_public.pem", [7u8; 32], false),
            refresh_private: write("refresh_private.pem", [11u8; 32], true),
            refresh_public: write("refresh_public.pem", [11u8; 32], false),
        }
    }
}

#[derive(Debug)]
pub struct BookSample {
    pub name: Option<String>,
    pub updated_at: Option<time::OffsetDateTime>,
    pub labels: i64,
    pub links: i64,
    pub titles: i64,
}

#[derive(Debug)]
pub struct BookVisibilityState {
    pub visibility: String,
    pub note: Option<String>,
    pub updated_at: Option<time::OffsetDateTime>,
}

#[derive(Debug)]
pub struct ChapterSample {
    pub number: Option<f32>,
    pub name: Option<String>,
    pub updated_at: Option<time::OffsetDateTime>,
    pub localizations: i64,
}

#[derive(Deserialize, Debug)]
pub struct RespWrapper<T, C = Uuid> {
    pub data: T,
    #[serde(rename = "nextCursor", default)]
    pub next_cursor: Option<C>,
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

pub fn redirect_target(resp: &Response) -> &str {
    resp.headers()
        .get(header::LOCATION)
        .expect("a redirect must carry a location")
        .to_str()
        .expect("a location must be printable")
}

pub fn labels(ids: &[Uuid]) -> Vec<Label> {
    ids.iter()
        .map(|id| {
            LABELS
                .iter()
                .find(|l| l.id == *id)
                .expect("fixture label must be seeded")
                .clone()
        })
        .collect()
}

pub fn label_keys(labels: &[Label]) -> Vec<(Uuid, &str, &str)> {
    let mut keys: Vec<(Uuid, &str, &str)> = labels
        .iter()
        .map(|l| (l.id, l.name.as_str(), l.kind.as_ref()))
        .collect();
    keys.sort();

    keys
}

type FacetRow = (
    &'static [Uuid],
    BookKind,
    BookStatus,
    PublicationDemographic,
    usize,
    usize,
);

pub fn day(n: i64) -> time::OffsetDateTime {
    time::OffsetDateTime::UNIX_EPOCH + time::Duration::days(n)
}

pub fn rfc3339(at: time::OffsetDateTime) -> String {
    at.format(&time::format_description::well_known::Rfc3339)
        .expect("a fixture timestamp must render")
}

pub fn ids(books: &[BookQuery]) -> Vec<Uuid> {
    books.iter().map(|b| b.id).collect()
}

pub fn pick(books: &[BookQuery], wanted: &[usize]) -> Vec<Uuid> {
    wanted.iter().map(|i| books[*i].id).collect()
}

pub fn sorted(mut v: Vec<Uuid>) -> Vec<Uuid> {
    v.sort();

    v
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

pub fn creator_keys(creators: &[CreatorQuery]) -> Vec<(Uuid, &str, &str, Vec<&str>)> {
    let mut keys: Vec<(Uuid, &str, &str, Vec<&str>)> = creators
        .iter()
        .map(|c| {
            let mut roles: Vec<&str> = c.roles.iter().map(AsRef::as_ref).collect();
            roles.sort();
            (c.id, c.first_name.as_ref(), c.last_name.as_ref(), roles)
        })
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

/// The known password whose Argon2id PHC string `UserFaker` stores, so login
/// tests can present a password that actually verifies.
pub const KNOWN_PASSWORD: &str = "correct horse battery staple";
pub const KNOWN_PASSWORD_PHC: &str = "$argon2id$v=19$m=19456,t=2,p=1$P/qEkLVC2cUEZbYOairU+A$vdXpqQIqcVifQO1OXrCfvkTanUQLmtw2L7jEc6UDbiA";

/// The default token every write helper attaches: a caller holding every
/// content-writing role.
pub const DEFAULT_ROLES: [Role; 3] = [Role::Uploader, Role::Moderator, Role::Admin];

// TODO: migrate this to axum_test::TestServer/TestResponse.
pub struct TestApp {
    pub pool: PgPool,
    pub router: Router,
    pub s3: aws_sdk_s3::Client,
    pub covers_bucket: String,
    pub release_pages_bucket: String,
    pub jwt: Jwt,
    pub memory: MemoryStore,
    pub hasher: Hasher,
}

impl TestApp {
    pub async fn new() -> TestApp {
        LazyLock::force(&TRACING);

        let db_name = Uuid::now_v7().to_string();
        let covers_bucket = format!("covers-{}", Uuid::now_v7());
        let release_pages_bucket = format!("release-pages-{}", Uuid::now_v7());
        let keys = &*KEYS;
        let access_private = keys.access_private.to_string_lossy().into_owned();
        let access_public = keys.access_public.to_string_lossy().into_owned();
        let refresh_private = keys.refresh_private.to_string_lossy().into_owned();
        let refresh_public = keys.refresh_public.to_string_lossy().into_owned();
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
            ("APP_REDIS__URL", "redis://localhost:6379"),
            ("APP_AUTH__JWT_ACCESS__PRIVATE_KEY", access_private.as_str()),
            ("APP_AUTH__JWT_ACCESS__PUBLIC_KEY", access_public.as_str()),
            ("APP_AUTH__JWT_ACCESS__TTL", "900"),
            (
                "APP_AUTH__JWT_REFRESH__PRIVATE_KEY",
                refresh_private.as_str(),
            ),
            ("APP_AUTH__JWT_REFRESH__PUBLIC_KEY", refresh_public.as_str()),
            ("APP_AUTH__JWT_REFRESH__TTL", "2592000"),
            ("APP_AUTH__PEPPER__KEY", "test-pepper"),
            // Minimum viable Argon2 cost.
            ("APP_AUTH__PASSWORD__M_COST", "8"),
            ("APP_AUTH__PASSWORD__T_COST", "1"),
            ("APP_AUTH__PASSWORD__P_COST", "1"),
            // Ignored pairs
            ("APP_ADDR", "0.0.0.0:0"),
            ("APP_LOG_LEVEL", "info"),
        ]);
        let pool = Self::configure_db(&cfg.database).await;
        let s3 = object_storage::client(&cfg.object_storage).await;
        let app = App::build(&cfg).await;

        let jwt = Jwt::load(&cfg.auth.jwt_access, &cfg.auth.jwt_refresh);
        let redis = memory_storage::connection(&cfg.redis).await;
        let memory = MemoryStore::new(redis, cfg.auth.jwt_refresh.ttl);
        let hasher = Hasher::new(
            cfg.auth.password.m_cost,
            cfg.auth.password.t_cost,
            cfg.auth.password.p_cost,
            b"test-pepper",
        )
        .unwrap();

        TestApp {
            pool,
            router: app.router(),
            s3,
            covers_bucket,
            release_pages_bucket,
            jwt,
            memory,
            hasher,
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

    /// Dispatch a request, attaching a default all-roles bearer token when it
    /// carries no `Authorization` header of its own.
    pub async fn send(&self, mut req: Request<Body>) -> Response {
        if !req.headers().contains_key(header::AUTHORIZATION) {
            let token = self.access_token(Uuid::now_v7(), Uuid::now_v7(), &DEFAULT_ROLES);
            req.headers_mut().insert(
                header::AUTHORIZATION,
                format!("Bearer {token}").parse().unwrap(),
            );
        }

        self.router.clone().oneshot(req).await.unwrap()
    }

    /// Dispatch a request exactly as given -- no token injection.
    pub async fn send_raw(&self, req: Request<Body>) -> Response {
        self.router.clone().oneshot(req).await.unwrap()
    }

    pub fn access_token(&self, sub: Uuid, sid: Uuid, roles: &[Role]) -> String {
        self.jwt
            .issue_access(sub, sid, roles, time::OffsetDateTime::now_utc())
            .expect("failed to issue a test access token")
    }

    pub fn bearer(&self, sub: Uuid, sid: Uuid, roles: &[Role]) -> String {
        format!("Bearer {}", self.access_token(sub, sid, roles))
    }

    pub async fn insert_user(&self, user: &UserQuery) {
        let roles: Vec<String> = user
            .roles
            .as_slice()
            .iter()
            .map(|r| r.as_ref().to_owned())
            .collect();

        sqlx::query!(
            r#"
insert into users(id, email, username, password_hash, roles, verified_at, created_at)
values($1, $2, $3, $4, $5, $6, $7);
        "#,
            user.id,
            user.email.as_ref(),
            user.username.as_ref(),
            user.password_hash.as_ref(),
            &roles,
            user.verified_at.map(Timestamp::into_inner),
            user.created_at.into_inner(),
        )
        .execute(&self.pool)
        .await
        .expect("failed to insert factory user");
    }

    /// Write a session blob straight into Redis. `created_at` is caller-set so
    /// the 24h revoke rule is exercised without a clock seam.
    pub async fn insert_session(&self, sub: Uuid, sid: Uuid, created_at: time::OffsetDateTime) {
        let session = Session {
            sid,
            jti: self
                .hasher
                .keyed_jti_hash(Uuid::now_v7())
                .expect("keyed jti hash"),
            ua: None,
            ip: None,
            created_at: created_at.into(),
            updated_at: created_at.into(),
        };

        self.memory
            .put_session(sub, session)
            .await
            .expect("failed to insert factory session");
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

    pub async fn insert_book_with_visibility(&self, visibility: BookVisibility) -> Uuid {
        let book: BookQuery = BookFaker {
            visibility,
            ..Default::default()
        }
        .fake();

        self.insert_book(&book).await;

        book.id
    }

    pub async fn fetch_book_visibility_state(&self, id: Uuid) -> BookVisibilityState {
        let row = sqlx::query!(
            r#"
select b.visibility, b.note, b.updated_at
from books b
where b.id = $1;
        "#,
            id
        )
        .fetch_one(&self.pool)
        .await
        .expect("failed to read book visibility state");

        BookVisibilityState {
            visibility: row.visibility,
            note: row.note,
            updated_at: row.updated_at,
        }
    }

    pub async fn put_visibility(&self, id: Uuid, visibility: &str, note: Option<&str>) -> Response {
        let body = match note {
            Some(note) => serde_json::json!({ "visibility": visibility, "note": note }),
            None => serde_json::json!({ "visibility": visibility }),
        }
        .to_string();
        let req = Request::put(format!("/books/{id}/visibility"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .unwrap();

        self.send(req).await
    }

    pub async fn get_books(&self, path: &str) -> Response {
        let req = Request::get(path).body(Body::empty()).unwrap();

        self.send_raw(req).await
    }

    pub async fn get_body(&self, path: &str) -> (StatusCode, String) {
        let req = Request::get(path).body(Body::empty()).unwrap();
        let resp = self.send_raw(req).await;
        let status = resp.status();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();

        (status, String::from_utf8_lossy(&bytes).into_owned())
    }

    pub async fn get_as_moderator(&self, path: &str) -> Response {
        let req = Request::get(path)
            .header(
                header::AUTHORIZATION,
                self.bearer(Uuid::now_v7(), Uuid::now_v7(), &[Role::Moderator]),
            )
            .body(Body::empty())
            .unwrap();

        self.send_raw(req).await
    }

    pub async fn insert_book(&self, b: &BookQuery) {
        sqlx::query!(
            r#"
insert into books(id, name, description, publication_year, content_rating, status, kind,
                  publication_language, publication_demographic, visibility, note, submitted_at,
                  updated_at, created_at)
values($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14);
        "#,
            b.id,
            b.name.as_ref(),
            b.description.as_ref(),
            b.publication_year,
            b.content_rating.id,
            b.status.as_ref() as _,
            b.kind.as_ref() as _,
            b.publication_language.id,
            b.publication_demographic.as_ref() as _,
            b.visibility.as_ref() as _,
            b.note.as_ref().map(AsRef::as_ref) as Option<&str>,
            b.submitted_at,
            b.updated_at,
            b.created_at
        )
        .execute(&self.pool)
        .await
        .expect("failed to insert factory book");

        let mut label_ids: Vec<Uuid> = Vec::with_capacity(b.labels.len());
        for l in b.labels.as_slice() {
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
        for l in b.links.as_slice() {
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
        for t in b.titles.as_slice() {
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

        let mut creator_ids: Vec<Uuid> = Vec::with_capacity(b.creators.len());
        let mut creator_first_names: Vec<String> = Vec::with_capacity(b.creators.len());
        let mut creator_last_names: Vec<String> = Vec::with_capacity(b.creators.len());
        let mut creator_created_ats: Vec<time::OffsetDateTime> =
            Vec::with_capacity(b.creators.len());
        let mut credit_creator_ids: Vec<Uuid> = Vec::new();
        let mut credit_roles: Vec<String> = Vec::new();
        for c in b.creators.as_slice() {
            creator_ids.push(c.id);
            creator_first_names.push(c.first_name.as_ref().to_owned());
            creator_last_names.push(c.last_name.as_ref().to_owned());
            creator_created_ats.push(c.created_at);
            for role in &c.roles {
                credit_creator_ids.push(c.id);
                credit_roles.push(role.as_ref().to_owned());
            }
        }

        if !creator_ids.is_empty() {
            sqlx::query!(
                r#"
insert into creators(id, first_name, last_name, created_at)
select c.id, c.first_name, c.last_name, c.created_at
from unnest($1::uuid[], $2::text[], $3::text[], $4::timestamptz[]) as c(id, first_name, last_name, created_at);
            "#,
                &creator_ids,
                &creator_first_names,
                &creator_last_names,
                &creator_created_ats
            )
            .execute(&self.pool)
            .await
            .expect("failed to insert factory book creators");

            sqlx::query!(
                r#"
insert into book_creators(book_id, creator_id, role)
select $1, credit.creator_id, credit.role
from unnest($2::uuid[], $3::text[]) as credit(creator_id, role);
            "#,
                b.id,
                &credit_creator_ids,
                &credit_roles
            )
            .execute(&self.pool)
            .await
            .expect("failed to insert factory book creator credits");
        }
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

    pub async fn fetch_book(&self, id: Uuid) -> BookQuery {
        let row = sqlx::query!(
            r#"
select b.name, b.description, b.publication_year, b.status, b.kind,
       b.publication_demographic, b.visibility, b.note, b.submitted_at, b.updated_at,
       b.created_at,
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

        let labels: Vec<Label> = sqlx::query!(
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

        let links: Vec<BookLink> = sqlx::query!(
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

        let titles: Vec<AlternativeTitle> = sqlx::query!(
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

        let creators: Vec<CreatorQuery> = sqlx::query!(
            r#"
select c.id, c.first_name, c.last_name,
       array_agg(bc.role order by bc.role) as "roles!", c.created_at
from book_creators bc
join creators c on c.id = bc.creator_id
where bc.book_id = $1
group by c.id
order by c.id;
        "#,
            id
        )
        .fetch_all(&self.pool)
        .await
        .expect("failed to read book creators")
        .into_iter()
        .map(|r| CreatorQuery {
            id: r.id,
            first_name: Name::try_from(r.first_name).expect("stored first name must be valid"),
            last_name: Name::try_from(r.last_name).expect("stored last name must be valid"),
            roles: r
                .roles
                .into_iter()
                .map(|role| role.parse().expect("stored creator role must be valid"))
                .collect(),
            created_at: r.created_at,
        })
        .collect();

        BookQuery {
            id,
            name: BookName::try_from(row.name).expect("stored name must be valid"),
            description: Text::try_from(row.description).expect("stored description must be valid"),
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
            publication_demographic: row
                .publication_demographic
                .parse()
                .expect("stored publication demographic must be valid"),
            labels: labels.try_into().expect("too many labels in test fixture"),
            links: links.try_into().expect("too many links in test fixture"),
            titles: titles.try_into().expect("too many titles in test fixture"),
            creators: creators
                .try_into()
                .expect("too many creators in test fixture"),
            visibility: row
                .visibility
                .parse()
                .expect("stored visibility must be valid"),
            note: row
                .note
                .map(Text::try_from)
                .transpose()
                .expect("stored note must be valid"),
            submitted_at: row.submitted_at,
            updated_at: row.updated_at,
            created_at: row.created_at,
        }
    }

    pub async fn insert_random_book(&self) -> Uuid {
        let book: BookQuery = BookFaker::default().fake();
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
            book_id,
            extension: r
                .extension
                .parse()
                .expect("stored cover extension must be valid"),
            is_main: r.is_main,
            url: CoverUrl { cover_id: r.id }.into(),
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
        for l in c.localizations.as_slice() {
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

        let localizations: Vec<ChapterLocalization> = sqlx::query!(
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
            localizations: localizations
                .try_into()
                .expect("too many localizations in test fixture"),
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
insert into creators(id, first_name, last_name, created_at)
values($1, $2, $3, $4);
        "#,
            c.id,
            c.first_name.as_ref(),
            c.last_name.as_ref(),
            c.created_at
        )
        .execute(&self.pool)
        .await
        .expect("failed to insert factory creator");
    }

    pub async fn credit_creator(&self, book_id: Uuid, creator_id: Uuid, role: CreatorRole) {
        sqlx::query!(
            r#"
insert into book_creators(book_id, creator_id, role)
values($1, $2, $3);
        "#,
            book_id,
            creator_id,
            role.as_ref()
        )
        .execute(&self.pool)
        .await
        .expect("failed to insert factory book creator credit");
    }

    pub async fn post_cover(&self, book_id: Uuid, image: &[u8]) -> Response {
        let form = Self::multipart_body(&[image]);
        let req = Request::post(format!("/books/{book_id}/covers"))
            .header(header::CONTENT_TYPE, form.content_type())
            .body(Body::from(form))
            .unwrap();

        self.send(req).await
    }

    pub async fn get_covers(&self, book_id: Uuid) -> Response {
        let req = Request::get(format!("/books/{book_id}/covers"))
            .body(Body::empty())
            .unwrap();

        self.send_raw(req).await
    }

    pub async fn get_cover_image(&self, cover_id: Uuid) -> Response {
        let req = Request::get(format!("/covers/{cover_id}/image"))
            .body(Body::empty())
            .unwrap();

        self.send_raw(req).await
    }

    pub async fn put_main_cover(&self, book_id: Uuid, cover_id: Uuid) -> StatusCode {
        let body = serde_json::json!({ "coverId": cover_id }).to_string();
        let req = Request::put(format!("/books/{book_id}/main-cover"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .unwrap();

        self.send(req).await.status()
    }

    pub async fn delete_cover(&self, cover_id: Uuid) -> Response {
        let req = Request::delete(format!("/covers/{cover_id}"))
            .body(Body::empty())
            .unwrap();

        self.send(req).await
    }

    pub async fn get_creator(&self, id: Uuid) -> Response {
        let req = Request::get(format!("/creators/{id}"))
            .body(Body::empty())
            .unwrap();

        self.send_raw(req).await
    }

    pub async fn delete_creator(&self, id: Uuid) -> Response {
        let req = Request::delete(format!("/creators/{id}"))
            .body(Body::empty())
            .unwrap();

        self.send(req).await
    }

    pub async fn post_feedback(&self, body: serde_json::Value) -> Response {
        let req = Request::post("/feedbacks")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();

        self.send(req).await
    }

    pub async fn insert_feedback(&self, f: &Feedback) {
        sqlx::query!(
            r#"
insert into feedback(id, kind, status, email, note, book_id, updated_at, created_at)
values($1, $2, $3, $4, $5, $6, $7, $8);
        "#,
            f.id,
            f.kind.as_ref(),
            f.status.as_ref(),
            f.email.as_ref(),
            f.note.as_ref(),
            f.book_id,
            f.updated_at,
            f.created_at
        )
        .execute(&self.pool)
        .await
        .expect("failed to insert factory feedback");
    }

    pub async fn fetch_feedback(&self, id: Uuid) -> Feedback {
        let row = sqlx::query!(
            r#"
select f.id, f.kind, f.status, f.email, f.note, f.book_id, f.updated_at, f.created_at
from feedback f
where f.id = $1;
        "#,
            id
        )
        .fetch_one(&self.pool)
        .await
        .expect("failed to read feedback");

        Feedback {
            id: row.id,
            kind: row.kind.parse().unwrap(),
            status: row.status.parse().unwrap(),
            email: Email::try_from(row.email).unwrap(),
            note: Text::try_from(row.note).unwrap(),
            book_id: row.book_id,
            updated_at: row.updated_at,
            created_at: row.created_at,
        }
    }

    pub async fn count_feedback(&self) -> i64 {
        sqlx::query_scalar!(r#"select count(*) as "count!" from feedback"#)
            .fetch_one(&self.pool)
            .await
            .expect("failed to count feedback")
    }

    pub async fn get_feedbacks(&self, path: &str) -> Response {
        let req = Request::get(path)
            .header(
                header::AUTHORIZATION,
                self.bearer(Uuid::now_v7(), Uuid::now_v7(), &[Role::Moderator]),
            )
            .body(Body::empty())
            .unwrap();

        self.send_raw(req).await
    }

    pub async fn get_feedback(&self, id: Uuid) -> Response {
        let req = Request::get(format!("/feedbacks/{id}"))
            .header(
                header::AUTHORIZATION,
                self.bearer(Uuid::now_v7(), Uuid::now_v7(), &[Role::Moderator]),
            )
            .body(Body::empty())
            .unwrap();

        self.send_raw(req).await
    }

    pub async fn put_feedback_status(&self, id: Uuid, status: &str) -> StatusCode {
        let body = serde_json::json!({ "status": status }).to_string();
        let req = Request::put(format!("/feedbacks/{id}/status"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .unwrap();

        self.send(req).await.status()
    }

    pub async fn delete_chapter(&self, id: Uuid) -> Response {
        let req = Request::delete(format!("/chapters/{id}"))
            .body(Body::empty())
            .unwrap();

        self.send(req).await
    }

    pub async fn get_books_page(&self, path: &str) -> RespWrapper<Vec<BookQuery>, String> {
        let req = Request::get(path).body(Body::empty()).unwrap();
        let resp = self.send_raw(req).await;
        assert_eq!(resp.status(), StatusCode::OK, "{path}");

        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).expect("failed to parse book page")
    }

    pub async fn seed_facet_corpus(&self) -> Vec<BookQuery> {
        const ROWS: [FacetRow; 12] = [
            (
                &[ACTION, ROMANCE],
                BookKind::Manga,
                BookStatus::Ongoing,
                PublicationDemographic::Shounen,
                0,
                0,
            ),
            (
                &[ACTION],
                BookKind::Manga,
                BookStatus::Completed,
                PublicationDemographic::Shounen,
                0,
                0,
            ),
            (
                &[FANTASY, ISEKAI],
                BookKind::Manhwa,
                BookStatus::Ongoing,
                PublicationDemographic::Seinen,
                1,
                1,
            ),
            (
                &[ACTION, ROMANCE, FANTASY],
                BookKind::Manhwa,
                BookStatus::Completed,
                PublicationDemographic::Josei,
                1,
                1,
            ),
            (
                &[FANTASY, LONG_STRIP],
                BookKind::Manhua,
                BookStatus::Hiatus,
                PublicationDemographic::Shoujo,
                2,
                2,
            ),
            (
                &[ROMANCE],
                BookKind::Manhua,
                BookStatus::Cancelled,
                PublicationDemographic::Kids,
                2,
                2,
            ),
            (
                &[],
                BookKind::Manga,
                BookStatus::Ongoing,
                PublicationDemographic::Seinen,
                3,
                3,
            ),
            (
                &[MAFIA, ZOMBIES],
                BookKind::Manhwa,
                BookStatus::Hiatus,
                PublicationDemographic::Shoujo,
                3,
                3,
            ),
            (
                &[SCHOOL_LIFE],
                BookKind::Manhua,
                BookStatus::Completed,
                PublicationDemographic::Josei,
                0,
                4,
            ),
            (
                &[ACTION, FANTASY],
                BookKind::Manga,
                BookStatus::Cancelled,
                PublicationDemographic::Kids,
                1,
                4,
            ),
            (
                &[ISEKAI],
                BookKind::Manhwa,
                BookStatus::Ongoing,
                PublicationDemographic::Shounen,
                2,
                0,
            ),
            (
                &[ZOMBIES],
                BookKind::Manhua,
                BookStatus::Completed,
                PublicationDemographic::Seinen,
                3,
                1,
            ),
        ];

        let mut books = Vec::with_capacity(ROWS.len());
        for (i, (label_ids, kind, status, demographic, rating, language)) in
            ROWS.into_iter().enumerate()
        {
            let book: BookQuery = BookFaker {
                links: 0..=0,
                titles: 0..=0,
                creators: 0..=0,
                exact_labels: Some(labels(label_ids)),
                kind: Some(kind),
                status: Some(status),
                publication_demographic: Some(demographic),
                content_rating: Some(CONTENT_RATINGS[rating].clone()),
                publication_language: Some(LANGUAGES[language].clone()),
                publication_year: Some(2000 + i as i16),
                created_at: Some(day(i as i64)),
                ..Default::default()
            }
            .fake();

            self.insert_book(&book).await;
            books.push(book);
        }

        books
    }

    pub async fn seed_range_corpus(&self) -> Vec<BookQuery> {
        const YEARS: [i16; 5] = [2010, 2012, 2015, 2018, 2020];

        let mut books = Vec::with_capacity(YEARS.len());
        for year in YEARS {
            let book: BookQuery = BookFaker {
                labels: 0..=0,
                links: 0..=0,
                titles: 0..=0,
                creators: 0..=0,
                publication_year: Some(year),
                created_at: Some(day(i64::from(year) - 2000)),
                ..Default::default()
            }
            .fake();

            self.insert_book(&book).await;
            books.push(book);
        }

        books
    }

    pub async fn seed_sort_corpus(&self) -> Vec<BookQuery> {
        const ROWS: [(&str, i16, i64); 4] = [
            ("Alpha", 2020, 4),
            ("Bravo", 2010, 1),
            ("Charlie", 2015, 3),
            ("Delta", 2005, 2),
        ];

        let mut books = Vec::with_capacity(ROWS.len());
        for (name, year, created) in ROWS {
            let book: BookQuery = BookFaker {
                labels: 0..=0,
                links: 0..=0,
                titles: 0..=0,
                creators: 0..=0,
                name: Some(name.to_owned()),
                publication_year: Some(year),
                created_at: Some(day(created)),
                ..Default::default()
            }
            .fake();

            self.insert_book(&book).await;
            books.push(book);
        }

        books
    }

    pub async fn seed_tie_corpus(&self, n: usize) -> Vec<BookQuery> {
        let mut books = Vec::with_capacity(n);
        for _ in 0..n {
            let book: BookQuery = BookFaker {
                labels: 0..=0,
                links: 0..=0,
                titles: 0..=0,
                creators: 0..=0,
                name: Some("Tied".to_owned()),
                publication_year: Some(2020),
                created_at: Some(day(0)),
                ..Default::default()
            }
            .fake();

            self.insert_book(&book).await;
            books.push(book);
        }

        books.sort_by_key(|b| b.id);

        books
    }

    pub async fn seed_translated_corpus(&self) -> Vec<BookQuery> {
        let japanese = LANGUAGES[0].clone();
        let english = LANGUAGES[3].clone();
        let russian = LANGUAGES[4].clone();

        let rows: [(Language, BookVisibility, Vec<Language>); 6] = [
            (
                japanese.clone(),
                BookVisibility::Listed,
                vec![english.clone()],
            ),
            (
                japanese.clone(),
                BookVisibility::Listed,
                vec![english.clone(), english.clone()],
            ),
            (japanese.clone(), BookVisibility::Listed, vec![russian]),
            (english.clone(), BookVisibility::Listed, vec![]),
            (japanese.clone(), BookVisibility::Listed, vec![]),
            (japanese, BookVisibility::Hidden, vec![english]),
        ];

        let mut books = Vec::with_capacity(rows.len());
        for (language, visibility, releases) in rows {
            let book: BookQuery = BookFaker {
                labels: 0..=0,
                links: 0..=0,
                titles: 0..=0,
                creators: 0..=0,
                publication_language: Some(language),
                visibility,
                ..Default::default()
            }
            .fake();

            self.insert_book(&book).await;

            for release_language in releases {
                let chapter_id = self.insert_random_chapter(book.id).await;
                self.insert_release(chapter_id, release_language.id).await;
            }

            books.push(book);
        }

        books
    }

    pub async fn get_labels(&self, path: &str) -> Vec<Label> {
        let req = Request::get(path).body(Body::empty()).unwrap();
        let resp = self.send_raw(req).await;
        assert_eq!(resp.status(), StatusCode::OK, "{path}");

        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let wrapper: RespWrapper<Vec<Label>> = serde_json::from_slice(&bytes).unwrap();

        wrapper.data
    }

    pub async fn get_chapters(&self, path: String) -> RespWrapper<Vec<Chapter>> {
        let req = Request::get(path).body(Body::empty()).unwrap();
        let resp = self.send_raw(req).await;
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
insert into chapter_releases(id, chapter_id, book_id, language_id, version)
select $1, $2, c.book_id, $3, 1
from chapters c
where c.id = $2;
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
        sort_order: Option<i32>,
        image: &[u8],
    ) -> Uuid {
        let id = Uuid::now_v7();
        let extension =
            ImageExtension::try_from(image).expect("factory page must be a supported image");

        sqlx::query!(
            r#"
insert into chapter_pages(id, release_id, book_id, sort_order, extension)
select $1, $2, cr.book_id, $3, $4
from chapter_releases cr
where cr.id = $2;
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
            .body(ByteStream::from(image.to_vec()))
            .send()
            .await
            .expect("failed to upload factory chapter page");

        id
    }

    pub async fn insert_staged_pages(&self, release_id: Uuid, parts: &[&[u8]]) -> Vec<Uuid> {
        let mut ids = Vec::with_capacity(parts.len());
        for part in parts {
            ids.push(self.insert_page(release_id, None, part).await);
        }

        ids
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

    pub async fn fetch_release_id(&self, release_id: Uuid) -> Option<Uuid> {
        sqlx::query_scalar!(
            r#"
select id
from chapter_releases
where id = $1;
        "#,
            release_id
        )
        .fetch_optional(&self.pool)
        .await
        .expect("failed to read chapter release id")
    }

    pub async fn fetch_page_order(&self, release_id: Uuid) -> Vec<(Uuid, Option<i32>)> {
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

    pub async fn fetch_committed_page_ids(&self, release_id: Uuid) -> Vec<Uuid> {
        sqlx::query_scalar!(
            r#"
select id
from chapter_pages
where release_id = $1 and sort_order is not null
order by sort_order;
        "#,
            release_id
        )
        .fetch_all(&self.pool)
        .await
        .expect("failed to read committed chapter page order")
    }

    pub async fn post_chapter(&self, book_id: Uuid, number: f32) -> Response {
        let body = serde_json::json!({ "number": number }).to_string();
        let req = Request::post(format!("/books/{book_id}/chapters"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .unwrap();

        self.send(req).await
    }

    pub async fn post_release(&self, chapter_id: Uuid, language_id: Uuid) -> Response {
        let body = serde_json::json!({ "languageId": language_id }).to_string();
        let req = Request::post(format!("/chapters/{chapter_id}/releases"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .unwrap();

        self.send(req).await
    }

    pub async fn get_releases(&self, chapter_id: Uuid) -> Response {
        let req = Request::get(format!("/chapters/{chapter_id}/releases"))
            .body(Body::empty())
            .unwrap();

        self.send_raw(req).await
    }

    pub async fn get_release(&self, id: Uuid) -> Response {
        let req = Request::get(format!("/releases/{id}"))
            .body(Body::empty())
            .unwrap();

        self.send_raw(req).await
    }

    pub async fn post_upload_pages(&self, release_id: Uuid, parts: &[&[u8]]) -> Response {
        let form = Self::multipart_body(parts);
        let req = Request::post(format!("/releases/{release_id}/upload"))
            .header(header::CONTENT_TYPE, form.content_type())
            .body(Body::from(form))
            .unwrap();

        self.send(req).await
    }

    pub async fn post_commit(&self, release_id: Uuid, page_order: &[Uuid]) -> Response {
        let body = serde_json::json!({ "pageOrder": page_order }).to_string();
        let req = Request::post(format!("/releases/{release_id}/commit"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .unwrap();

        self.send(req).await
    }

    pub async fn get_pages(&self, release_id: Uuid) -> Response {
        let req = Request::get(format!("/releases/{release_id}/pages"))
            .body(Body::empty())
            .unwrap();

        self.send_raw(req).await
    }

    pub async fn get_staged(&self, release_id: Uuid) -> Response {
        let req = Request::get(format!("/releases/{release_id}/pages?status=Staged"))
            .body(Body::empty())
            .unwrap();

        self.send_raw(req).await
    }

    pub async fn get_page_image(&self, release_id: Uuid, page_id: Uuid) -> Response {
        let req = Request::get(format!("/releases/{release_id}/pages/{page_id}/image"))
            .body(Body::empty())
            .unwrap();

        self.send_raw(req).await
    }

    pub async fn get_page(&self, release_id: Uuid, page_number: i32) -> Response {
        let req = Request::get(format!("/releases/{release_id}/pages/{page_number}"))
            .body(Body::empty())
            .unwrap();

        self.send_raw(req).await
    }

    pub async fn delete_book(&self, id: Uuid) -> Response {
        let req = Request::delete(format!("/books/{id}"))
            .body(Body::empty())
            .unwrap();

        self.send(req).await
    }

    pub async fn delete_release(&self, id: Uuid) -> Response {
        let req = Request::delete(format!("/releases/{id}"))
            .body(Body::empty())
            .unwrap();

        self.send(req).await
    }
}
