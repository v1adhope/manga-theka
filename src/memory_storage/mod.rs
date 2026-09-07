mod session;

use std::fmt;

use secrecy::ExposeSecret;
use uuid::Uuid;

use crate::config;

fn blob_key(sub: Uuid, sid: &str) -> String {
    format!("refresh-tokens:{sub}:{sid}")
}

fn sids_key(sub: Uuid) -> String {
    format!("sids:{sub}")
}

pub async fn connection(cfg: &config::Redis) -> redis::aio::ConnectionManager {
    let client = redis::Client::open(cfg.url.expose_secret())
        .expect("failed to parse the Redis connection url");

    client
        .get_connection_manager()
        .await
        .expect("failed to connect to Redis")
}

/// The Redis sibling of `object_storage::ObjectStorage`.
#[derive(Clone)]
pub struct MemoryStore {
    conn: redis::aio::ConnectionManager,
    refresh_ttl: u64,
}

impl fmt::Debug for MemoryStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MemoryStore")
            .field("refresh_ttl", &self.refresh_ttl)
            .finish_non_exhaustive()
    }
}

impl MemoryStore {
    pub fn new(conn: redis::aio::ConnectionManager, refresh_ttl: i64) -> Self {
        Self {
            conn,
            refresh_ttl: refresh_ttl.max(0) as u64,
        }
    }
}
