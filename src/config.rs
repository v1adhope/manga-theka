use std::path::PathBuf;

use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use sqlx::postgres::PgConnectOptions;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub addr: String,
    pub database: Database,
    pub object_storage: ObjectStorage,
    pub redis: Redis,
    pub jwt_access: Jwt,
    pub jwt_refresh: Jwt,
    pub pepper: Pepper,
    pub password: Password,
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
    pub release_pages_bucket: String,
}

#[derive(Deserialize, Debug)]
pub struct Redis {
    pub url: SecretString,
}

#[derive(Deserialize, Debug)]
pub struct Jwt {
    pub private_key: PathBuf,
    pub public_key: PathBuf,
    pub ttl: i64,
}

/// Pepper for the keyed BLAKE3 hash that maps a session refresh jti to its
/// stored digest.
#[derive(Deserialize, Debug)]
pub struct Pepper {
    pub key: SecretString,
}

#[derive(Deserialize, Debug)]
pub struct Password {
    pub m_cost: u32,
    pub t_cost: u32,
    pub p_cost: u32,
}
