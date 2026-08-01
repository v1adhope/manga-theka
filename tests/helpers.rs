use std::sync::LazyLock;

use axum::Router;
use manga_theka::{
    config::{Config, Database},
    startup::App,
    telemetry,
};
use sqlx::{AssertSqlSafe, Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
static TRACING: LazyLock<()> = LazyLock::new(|| {
    telemetry::init_subsciber("info".into());
});

pub struct TestApp {
    #[allow(dead_code)]
    pub pool: PgPool,
    pub router: Router,
}

pub async fn spawn_app() -> TestApp {
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
    let pool = configure_database(&cfg.database).await;
    let app = App::build(cfg).await;

    TestApp {
        pool,
        router: app.router(),
    }
}

async fn configure_database(cfg: &Database) -> PgPool {
    let mut conn = PgConnection::connect_with(&cfg.without_db())
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
