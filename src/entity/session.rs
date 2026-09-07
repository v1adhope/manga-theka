use std::net::IpAddr;

use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::{
    entity::{Entity, Text},
    error::EntityError,
};

const MIN_REVOKER_AGE: Duration = Duration::hours(24);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JtiHash(String);

impl JtiHash {
    pub(crate) fn from_hex(hex: String) -> Self {
        Self(hex)
    }
}

impl AsRef<str> for JtiHash {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// The per-session blob stored in Redis; the `sid` is the key, not a field.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub jti: JtiHash,
    pub ua: Option<Text>,
    pub ip: Option<IpAddr>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

impl Entity for Session {
    const NAME: &'static str = "Session";
}

impl Session {
    /// A session younger than 24h may not revoke other sessions, so a freshly
    /// compromised credential cannot lock the user out of their live sessions.
    pub fn ensure_revoker(&self, now: OffsetDateTime) -> Result<(), EntityError> {
        if now - self.created_at < MIN_REVOKER_AGE {
            return Err(EntityError::SessionTooNewToRevoke);
        }

        Ok(())
    }
}

/// The `GET /sessions/me` read shape -- `jti` absent by construction.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionQuery {
    pub sid: Uuid,
    pub ua: Option<Text>,
    pub ip: Option<IpAddr>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

impl SessionQuery {
    pub fn from_session(sid: Uuid, session: Session) -> Self {
        Self {
            sid,
            ua: session.ua,
            ip: session.ip,
            created_at: session.created_at,
            updated_at: session.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use time::{Duration, OffsetDateTime};
    use uuid::Uuid;

    use crate::{
        entity::{JtiHash, Session},
        hasher::Hasher,
    };

    fn session(age: Duration) -> Session {
        let now = OffsetDateTime::now_utc();

        Session {
            jti: JtiHash::from_hex("deadbeef".to_owned()),
            ua: None,
            ip: None,
            created_at: now - age,
            updated_at: now - age,
        }
    }

    #[test]
    fn a_session_older_than_24h_may_revoke() {
        let s = session(Duration::hours(24) + Duration::seconds(1));

        assert!(s.ensure_revoker(OffsetDateTime::now_utc()).is_ok());
    }

    #[test]
    fn a_session_younger_than_24h_may_not_revoke() {
        let s = session(Duration::hours(23));

        assert!(s.ensure_revoker(OffsetDateTime::now_utc()).is_err());
    }

    #[test]
    fn the_stored_blob_round_trips_without_leaking_jti_into_the_read_shape() {
        let hasher = Hasher::new(19456, 2, 1, b"pepper").unwrap();
        let s = Session {
            jti: hasher.keyed_jti_hash(Uuid::now_v7()),
            ..session(Duration::ZERO)
        };

        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("\"jti\""), "the redis blob carries jti");

        let read = crate::entity::SessionQuery::from_session(Uuid::now_v7(), s);
        let read_json = serde_json::to_string(&read).unwrap();
        assert!(!read_json.contains("jti"), "the read shape scrubs jti");
    }
}
