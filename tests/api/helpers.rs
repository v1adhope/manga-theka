use std::sync::LazyLock;
use std::time::Duration;

use axum::Router;
use fake::Dummy;
use fake::Fake;
use fake::faker::name::en::FirstName;
use fake::rand::RngExt;
use manga_theka::{
    config::{Config, Database},
    entity::{Creator, CreatorRole, Label, Name},
    startup::App,
    telemetry,
};
use serde::Deserialize;
use sqlx::{AssertSqlSafe, ConnectOptions, Connection, Executor, PgConnection, PgPool};
use time::OffsetDateTime;
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

    pub async fn insert_label(&self, l: &Label) {
        sqlx::query!(
            r#"
insert into labels(id, name, kind)
values($1, $2, $3);
        "#,
            l.id,
            l.name,
            l.kind.as_ref() as _
        )
        .execute(&self.pool)
        .await
        .expect("failed to insert factory label");
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
