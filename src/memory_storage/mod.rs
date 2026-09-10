mod session;

use std::fmt;

use secrecy::ExposeSecret;
use uuid::Uuid;

use crate::config;

/// Per-session blob key. One key each so every session gets its own native
/// Redis TTL; a single per-user hash can't, since expiry is per-key not
/// per-field.
fn blob_key(sub: Uuid, sid: Uuid) -> String {
    format!("refresh-tokens:{sub}:{sid}")
}

/// Per-user index: a SET of every `sid`. Lets callers enumerate a user's
/// sessions without a keyspace `SCAN` over the scattered [`blob_key`]s. A
/// superset filtered on read; a blob's TTL can lapse before its entry is pruned.
fn sids_key(sub: Uuid) -> String {
    format!("refresh-tokens:sids:{sub}")
}

pub async fn connection(cfg: &config::Redis) -> redis::aio::ConnectionManager {
    let client = redis::Client::open(cfg.url.expose_secret())
        .expect("failed to parse the Redis connection url");

    client
        .get_connection_manager()
        .await
        .expect("failed to connect to Redis")
}

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
