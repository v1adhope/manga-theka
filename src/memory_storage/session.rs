use std::net::IpAddr;

use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use uuid::Uuid;

use crate::{
    entity::{HexHash, Session, Text, Timestamp},
    error::{EntityError, LogInternal, MemoryStoreError},
};

use super::{MemoryStore, blob_key, sids_key};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionBlob {
    jti: String,
    ua: Option<Text>,
    ip: Option<IpAddr>,
    created_at: Timestamp,
    updated_at: Timestamp,
}

impl From<Session> for SessionBlob {
    fn from(session: Session) -> Self {
        Self {
            jti: session.jti.into_inner(),
            ua: session.ua,
            ip: session.ip,
            created_at: session.created_at,
            updated_at: session.updated_at,
        }
    }
}

impl TryFrom<(SessionBlob, Uuid)> for Session {
    type Error = EntityError;

    fn try_from((blob, sid): (SessionBlob, Uuid)) -> Result<Self, Self::Error> {
        Ok(Self {
            sid,
            jti: HexHash::try_from(blob.jti)?,
            ua: blob.ua,
            ip: blob.ip,
            created_at: blob.created_at,
            updated_at: blob.updated_at,
        })
    }
}

impl MemoryStore {
    #[instrument(name = "memory.session.put", skip_all, fields(user.id = %sub, session.id = %session.sid))]
    pub async fn put_session(&self, sub: Uuid, session: Session) -> Result<(), MemoryStoreError> {
        let sid = session.sid;
        let json =
            serde_json::to_string(&SessionBlob::from(session)).map_err(MemoryStoreError::Serde)?;
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

        let blob = json
            .map(|j| serde_json::from_str::<SessionBlob>(&j))
            .transpose()
            .map_err(MemoryStoreError::Serde)
            .inspect_err(MemoryStoreError::log_internal)?;

        blob.map(|b| Session::try_from((b, sid)))
            .transpose()
            .map_err(MemoryStoreError::CorruptJti)
            .inspect_err(MemoryStoreError::log_internal)
    }

    #[instrument(name = "memory.session.list", skip_all, fields(user.id = %sub))]
    pub async fn list_sessions(&self, sub: Uuid) -> Result<Vec<Session>, MemoryStoreError> {
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
                    let blob: SessionBlob =
                        serde_json::from_str(&json).map_err(MemoryStoreError::Serde)?;
                    let session = Session::try_from((blob, parsed))
                        .map_err(MemoryStoreError::CorruptJti)
                        .inspect_err(MemoryStoreError::log_internal)?;
                    live.push(session);
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

#[cfg(test)]
mod tests {
    use time::OffsetDateTime;
    use uuid::Uuid;

    use super::SessionBlob;
    use crate::entity::{HexHash, Session};

    #[test]
    fn the_blob_round_trips_and_never_stores_the_sid() {
        let now = OffsetDateTime::now_utc();
        let sid = Uuid::now_v7();
        let session = Session {
            sid,
            jti: HexHash::try_from("a".repeat(64)).unwrap(),
            ua: None,
            ip: None,
            created_at: now.into(),
            updated_at: now.into(),
        };

        let json = serde_json::to_string(&SessionBlob::from(session)).unwrap();
        assert!(json.contains("jti"), "the redis blob carries jti");
        assert!(
            !json.contains(&sid.to_string()),
            "the sid is the key, not a field"
        );

        let blob: SessionBlob = serde_json::from_str(&json).unwrap();
        assert_eq!(Session::try_from((blob, sid)).unwrap().sid, sid);
    }
}
