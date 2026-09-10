use std::fmt;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    config,
    entity::{Role, Token},
    error::JwtError,
};

use super::{ACCESS_TYP, Keys, REALM, REFRESH_TYP, sign, verify};

#[derive(Debug, Serialize, Deserialize)]
pub struct AccessClaims {
    pub iss: String,
    pub sub: Uuid,
    pub aud: String,
    pub sid: Uuid,
    pub roles: Vec<Role>,
    pub iat: i64,
    pub exp: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshClaims {
    pub iss: String,
    pub sub: Uuid,
    pub aud: String,
    pub sid: Uuid,
    pub jti: Uuid,
    pub iat: i64,
    pub exp: i64,
}

#[derive(Clone)]
pub struct Jwt {
    access: Keys,
    refresh: Keys,
}

impl fmt::Debug for Jwt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Jwt").finish_non_exhaustive()
    }
}

impl Jwt {
    pub fn load(access: &config::Jwt, refresh: &config::Jwt) -> Self {
        Self {
            access: Keys::load(access),
            refresh: Keys::load(refresh),
        }
    }

    pub fn issue_access(
        &self,
        sub: Uuid,
        sid: Uuid,
        roles: &[Role],
        now: OffsetDateTime,
    ) -> Result<Token, JwtError> {
        let iat = now.unix_timestamp();
        let claims = AccessClaims {
            iss: REALM.to_owned(),
            sub,
            aud: REALM.to_owned(),
            sid,
            roles: roles.to_vec(),
            iat,
            exp: iat + self.access.ttl,
        };

        let value = sign(&self.access, ACCESS_TYP, &claims)?;

        Ok(Token {
            value,
            ttl: self.access.ttl,
        })
    }

    pub fn issue_refresh(
        &self,
        sub: Uuid,
        sid: Uuid,
        jti: Uuid,
        now: OffsetDateTime,
    ) -> Result<Token, JwtError> {
        let iat = now.unix_timestamp();
        let claims = RefreshClaims {
            iss: REALM.to_owned(),
            sub,
            aud: REALM.to_owned(),
            sid,
            jti,
            iat,
            exp: iat + self.refresh.ttl,
        };

        let value = sign(&self.refresh, REFRESH_TYP, &claims)?;

        Ok(Token {
            value,
            ttl: self.refresh.ttl,
        })
    }

    pub fn verify_access(&self, token: &str) -> Result<AccessClaims, JwtError> {
        verify(&self.access, ACCESS_TYP, token)
    }

    pub fn verify_refresh(&self, token: &str) -> Result<RefreshClaims, JwtError> {
        verify(&self.refresh, REFRESH_TYP, token)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};

    use time::{Duration, OffsetDateTime};
    use uuid::Uuid;

    use crate::{config, entity::Role};

    use super::Jwt;

    const REALM: &str = "manga-theka";
    const ACCESS_TTL: i64 = 3600;
    const REFRESH_TTL: i64 = 7200;

    fn key_pair(ttl: i64) -> config::Jwt {
        use ed25519_dalek::SigningKey;
        use ed25519_dalek::pkcs8::{EncodePrivateKey, EncodePublicKey, spki::der::pem::LineEnding};

        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);

