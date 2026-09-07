use redis::AsyncCommands;
use tracing::instrument;
use uuid::Uuid;

use crate::{entity::Session, error::MemoryStoreError};

use super::{MemoryStore, blob_key, sids_key};

impl MemoryStore {
    #[instrument(name = "memory.session.put", skip_all, fields(user.id = %sub, session.id = %sid))]
    pub async fn put_session(
        &self,
        sub: Uuid,
        sid: Uuid,
        session: &Session,
    ) -> Result<(), MemoryStoreError> {
        let json = serde_json::to_string(session).map_err(MemoryStoreError::Serde)?;
        let mut conn = self.conn.clone();
        let ttl = self.refresh_ttl;

        redis::pipe()
            .atomic()
            .set_ex(blob_key(sub, &sid.to_string()), json, ttl)
            .ignore()
            .sadd(sids_key(sub), sid.to_string())
            .ignore()
            .expire(sids_key(sub), ttl as i64)
            .ignore()
            .query_async::<()>(&mut conn)
            .await
            .map_err(MemoryStoreError::from)
            .inspect_err(MemoryStoreError::log_internal)?;

        Ok(())
    }

    #[instrument(name = "memory.session.get", skip_all, fields(user.id = %sub, session.id = %sid))]
    pub async fn get_session(
        &self,
        sub: Uuid,
        sid: Uuid,
    ) -> Result<Option<Session>, MemoryStoreError> {
        let mut conn = self.conn.clone();

        let json: Option<String> = conn
            .get(blob_key(sub, &sid.to_string()))
            .await
            .map_err(MemoryStoreError::from)
            .inspect_err(MemoryStoreError::log_internal)?;

        json.map(|j| serde_json::from_str(&j))
            .transpose()
            .map_err(MemoryStoreError::Serde)
            .inspect_err(MemoryStoreError::log_internal)
    }

    #[instrument(name = "memory.session.list", skip_all, fields(user.id = %sub))]
    pub async fn list_sessions(&self, sub: Uuid) -> Result<Vec<(Uuid, Session)>, MemoryStoreError> {
        let mut conn = self.conn.clone();

        let sids: Vec<String> = conn
            .smembers(sids_key(sub))
            .await
            .map_err(MemoryStoreError::from)
            .inspect_err(MemoryStoreError::log_internal)?;

        if sids.is_empty() {
            return Ok(Vec::new());
        }

        let keys: Vec<String> = sids.iter().map(|s| blob_key(sub, s)).collect();
        let blobs: Vec<Option<String>> = conn
            .mget(&keys)
            .await
            .map_err(MemoryStoreError::from)
            .inspect_err(MemoryStoreError::log_internal)?;

        let mut live = Vec::new();
        let mut dead = Vec::new();
        for (sid, blob) in sids.into_iter().zip(blobs) {
            match blob {
                Some(json) => {
                    let parsed = Uuid::parse_str(&sid).map_err(MemoryStoreError::CorruptSid)?;
                    let session: Session =
                        serde_json::from_str(&json).map_err(MemoryStoreError::Serde)?;
                    live.push((parsed, session));
                }
                None => dead.push(sid),
            }
        }

        if !dead.is_empty() {
            // Native TTL can clear a blob before the index catches up, so the
            // SET is a superset filtered on read.
            let _: () = conn
                .srem(sids_key(sub), dead)
                .await
                .map_err(MemoryStoreError::from)
                .inspect_err(MemoryStoreError::log_internal)?;
        }

        Ok(live)
    }

    #[instrument(name = "memory.session.owns", skip_all, fields(user.id = %sub, session.id = %sid))]
    pub async fn owns_session(&self, sub: Uuid, sid: Uuid) -> Result<bool, MemoryStoreError> {
        let mut conn = self.conn.clone();

        conn.sismember(sids_key(sub), sid.to_string())
            .await
            .map_err(MemoryStoreError::from)
            .inspect_err(MemoryStoreError::log_internal)
    }

    #[instrument(name = "memory.session.revoke", skip_all, fields(user.id = %sub, session.id = %sid))]
    pub async fn revoke_session(&self, sub: Uuid, sid: Uuid) -> Result<(), MemoryStoreError> {
        let mut conn = self.conn.clone();

        redis::pipe()
            .atomic()
            .del(blob_key(sub, &sid.to_string()))
            .ignore()
            .srem(sids_key(sub), sid.to_string())
            .ignore()
            .query_async::<()>(&mut conn)
            .await
            .map_err(MemoryStoreError::from)
            .inspect_err(MemoryStoreError::log_internal)?;

        Ok(())
    }

    #[instrument(name = "memory.session.revoke_all", skip_all, fields(user.id = %sub))]
    pub async fn revoke_all_sessions(&self, sub: Uuid) -> Result<(), MemoryStoreError> {
        let mut conn = self.conn.clone();

        let sids: Vec<String> = conn
            .smembers(sids_key(sub))
            .await
            .map_err(MemoryStoreError::from)
            .inspect_err(MemoryStoreError::log_internal)?;

        let mut pipe = redis::pipe();
        pipe.atomic();
        for sid in &sids {
            pipe.del(blob_key(sub, sid)).ignore();
        }
        pipe.del(sids_key(sub)).ignore();

        pipe.query_async::<()>(&mut conn)
            .await
            .map_err(MemoryStoreError::from)
            .inspect_err(MemoryStoreError::log_internal)?;

        Ok(())
    }
}
