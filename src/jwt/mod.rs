mod tokens;

#[cfg(test)]
mod fixtures;

pub use tokens::{AccessClaims, Jwt, RefreshClaims};

use std::fs;

use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Serialize, de::DeserializeOwned};

use crate::{
    config,
    error::{JwtError, LogInternal},
};

const ACCESS_TYP: &str = "at+jwt";
const REFRESH_TYP: &str = "rt+jwt";

const REALM: &str = "manga-theka";

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
        validation.set_required_spec_claims(&["exp", "iss", "aud"]);
        validation.set_issuer(&[REALM]);
        validation.set_audience(&[REALM]);

        Self {
            encoding,
            decoding,
            validation,
            ttl: cfg.ttl,
        }
    }
}

fn sign<T: Serialize>(keys: &Keys, typ: &str, claims: &T) -> Result<String, JwtError> {
    let mut header = Header::new(Algorithm::EdDSA);
    header.typ = Some(typ.to_owned());

    encode(&header, claims, &keys.encoding)
        .map_err(JwtError::Sign)
        .inspect_err(JwtError::log_internal)
}

fn verify<T: DeserializeOwned>(keys: &Keys, typ: &str, token: &str) -> Result<T, JwtError> {
    let data = decode::<T>(token, &keys.decoding, &keys.validation).map_err(JwtError::Verify)?;

    if data.header.typ.as_deref() != Some(typ) {
        return Err(JwtError::WrongTokenClass);
    }

    Ok(data.claims)
}