        let dir =
            std::env::temp_dir().join(format!("manga-theka-jwt-tokens-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let signing = SigningKey::from_bytes(&[(n as u8).wrapping_add(1); 32]);
        let private_key = dir.join("private.pem");
        let public_key = dir.join("public.pem");
        std::fs::write(
            &private_key,
            signing.to_pkcs8_pem(LineEnding::LF).unwrap().as_bytes(),
        )
        .unwrap();
        std::fs::write(
            &public_key,
            signing
                .verifying_key()
                .to_public_key_pem(LineEnding::LF)
                .unwrap(),
        )
        .unwrap();

        config::Jwt {
            private_key,
            public_key,
            ttl,
        }
    }

    fn jwt() -> Jwt {
        Jwt::load(&key_pair(ACCESS_TTL), &key_pair(REFRESH_TTL))
    }

    fn jwt_shared() -> Jwt {
        let cfg = key_pair(ACCESS_TTL);
        Jwt::load(&cfg, &cfg)
    }

    #[test]
    fn an_issued_access_token_round_trips_its_claims() {
        let jwt = jwt();
        let sub = Uuid::now_v7();
        let sid = Uuid::now_v7();
        let now = OffsetDateTime::now_utc();

        let token = jwt
            .issue_access(sub, sid, &[Role::Reader, Role::Admin], now)
            .unwrap();
        assert_eq!(token.ttl, ACCESS_TTL);
        let claims = jwt.verify_access(&token.value).unwrap();

        assert_eq!(claims.sub, sub);
        assert_eq!(claims.sid, sid);
        assert_eq!(claims.roles, vec![Role::Reader, Role::Admin]);
        assert_eq!(claims.iss, REALM);
        assert_eq!(claims.aud, REALM);
        assert_eq!(claims.iat, now.unix_timestamp());
        assert_eq!(claims.exp, now.unix_timestamp() + ACCESS_TTL);
    }

    #[test]
    fn an_issued_refresh_token_round_trips_its_claims() {
        let jwt = jwt();
        let sub = Uuid::now_v7();
        let sid = Uuid::now_v7();
        let jti = Uuid::now_v7();
        let now = OffsetDateTime::now_utc();

        let token = jwt.issue_refresh(sub, sid, jti, now).unwrap();
        assert_eq!(token.ttl, REFRESH_TTL);
        let claims = jwt.verify_refresh(&token.value).unwrap();

        assert_eq!(claims.sub, sub);
        assert_eq!(claims.sid, sid);
        assert_eq!(claims.jti, jti);
        assert_eq!(claims.iss, REALM);
        assert_eq!(claims.aud, REALM);
        assert_eq!(claims.iat, now.unix_timestamp());
        assert_eq!(claims.exp, now.unix_timestamp() + REFRESH_TTL);
    }

    #[test]
    fn an_access_token_with_no_roles_round_trips() {
        let jwt = jwt();
        let token = jwt
            .issue_access(
                Uuid::now_v7(),
                Uuid::now_v7(),
                &[],
                OffsetDateTime::now_utc(),
            )
            .unwrap();

        assert!(jwt.verify_access(&token.value).unwrap().roles.is_empty());
    }

    #[test]
    fn an_access_token_is_not_accepted_as_a_refresh_token() {
        let jwt = jwt_shared();
        let token = jwt
            .issue_access(
                Uuid::now_v7(),
                Uuid::now_v7(),
                &[Role::Reader],
                OffsetDateTime::now_utc(),
            )
            .unwrap();

        assert!(jwt.verify_refresh(&token.value).is_err());
    }

    #[test]
    fn a_refresh_token_is_not_accepted_as_an_access_token() {
        let jwt = jwt_shared();
        let token = jwt
            .issue_refresh(
                Uuid::now_v7(),
                Uuid::now_v7(),
                Uuid::now_v7(),
                OffsetDateTime::now_utc(),
            )
            .unwrap();

        assert!(jwt.verify_access(&token.value).is_err());
    }

    #[test]
    fn an_expired_access_token_is_rejected() {
        let jwt = jwt();
        let long_ago = OffsetDateTime::now_utc() - Duration::hours(2);
        let token = jwt
            .issue_access(Uuid::now_v7(), Uuid::now_v7(), &[Role::Reader], long_ago)
            .unwrap();

        assert!(jwt.verify_access(&token.value).is_err());
    }

    #[test]
    fn a_token_signed_by_a_foreign_key_is_rejected() {
        let issuer = jwt();
        let verifier = jwt();
        let token = issuer
            .issue_access(
                Uuid::now_v7(),
                Uuid::now_v7(),
                &[Role::Reader],
                OffsetDateTime::now_utc(),
            )
            .unwrap();

        assert!(verifier.verify_access(&token.value).is_err());
    }

    #[test]
    fn a_tampered_payload_is_rejected() {
        let jwt = jwt();
        let token = jwt
            .issue_access(
                Uuid::now_v7(),
                Uuid::now_v7(),
                &[Role::Reader],
                OffsetDateTime::now_utc(),
            )
            .unwrap();

        let (header, rest) = token.value.split_once('.').unwrap();
        let (payload, sig) = rest.split_once('.').unwrap();
        let mut bytes = payload.as_bytes().to_vec();
        let last = bytes.last_mut().unwrap();
        *last = if *last == b'A' { b'B' } else { b'A' };
        let tampered = format!("{header}.{}.{sig}", String::from_utf8(bytes).unwrap());

        assert!(jwt.verify_access(&tampered).is_err());
    }

    #[test]
    fn a_non_jwt_string_is_rejected() {
        assert!(jwt().verify_access("definitely-not-a-jwt").is_err());
    }
}
