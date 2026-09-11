use std::sync::atomic::{AtomicU32, Ordering};

use ed25519_dalek::{
    SigningKey,
    pkcs8::{EncodePrivateKey, EncodePublicKey, spki::der::pem::LineEnding},
};

use crate::{config, jwt::Jwt};

pub(crate) const REALM: &str = "manga-theka";
pub(crate) const ACCESS_TTL: i64 = 3600;
pub(crate) const REFRESH_TTL: i64 = 7200;

fn key_pair(ttl: i64) -> config::Jwt {
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

pub(crate) fn jwt() -> Jwt {
    Jwt::load(&key_pair(ACCESS_TTL), &key_pair(REFRESH_TTL))
}

pub(crate) fn jwt_shared() -> Jwt {
    let cfg = key_pair(ACCESS_TTL);
    Jwt::load(&cfg, &cfg)
}
