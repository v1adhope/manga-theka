use std::sync::LazyLock;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, header};
use axum::response::Response;
use manga_theka::{
    config::{Config, Database},
    database,
    entity::Role,
    hasher::Hasher,
    jwt::Jwt,
    memory_storage::{self, MemoryStore},
    object_storage,
    startup::App,
    telemetry,
};
use sqlx::{AssertSqlSafe, ConnectOptions, Connection, Executor, PgConnection, PgPool};
use tower::ServiceExt;
use tracing_log::log::LevelFilter;
use uuid::Uuid;

static TRACING: LazyLock<()> = LazyLock::new(|| {
    telemetry::init_subscriber("info");
});

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

pub const DEFAULT_ROLES: [Role; 3] = [Role::Uploader, Role::Moderator, Role::Admin];

// TODO: migrate this to axum_test::TestServer/TestResponse?
pub struct TestApp {
    pub pool: PgPool,
    pub router: Router,
    pub s3: aws_sdk_s3::Client,
    pub covers_bucket: String,
    pub release_pages_bucket: String,
    pub jwt: Jwt,
    pub memory: MemoryStore,
    pub hasher: Hasher,
    pub caller_id: Uuid,
    pub caller_sid: Uuid,
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

        let test_app = TestApp {
            pool,
            router: app.router(),
            s3,
            covers_bucket,
            release_pages_bucket,
            jwt,
            memory,
            hasher,
            caller_id: Uuid::now_v7(),
            caller_sid: Uuid::now_v7(),
        };
        test_app.db_seed_caller().await;

        test_app
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

    pub async fn send_authed(&self, mut req: Request<Body>) -> Response {
        if !req.headers().contains_key(header::AUTHORIZATION) {
            let token = self.access_token(self.caller_id, self.caller_sid, &DEFAULT_ROLES);
            req.headers_mut().insert(
                header::AUTHORIZATION,
                format!("Bearer {token}").parse().unwrap(),
            );
        }

        self.router.clone().oneshot(req).await.unwrap()
    }

    pub async fn send_raw(&self, req: Request<Body>) -> Response {
        self.router.clone().oneshot(req).await.unwrap()
    }

    pub fn access_token(&self, sub: Uuid, sid: Uuid, roles: &[Role]) -> String {
        self.jwt
            .issue_access(sub, sid, roles, time::OffsetDateTime::now_utc())
            .expect("failed to issue a test access token")
            .value
    }

    pub fn bearer(&self, sub: Uuid, sid: Uuid, roles: &[Role]) -> String {
        format!("Bearer {}", self.access_token(sub, sid, roles))
    }
}
