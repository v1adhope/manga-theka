use std::sync::LazyLock;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use fake::Dummy;
use fake::Fake;
use fake::faker::name::en::FirstName;
use fake::rand::RngExt;
use http_body_util::BodyExt;
use manga_theka::{
    config::{Config, Database},
    entity::{Creator, CreatorRole, Name},
    startup::App,
    telemetry,
};
use serde::Deserialize;
use sqlx::{AssertSqlSafe, ConnectOptions, Connection, Executor, PgConnection, PgPool};
use time::OffsetDateTime;
use tower::ServiceExt;
use tracing_log::log::LevelFilter;
use uuid::Uuid;

#[derive(Deserialize, Debug)]
pub struct RespWrapper<T> {
    pub data: T,
    #[serde(rename = "nextCursor", default)]
    pub next_cursor: Option<uuid::Uuid>,
}

static TRACING: LazyLock<()> = LazyLock::new(|| {
    telemetry::init_subscriber("info".into());
});

pub struct TestApp {
    pub pool: PgPool,
    pub router: Router,
}

#[derive(Debug)]
pub struct BookRefs {
    pub author: Uuid,
    pub artist: Uuid,
    pub content_rating: Uuid,
    pub language: Uuid,
    pub other_language: Uuid,
}

impl TestApp {
    pub async fn new() -> TestApp {
        LazyLock::force(&TRACING);

        let db_name = Uuid::now_v7().to_string();
        let cfg = Config::from_env_pairs([
            ("APP_DATABASE__USERNAME", "postgres"),
            ("APP_DATABASE__PASSWORD", "postgres"),
            ("APP_DATABASE__HOST", "localhost"),
            ("APP_DATABASE__PORT", "5432"),
            ("APP_DATABASE__DATABASE_NAME", db_name.as_str()),
            // Ignored pairs
            ("APP_ADDR", "0.0.0.0:0"),
            ("APP_LOG_LEVEL", "info"),
        ]);
        let pool = Self::configure_db(&cfg.database).await;
        let app = App::build(&cfg).await;

        TestApp {
            pool,
            router: app.router(),
        }
    }

    async fn configure_db(cfg: &Database) -> PgPool {
        let conn_cfg = cfg
            .without_db()
            .log_slow_statements(LevelFilter::Warn, Duration::from_secs(3));
        let mut conn = PgConnection::connect_with(&conn_cfg)
            .await
            .expect("failed to connect to Postgres without database");

        conn.execute(AssertSqlSafe(format!(
            r#"CREATE DATABASE "{}""#,
            cfg.database_name
        )))
        .await
        .expect("failed to create database");

        let pool = PgPool::connect_with(cfg.with_db())
            .await
            .expect("failed to connect to Postgres");

        sqlx::migrate!()
            .run(&pool)
            .await
            .expect("failed to migrate the database");

        pool
    }

    /// Seeds the rows a book must reference and hands back their ids.
    pub async fn book_refs(&self) -> BookRefs {
        let author: Creator = CreatorFaker.fake();
        let artist: Creator = CreatorFaker.fake();
        self.insert_creator(&author).await;
        self.insert_creator(&artist).await;

        let content_rating = sqlx::query_scalar!("select id from content_ratings order by name")
            .fetch_one(&self.pool)
            .await
            .expect("failed to pick a seeded content rating");
        let languages = sqlx::query_scalar!("select id from languages order by name")
            .fetch_all(&self.pool)
            .await
            .expect("failed to pick seeded languages");

        BookRefs {
            author: author.id,
            artist: artist.id,
            content_rating,
            language: languages[0],
            other_language: languages[1],
        }
    }

    /// Returns `n` ids from the label catalog seeded by the migrations.
    pub async fn label_ids(&self, n: i64) -> Vec<Uuid> {
        sqlx::query_scalar!("select id from labels order by name limit $1", n)
            .fetch_all(&self.pool)
            .await
            .expect("failed to pick seeded labels")
    }

    /// The minimal valid `POST /books` body, with no attached arrays.
    pub fn book_body(refs: &BookRefs) -> serde_json::Value {
        serde_json::json!({
            "name": "Berserk",
            "description": "A wandering swordsman and his enormous sword.",
            "publicationYear": 1989,
            "contentRating": refs.content_rating,
            "status": "Ongoing",
            "type": "Manga",
            "publicationLanguage": refs.language,
            "author": refs.author,
            "artist": refs.artist,
        })
    }

    /// Row counts of the three tables a book write replaces wholesale, as
    /// `(labels, links, titles)`.
    pub async fn count_book_relations(&self, id: Uuid) -> (i64, i64, i64) {
        let labels = sqlx::query_scalar!("select count(*) from book_labels where book_id = $1", id)
            .fetch_one(&self.pool)
            .await
            .expect("failed to count book labels");
        let links = sqlx::query_scalar!("select count(*) from book_links where book_id = $1", id)
            .fetch_one(&self.pool)
            .await
            .expect("failed to count book links");
        let titles = sqlx::query_scalar!("select count(*) from book_titles where book_id = $1", id)
            .fetch_one(&self.pool)
            .await
            .expect("failed to count book titles");

        (
            labels.unwrap_or_default(),
            links.unwrap_or_default(),
            titles.unwrap_or_default(),
        )
    }

    /// Stores a book through the API and returns its id.
    pub async fn insert_book(&self, body: &serde_json::Value) -> Uuid {
        let req = Request::post("/books")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();

        let resp = self.router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED, "failed to insert book");

        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        Uuid::parse_str(v["data"]["id"].as_str().unwrap()).unwrap()
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
}

pub struct CreatorRoleFaker;

impl Dummy<CreatorRoleFaker> for CreatorRole {
    fn dummy_with_rng<R: RngExt + ?Sized>(_config: &CreatorRoleFaker, rng: &mut R) -> Self {
        if rng.random() {
            CreatorRole::Artist
        } else {
            CreatorRole::Author
        }
    }
}

pub struct NameFaker;

impl Dummy<NameFaker> for Name {
    fn dummy_with_rng<R: RngExt + ?Sized>(_config: &NameFaker, rng: &mut R) -> Self {
        let name = FirstName().fake_with_rng::<String, R>(rng);
        Name::try_from(name).unwrap()
    }
}

pub struct CreatorFaker;

impl Dummy<CreatorFaker> for Creator {
    fn dummy_with_rng<R: RngExt + ?Sized>(_config: &CreatorFaker, rng: &mut R) -> Self {
        Creator {
            id: Uuid::now_v7(),
            first_name: NameFaker.fake_with_rng(rng),
            last_name: NameFaker.fake_with_rng(rng),
            role: CreatorRoleFaker.fake_with_rng(rng),
            created_at: OffsetDateTime::now_utc(),
        }
    }
}
