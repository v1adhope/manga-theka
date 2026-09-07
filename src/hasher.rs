use std::sync::Arc;

use argon2::{
    Algorithm, Argon2, Params, Version,
    password_hash::{PasswordHasher, PasswordVerifier},
};

use crate::{
    entity::{BookSelection, HexHash, JtiHash, PasswordHash},
    error::HasherError,
};

const JTI_PEPPER_CONTEXT: &str = "manga-theka jti pepper v1";
const DUMMY_PASSWORD: &[u8] = b"manga-theka account-enumeration timing equalizer";

#[derive(Debug, Clone)]
pub struct Hasher {
    params: Params,
    pepper: [u8; 32],
    /// Hashed once with `params` so the no-such-user login path costs the same
    /// as a real verify regardless of the configured cost (issue #3, story 17).
    dummy_phc: Arc<str>,
}

impl Hasher {
    pub fn new(m_cost: u32, t_cost: u32, p_cost: u32, secret: &[u8]) -> Result<Self, HasherError> {
        let params = Params::new(m_cost, t_cost, p_cost, None)
            .map_err(HasherError::Params)
            .inspect_err(HasherError::log_internal)?;
        let pepper = blake3::derive_key(JTI_PEPPER_CONTEXT, secret);

        let dummy_phc: Arc<str> = Argon2::new(Algorithm::Argon2id, Version::V0x13, params.clone())
            .hash_password(DUMMY_PASSWORD)
            .map_err(HasherError::HashPassword)
            .inspect_err(HasherError::log_internal)?
            .to_string()
            .into();

        Ok(Self {
            params,
            pepper,
            dummy_phc,
        })
    }

    fn argon2(&self) -> Argon2<'_> {
        Argon2::new(Algorithm::Argon2id, Version::V0x13, self.params.clone())
    }

    pub fn hash_password(&self, password: &str) -> Result<PasswordHash, HasherError> {
        let phc = self
            .argon2()
            .hash_password(password.as_bytes())
            .map_err(HasherError::HashPassword)
            .inspect_err(HasherError::log_internal)?
            .to_string();

        PasswordHash::try_from(phc)
            .map_err(HasherError::Phc)
            .inspect_err(HasherError::log_internal)
    }

    pub fn verify_password(&self, password: &str, hash: &str) -> Result<bool, HasherError> {
        match self.argon2().verify_password(password.as_bytes(), hash) {
            Ok(()) => Ok(true),
            Err(argon2::password_hash::Error::PasswordInvalid) => Ok(false),
            Err(e) => {
                let err = HasherError::VerifyPassword(e);
                err.log_internal();
                Err(err)
            }
        }
    }

    pub fn verify_dummy(&self, password: &str) {
        let _ = self
            .argon2()
            .verify_password(password.as_bytes(), self.dummy_phc.as_ref());
    }

    pub fn keyed_jti_hash(&self, jti: uuid::Uuid) -> JtiHash {
        let hex = blake3::keyed_hash(&self.pepper, jti.as_bytes()).to_hex();

        JtiHash::from_hex(hex.to_string())
    }

    pub fn compute_hex_hash(target: &BookSelection) -> Result<HexHash, HasherError> {
        const LEN: usize = 16;

        let json = serde_json::to_string(target)
            .map_err(HasherError::Serialize)
            .inspect_err(HasherError::log_internal)?;
        let hex = blake3::hash(json.as_bytes()).to_hex();

        HexHash::try_from(hex[..LEN].to_owned())
            .map_err(HasherError::Digest)
            .inspect_err(HasherError::log_internal)
    }
}

#[cfg(test)]
pub mod tests {
    use time::{Duration, OffsetDateTime};

    use crate::{
        entity::{
            BookKinds, BookLabelIds, BookSelection, BookSortField, BookStatuses, BookVisibility,
            CreatedAtRange, FilterLookupIds, LabelFilter, LabelsMode, PublicationDemographics,
            PublicationYearRange, SortOrder, Timestamp,
        },
        hasher::Hasher,
    };

    pub fn stub() -> BookSelection {
        BookSelection {
            visibility: BookVisibility::Listed,
            sort_field: BookSortField::CreatedAt,
            order: SortOrder::Desc,
            labels: LabelFilter {
                included: BookLabelIds::try_from(vec![]).unwrap(),
                mode: LabelsMode::And,
                excluded: BookLabelIds::try_from(vec![]).unwrap(),
            },
            kinds: BookKinds::try_from(vec![]).unwrap(),
            statuses: BookStatuses::try_from(vec![]).unwrap(),
            content_rating_ids: FilterLookupIds::try_from(vec![]).unwrap(),
            publication_language_ids: FilterLookupIds::try_from(vec![]).unwrap(),
            publication_demographics: PublicationDemographics::try_from(vec![]).unwrap(),
            available_translated_language_ids: FilterLookupIds::try_from(vec![]).unwrap(),
            publication_year: PublicationYearRange::try_new(None, None).unwrap(),
            created_at: CreatedAtRange::try_new(None, None).unwrap(),
        }
    }

    fn hasher() -> Hasher {
        Hasher::new(19456, 2, 1, b"test-pepper").unwrap()
    }

    #[test]
    fn a_hash_is_stable_across_runs() {
        assert_eq!(
            Hasher::compute_hex_hash(&stub()).unwrap(),
            Hasher::compute_hex_hash(&stub()).unwrap(),
        );
    }

    #[test]
    fn a_moved_facet_hashes_differently() {
        let moved = BookSelection {
            visibility: BookVisibility::Hidden,
            ..stub()
        };

        assert_ne!(
            Hasher::compute_hex_hash(&stub()).unwrap(),
            Hasher::compute_hex_hash(&moved).unwrap(),
        );
    }

    #[test]
    fn two_spellings_of_one_instant_are_the_same_filter() {
        let utc = OffsetDateTime::UNIX_EPOCH + Duration::hours(12);
        let shifted = utc.to_offset(time::UtcOffset::from_hms(2, 0, 0).unwrap());

        let one = BookSelection {
            created_at: CreatedAtRange::try_new(Some(Timestamp::from(utc)), None).unwrap(),
            ..stub()
        };
        let other = BookSelection {
            created_at: CreatedAtRange::try_new(Some(Timestamp::from(shifted)), None).unwrap(),
            ..stub()
        };

        assert_eq!(
            Hasher::compute_hex_hash(&one).unwrap(),
            Hasher::compute_hex_hash(&other).unwrap(),
        );
    }

    #[test]
    fn a_password_verifies_against_its_own_hash() {
        let hasher = hasher();
        let hash = hasher.hash_password("correct horse battery").unwrap();

        assert!(
            hasher
                .verify_password("correct horse battery", hash.as_ref())
                .unwrap()
        );
        assert!(
            !hasher
                .verify_password("wrong horse battery", hash.as_ref())
                .unwrap()
        );
    }

    #[test]
    fn a_keyed_jti_hash_is_stable_and_key_dependent() {
        let jti = uuid::Uuid::now_v7();
        let a = Hasher::new(19456, 2, 1, b"one").unwrap();
        let b = Hasher::new(19456, 2, 1, b"two").unwrap();

        assert_eq!(
            a.keyed_jti_hash(jti).as_ref(),
            a.keyed_jti_hash(jti).as_ref()
        );
        assert_ne!(
            a.keyed_jti_hash(jti).as_ref(),
            b.keyed_jti_hash(jti).as_ref()
        );
    }
}
