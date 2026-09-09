use std::fmt;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{config, entity::Role, error::JwtError};

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

    pub fn refresh_ttl(&self) -> i64 {
        self.refresh.ttl
    }

    pub fn issue_access(
        &self,
        sub: Uuid,
        sid: Uuid,
        roles: &[Role],
        now: OffsetDateTime,
    ) -> Result<String, JwtError> {
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

        sign(&self.access, ACCESS_TYP, &claims)
    }

    pub fn issue_refresh(
        &self,
        sub: Uuid,
        sid: Uuid,
        jti: Uuid,
        now: OffsetDateTime,
    ) -> Result<String, JwtError> {
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

        sign(&self.refresh, REFRESH_TYP, &claims)
    }

    pub fn verify_access(&self, token: &str) -> Result<AccessClaims, JwtError> {
        verify(&self.access, ACCESS_TYP, token)
    }

    pub fn verify_refresh(&self, token: &str) -> Result<RefreshClaims, JwtError> {
        verify(&self.refresh, REFRESH_TYP, token)
    }
}
