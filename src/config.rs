use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use sqlx::postgres::PgConnectOptions;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub addr: String,
    pub database: Database,
    pub object_storage: ObjectStorage,
    pub log_level: String,
}

impl Config {
    pub fn from_env() -> Self {
        envious::Config::default()
            .with_prefix("APP_")
            .build_from_env()
            .expect("could not deserialize Config from env")
    }

    pub fn from_env_pairs<I, K, V>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        envious::Config::new()
            .with_prefix("APP_")
            .build_from_iter(pairs)
            .expect("failed to build Config from env pairs")
    }
}

#[derive(Deserialize, Debug)]
pub struct Database {
    pub username: String,
    pub password: SecretString,
    pub host: String,
    pub port: u16,
    pub database_name: String,
}

impl Database {
    pub fn with_db(&self) -> PgConnectOptions {
        self.without_db().database(&self.database_name)
    }

    pub fn without_db(&self) -> PgConnectOptions {
        PgConnectOptions::new()
            .username(&self.username)
            .password(self.password.expose_secret())
            .host(&self.host)
            .port(self.port)
    }
}

#[derive(Deserialize, Debug)]
pub struct ObjectStorage {
    pub endpoint: String,
    pub region: String,
    pub access_key: String,
    pub secret_key: SecretString,
    pub covers_bucket: String,
}
