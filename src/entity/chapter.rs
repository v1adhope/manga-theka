use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    entity::{Bounded, BoundedVec, Entity, Timestamp, validate_name},
    error::EntityError,
};

// Closed vocabulary: one value per row seeded in the `languages` table. Seeding or retiring
// a language means updating this constant.
pub const MAX_CHAPTER_LOCALIZATIONS: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "f32")]
pub struct ChapterNumber(f32);

impl TryFrom<f32> for ChapterNumber {
    type Error = EntityError;

    fn try_from(n: f32) -> Result<Self, Self::Error> {
        const MIN: f32 = 0.0;
        const MAX: f32 = 99_999.99;
        const DECIMALS: i32 = 2;
        static PATTERN: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^\d{1,5}(?:\.\d{1,2})?$").unwrap());

        if !PATTERN.is_match(&n.to_string()) {
            return Err(EntityError::ChapterNumberOutOfRange(n, MIN, MAX, DECIMALS));
        }

        Ok(Self(n))
    }
}

impl ChapterNumber {
    pub fn as_f32(self) -> f32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "i16")]
pub struct ChapterVolume(i16);

impl TryFrom<i16> for ChapterVolume {
    type Error = EntityError;

    fn try_from(v: i16) -> Result<Self, Self::Error> {
        const MIN: i16 = 0;
        const MAX: i16 = 1_000;

        if !(MIN..=MAX).contains(&v) {
            return Err(EntityError::ChapterVolumeOutOfRange(v, MIN, MAX));
        }

        Ok(Self(v))
    }
}

impl ChapterVolume {
    pub fn as_i16(self) -> i16 {
        self.0
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ChapterName(String);

impl TryFrom<String> for ChapterName {
    type Error = EntityError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Ok(Self(validate_name(s)?))
    }
}

impl AsRef<str> for ChapterName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Chapter {
    pub id: Uuid,
    pub book_id: Uuid,
    pub number: ChapterNumber,
    pub name: Option<ChapterName>,
    pub volume: Option<ChapterVolume>,
    pub localizations: ChapterLocalizations,
    pub updated_at: Option<Timestamp>,
    pub created_at: Timestamp,
}

impl Entity for Chapter {
    const NAME: &'static str = "Chapter";
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterLocalization {
    pub language_id: Uuid,
    pub name: ChapterName,
}

pub struct ChapterLocalizationsBound;

impl Bounded for ChapterLocalizationsBound {
    const MAX: usize = MAX_CHAPTER_LOCALIZATIONS;
    const NAME: &'static str = "chapter localizations";
}

pub type ChapterLocalizations = BoundedVec<ChapterLocalization, ChapterLocalizationsBound>;

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::entity::{
        ChapterLocalization, ChapterLocalizations, ChapterName, ChapterNumber, ChapterVolume,
        MAX_CHAPTER_LOCALIZATIONS,
    };

    fn sample_localization() -> ChapterLocalization {
        ChapterLocalization {
            language_id: Uuid::now_v7(),
            name: ChapterName::try_from("Chapter 1".to_owned()).unwrap(),
        }
    }

    #[test]
    fn chapter_localizations_at_the_ceiling_is_valid() {
        let localizations: Vec<ChapterLocalization> = (0..MAX_CHAPTER_LOCALIZATIONS)
            .map(|_| sample_localization())
            .collect();

        assert!(ChapterLocalizations::try_from(localizations).is_ok());
    }

    #[test]
    fn chapter_localizations_over_the_ceiling_is_rejected() {
        let localizations: Vec<ChapterLocalization> = (0..=MAX_CHAPTER_LOCALIZATIONS)
            .map(|_| sample_localization())
            .collect();

        assert!(ChapterLocalizations::try_from(localizations).is_err());
    }

    #[test]
    fn chapter_number_zero_is_valid() {
        let res = ChapterNumber::try_from(0.);
        assert!(res.is_ok());
    }

    #[test]
    fn chapter_number_fractional_is_valid() {
        let res = ChapterNumber::try_from(12.5);
        assert!(res.is_ok());
    }

    #[test]
    fn chapter_number_negative_is_rejected() {
        let res = ChapterNumber::try_from(-1.0);
        assert!(res.is_err());
    }

    #[test]
    fn chapter_number_negative_zero_is_rejected() {
        let res = ChapterNumber::try_from(-0.0);
        assert!(res.is_err());
    }

    #[test]
    fn chapter_number_at_limit_is_valid() {
        let res = ChapterNumber::try_from(99_999.99);
        assert!(res.is_ok());
    }

    #[test]
    fn chapter_number_above_limit_is_rejected() {
        let res = ChapterNumber::try_from(100_000.0);
        assert!(res.is_err());
    }

    #[test]
    fn chapter_number_infinity_is_rejected() {
        let res = ChapterNumber::try_from(f32::INFINITY);
        assert!(res.is_err());
    }

    #[test]
    fn chapter_number_two_decimals_is_valid() {
        let res = ChapterNumber::try_from(12.34);
        assert!(res.is_ok());
    }

    #[test]
    fn chapter_number_three_decimals_is_rejected() {
        let res = ChapterNumber::try_from(12.345);
        assert!(res.is_err());
    }

    #[test]
    fn chapter_number_nan_is_rejected() {
        let res = ChapterNumber::try_from(f32::NAN);
        assert!(res.is_err());
    }

    #[test]
    fn deserialized_chapter_number_above_limit_is_rejected() {
        let res = serde_json::from_str::<ChapterNumber>("100000.0");
        assert!(res.is_err());
    }

    #[test]
    fn chapter_number_serializes_as_a_bare_number() {
        let number = ChapterNumber::try_from(12.5).unwrap();

        assert_eq!(serde_json::to_string(&number).unwrap(), "12.5");
    }

    #[test]
    fn chapter_name_255_chars_is_valid() {
        let res = ChapterName::try_from("ё".repeat(255));
        assert!(res.is_ok());
    }

    #[test]
    fn chapter_name_longer_256_chars_is_rejected() {
        let res = ChapterName::try_from("ё".repeat(256));
        assert!(res.is_err());
    }

    #[test]
    fn whitespace_only_chapter_name_is_rejected() {
        let res = ChapterName::try_from(" ".to_owned());
        assert!(res.is_err());
    }

    #[test]
    fn volume_zero_is_valid() {
        let res = ChapterVolume::try_from(0);
        assert!(res.is_ok());
    }

    #[test]
    fn volume_negative_is_rejected() {
        let res = ChapterVolume::try_from(-1);
        assert!(res.is_err());
    }

    #[test]
    fn volume_at_limit_is_valid() {
        let res = ChapterVolume::try_from(1_000);
        assert!(res.is_ok());
    }

    #[test]
    fn volume_above_limit_is_rejected() {
        let res = ChapterVolume::try_from(1_001);
        assert!(res.is_err());
    }

    #[test]
    fn deserialized_volume_out_of_range_is_rejected() {
        for v in ["-1", "1001"] {
            assert!(serde_json::from_str::<ChapterVolume>(v).is_err());
        }
    }

    #[test]
    fn volume_serializes_as_a_bare_number() {
        let volume = ChapterVolume::try_from(3).unwrap();

        assert_eq!(serde_json::to_string(&volume).unwrap(), "3");
    }
}
