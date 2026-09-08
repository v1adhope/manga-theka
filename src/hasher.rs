use argon2::{
    Algorithm, Argon2, Params, Version,
    password_hash::{PasswordHasher, PasswordVerifier},
};

use crate::{
    entity::{BookSelection, HexHash, Password, PasswordHash, ShortHexHash},
    error::HasherError,
};

const JTI_PEPPER_CONTEXT: &str = "manga-theka jti pepper v1";

#[derive(Debug, Clone)]
pub struct Hasher {
    params: Params,
    pepper: [u8; 32],
}

impl Hasher {
    pub fn new(m_cost: u32, t_cost: u32, p_cost: u32, secret: &[u8]) -> Result<Self, HasherError> {
        let params = Params::new(m_cost, t_cost, p_cost, None)
            .map_err(HasherError::Params)
            .inspect_err(HasherError::log_internal)?;
        let pepper = blake3::derive_key(JTI_PEPPER_CONTEXT, secret);

        Ok(Self { params, pepper })
    }

    fn argon2(&self) -> Argon2<'_> {
        Argon2::new(Algorithm::Argon2id, Version::V0x13, self.params.clone())
    }

    pub fn hash_password(&self, password: Password) -> Result<PasswordHash, HasherError> {
        let phc = self
            .argon2()
            .hash_password(password.expose_secret().as_bytes())
            .map_err(HasherError::HashPassword)
            .inspect_err(HasherError::log_internal)?
            .to_string();

        PasswordHash::try_from(phc)
            .map_err(HasherError::Phc)
            .inspect_err(HasherError::log_internal)
    }

    pub fn verify_password(&self, password: Password, hash: &str) -> Result<(), HasherError> {
        match self
            .argon2()
            .verify_password(password.expose_secret().as_bytes(), hash)
        {
            Ok(()) => Ok(()),
            Err(e @ argon2::password_hash::Error::PasswordInvalid) => {
                Err(HasherError::PasswordMismatch(e.into()))
            }
            Err(e) => {
                let err = HasherError::VerifyPassword(e);
                err.log_internal();
                Err(err)
            }
        }
    }

    pub fn keyed_jti_hash(&self, jti: uuid::Uuid) -> Result<HexHash, HasherError> {
        let hex = blake3::keyed_hash(&self.pepper, jti.as_bytes()).to_hex();

        HexHash::try_from(hex.to_string())
            .map_err(HasherError::Digest)
            .inspect_err(HasherError::log_internal)
    }

    pub fn compute_hex_hash(target: &BookSelection) -> Result<ShortHexHash, HasherError> {
        const LEN: usize = 16;

        let json = serde_json::to_string(target)
            .map_err(HasherError::Serialize)
            .inspect_err(HasherError::log_internal)?;
        let hex = blake3::hash(json.as_bytes()).to_hex();

        ShortHexHash::try_from(hex[..LEN].to_owned())
            .map_err(HasherError::Digest)
            .inspect_err(HasherError::log_internal)
    }
}

#[cfg(test)]
mod tests {
    use time::{Duration, OffsetDateTime};

    use crate::{
        entity::{
            BookSelection, BookVisibility, CreatedAtRange, Password, Timestamp,
            book_filter::tests::unfiltered_selection,
        },
        error::HasherError,
        hasher::Hasher,
    };

    fn hasher() -> Hasher {
        Hasher::new(19456, 2, 1, b"test-pepper").unwrap()
    }

    #[test]
    fn a_hash_is_stable_across_runs() {
        assert_eq!(
            Hasher::compute_hex_hash(&unfiltered_selection()).unwrap(),
            Hasher::compute_hex_hash(&unfiltered_selection()).unwrap(),
        );
    }

    #[test]
    fn a_moved_facet_hashes_differently() {
        let moved = BookSelection {
            visibility: BookVisibility::Hidden,
            ..unfiltered_selection()
        };

        assert_ne!(
            Hasher::compute_hex_hash(&unfiltered_selection()).unwrap(),
            Hasher::compute_hex_hash(&moved).unwrap(),
        );
    }

    #[test]
    fn two_spellings_of_one_instant_are_the_same_filter() {
        let utc = OffsetDateTime::UNIX_EPOCH + Duration::hours(12);
        let shifted = utc.to_offset(time::UtcOffset::from_hms(2, 0, 0).unwrap());

        let one = BookSelection {
            created_at: CreatedAtRange::try_new(Some(Timestamp::from(utc)), None).unwrap(),
            ..unfiltered_selection()
        };
        let other = BookSelection {
            created_at: CreatedAtRange::try_new(Some(Timestamp::from(shifted)), None).unwrap(),
            ..unfiltered_selection()
        };

        assert_eq!(
            Hasher::compute_hex_hash(&one).unwrap(),
            Hasher::compute_hex_hash(&other).unwrap(),
        );
    }

    #[test]
    fn a_password_verifies_against_its_own_hash() {
        let hasher = hasher();
        let hash = hasher
            .hash_password(Password::try_from("correct horse battery".to_owned()).unwrap())
            .unwrap();

        hasher
            .verify_password(
                Password::try_from("correct horse battery".to_owned()).unwrap(),
                hash.as_ref(),
            )
            .unwrap();

        assert!(matches!(
            hasher.verify_password(
                Password::try_from("wrong horse battery".to_owned()).unwrap(),
                hash.as_ref(),
            ),
            Err(HasherError::PasswordMismatch(_))
        ));
    }

    #[test]
    fn a_keyed_jti_hash_is_stable_and_key_dependent() {
        let jti = uuid::Uuid::now_v7();
        let a = Hasher::new(19456, 2, 1, b"one").unwrap();
        let b = Hasher::new(19456, 2, 1, b"two").unwrap();

        assert_eq!(
            a.keyed_jti_hash(jti).unwrap().as_ref(),
            a.keyed_jti_hash(jti).unwrap().as_ref()
        );
        assert_ne!(
            a.keyed_jti_hash(jti).unwrap().as_ref(),
            b.keyed_jti_hash(jti).unwrap().as_ref()
        );
    }
}
