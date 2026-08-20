use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{entity::validate_name, error::EntityError};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
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
#[serde(transparent)]
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
#[serde(transparent)]
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
    pub localizations: Vec<ChapterLocalization>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub updated_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterLocalization {
    pub language_id: Uuid,
    pub name: ChapterName,
}

#[cfg(test)]
mod tests {
    use crate::entity::{ChapterName, ChapterNumber, ChapterVolume};

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
}
