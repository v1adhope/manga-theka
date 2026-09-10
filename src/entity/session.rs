use std::net::IpAddr;

use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::{
    entity::{Email, Entity, HexHash, Password, ShortText, Timestamp},
    error::EntityError,
};

#[derive(Debug)]
pub struct LoginForm {
    pub email: Email,
    pub password: Password,
    pub ua: Option<ShortText>,
    pub ip: Option<IpAddr>,
    pub now: OffsetDateTime,
}

pub struct Token {
    pub value: String,
    pub ttl: i64,
}

pub struct SessionTokens {
    pub access: Token,
    pub refresh: Token,
}

#[derive(Debug)]
pub struct Session {
    pub sid: Uuid,
    pub jti: HexHash,
    pub ua: Option<ShortText>,
    pub ip: Option<IpAddr>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl Entity for Session {
    const NAME: &'static str = "Session";
}

impl Session {
    pub fn ensure_revoker(&self, now: OffsetDateTime) -> Result<(), EntityError> {
        const MIN_REVOKER_AGE: Duration = Duration::hours(24);

        if now - self.created_at.into_inner() < MIN_REVOKER_AGE {
            return Err(EntityError::SessionTooNewToRevoke);
        }

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionQuery {
    pub sid: Uuid,
    pub ua: Option<ShortText>,
    pub ip: Option<IpAddr>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl From<Session> for SessionQuery {
    fn from(session: Session) -> Self {
        Self {
            sid: session.sid,
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

    use crate::entity::{HexHash, Session, SessionQuery};

    fn session(age: Duration) -> Session {
        let now = OffsetDateTime::now_utc();

        Session {
            sid: Uuid::now_v7(),
            jti: HexHash::try_from("a".repeat(64)).unwrap(),
            ua: None,
            ip: None,
            created_at: (now - age).into(),
            updated_at: (now - age).into(),
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
    fn the_read_shape_exposes_the_sid_and_scrubs_the_jti() {
        let s = session(Duration::ZERO);
        let sid = s.sid;

        let read = SessionQuery::from(s);
        assert_eq!(read.sid, sid);

        let json = serde_json::to_string(&read).unwrap();
        assert!(!json.contains("jti"), "the read shape scrubs jti");
        assert!(
            json.contains(&sid.to_string()),
            "the read shape carries sid"
        );
    }
}
