use std::fmt;
use std::fs;

use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::Deserialize;
use serde::{Serialize, de::DeserializeOwned};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    config,
    entity::Role,
    error::{JwtError, LogInternal},
};

const ACCESS_TYP: &str = "at+jwt";
const REFRESH_TYP: &str = "rt+jwt";

#[derive(Debug, Serialize, Deserialize)]
pub struct AccessClaims {
    pub sub: Uuid,
    pub sid: Uuid,
    pub roles: Vec<Role>,
    pub iat: i64,
    pub exp: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshClaims {
    pub sub: Uuid,
    pub sid: Uuid,
    pub jti: Uuid,
    pub iat: i64,
    pub exp: i64,
}

#[derive(Clone)]
struct Keys {
    encoding: EncodingKey,
    decoding: DecodingKey,
    validation: Validation,
    ttl: i64,
}

impl Keys {
    fn load(cfg: &config::Jwt) -> Self {
        let private = fs::read(&cfg.private_key).expect("failed to read the JWT private key");
        let public = fs::read(&cfg.public_key).expect("failed to read the JWT public key");

        let encoding =
            EncodingKey::from_ed_pem(&private).expect("JWT private key is not a valid Ed25519 PEM");
        let decoding =
            DecodingKey::from_ed_pem(&public).expect("JWT public key is not a valid Ed25519 PEM");

        let mut validation = Validation::new(Algorithm::EdDSA);
        validation.leeway = 0;
        validation.validate_exp = true;

        Self {
            encoding,
            decoding,
            validation,
            ttl: cfg.ttl,
        }
    }
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
            sub,
            sid,
            roles: roles.to_vec(),
            iat,
            exp: iat + self.access.ttl,
        };

        Self::sign(&self.access, ACCESS_TYP, &claims)
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
            sub,
            sid,
            jti,
            iat,
            exp: iat + self.refresh.ttl,
        };

        Self::sign(&self.refresh, REFRESH_TYP, &claims)
    }

    pub fn verify_access(&self, token: &str) -> Result<AccessClaims, JwtError> {
        Self::verify(&self.access, ACCESS_TYP, token)
    }

    pub fn verify_refresh(&self, token: &str) -> Result<RefreshClaims, JwtError> {
        Self::verify(&self.refresh, REFRESH_TYP, token)
    }

    fn sign<T: Serialize>(keys: &Keys, typ: &str, claims: &T) -> Result<String, JwtError> {
        let mut header = Header::new(Algorithm::EdDSA);
        header.typ = Some(typ.to_owned());

        encode(&header, claims, &keys.encoding)
            .map_err(JwtError::Sign)
            .inspect_err(JwtError::log_internal)
    }

    fn verify<T: DeserializeOwned>(keys: &Keys, typ: &str, token: &str) -> Result<T, JwtError> {
        let data =
            decode::<T>(token, &keys.decoding, &keys.validation).map_err(JwtError::Verify)?;

        // `jsonwebtoken::Validation` has no `typ` check, so cross-class rejection
        // is enforced here.
        if data.header.typ.as_deref() != Some(typ) {
            return Err(JwtError::WrongTokenClass);
        }

        Ok(data.claims)
    }
}
