use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::EntityError;

const NUMBER_MAX: f32 = 99_999.99;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ChapterNumber(f32);

impl TryFrom<f32> for ChapterNumber {
    type Error = EntityError;

    fn try_from(n: f32) -> Result<Self, Self::Error> {
        if !n.is_finite() {
            return Err(EntityError::ChapterNumberIsNotFinite(n));
        }
        if n < 0.0 {
            return Err(EntityError::ChapterNumberIsNegative(n));
        }
        if n > NUMBER_MAX {
            return Err(EntityError::ChapterNumberExceedsLimit(n, NUMBER_MAX));
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
pub struct Volume(i16);

impl TryFrom<i16> for Volume {
    type Error = EntityError;

    fn try_from(v: i16) -> Result<Self, Self::Error> {
        if v < 0 {
            return Err(EntityError::VolumeIsNegative(v));
        }

        Ok(Self(v))
    }
}

impl Volume {
    pub fn as_i16(self) -> i16 {
        self.0
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ChapterTitle(String);

impl TryFrom<String> for ChapterTitle {
    type Error = EntityError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        if s.trim().is_empty() {
            return Err(EntityError::NameIsEmptyOrWhitespace);
        }
        if s.chars().count() > 255 {
            return Err(EntityError::NameExceedsCharLimit(s));
        }
        Ok(Self(s))
    }
}

impl AsRef<str> for ChapterTitle {
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
    pub name: Option<ChapterTitle>,
    pub volume: Option<Volume>,
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
    pub name: ChapterTitle,
}

#[cfg(test)]
mod tests {
    use crate::entity::{ChapterNumber, ChapterTitle, Volume, chapter::NUMBER_MAX};

    #[test]
    fn chapter_number_zero_is_valid() {
        let res = ChapterNumber::try_from(0.0);
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
    fn chapter_number_at_limit_is_valid() {
        let res = ChapterNumber::try_from(NUMBER_MAX);
        assert!(res.is_ok());
    }

    #[test]
    fn chapter_number_above_limit_is_rejected() {
        let res = ChapterNumber::try_from(NUMBER_MAX + 1.0);
        assert!(res.is_err());
    }

    #[test]
    fn chapter_number_infinity_is_rejected() {
        let res = ChapterNumber::try_from(f32::INFINITY);
        assert!(res.is_err());
    }

    #[test]
    fn chapter_number_nan_is_rejected() {
        let res = ChapterNumber::try_from(f32::NAN);
        assert!(res.is_err());
    }

    #[test]
    fn chapter_title_255_chars_is_valid() {
        let res = ChapterTitle::try_from("ё".repeat(255));
        assert!(res.is_ok());
    }

    #[test]
    fn chapter_title_longer_256_chars_is_rejected() {
        let res = ChapterTitle::try_from("ё".repeat(256));
        assert!(res.is_err());
    }

    #[test]
    fn whitespace_only_chapter_title_is_rejected() {
        let res = ChapterTitle::try_from(" ".to_owned());
        assert!(res.is_err());
    }

    #[test]
    fn volume_zero_is_valid() {
        let res = Volume::try_from(0);
        assert!(res.is_ok());
    }

    #[test]
    fn volume_negative_is_rejected() {
        let res = Volume::try_from(-1);
        assert!(res.is_err());
    }
}
