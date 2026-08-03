use serde::{Deserialize, Serialize};
use std::str::FromStr;
use time::OffsetDateTime;
use unicode_segmentation::UnicodeSegmentation;
use uuid::Uuid;

use crate::error::EntityError;

#[derive(Debug, PartialEq, Eq, sqlx::Type, Deserialize, Serialize)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "camelCase")]
pub enum CreatorRole {
    Artist,
    Author,
}

impl FromStr for CreatorRole {
    type Err = EntityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "artist" => Ok(Self::Artist),
            "author" => Ok(Self::Author),
            other => Err(EntityError::InvalidCreatorRole(other.to_owned())),
        }
    }
}

impl AsRef<str> for CreatorRole {
    fn as_ref(&self) -> &str {
        match self {
            Self::Artist => "artist",
            Self::Author => "author",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Creator {
    pub id: Uuid,
    pub first_name: Name,
    pub last_name: Name,
    pub role: CreatorRole,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Name(String);

impl TryFrom<String> for Name {
    type Error = EntityError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        if s.trim().is_empty() {
            return Err(EntityError::NameIsEmptyOrWhitespace(s));
        }
        if s.graphemes(true).count() > 255 {
            return Err(EntityError::NameExceedsCharLimit(s));
        }
        if !s.chars().all(|c| c.is_alphabetic()) {
            return Err(EntityError::NameContainsNotLetters(s));
        }
        Ok(Self(s))
    }
}

impl AsRef<str> for Name {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::entity::Name;

    #[test]
    fn whitespace_only_name_is_rejected() {
        let res = Name::try_from(" ".to_owned());
        assert!(res.is_err());
    }

    #[test]
    fn empty_string_is_rejected() {
        let res = Name::try_from("".to_owned());
        assert!(res.is_err());
    }

    #[test]
    fn name_255_graphemes_is_valid() {
        let res = Name::try_from("ё".repeat(255));
        assert!(res.is_ok());
    }

    #[test]
    fn name_longer_256_graphemes_is_rejected() {
        let res = Name::try_from("ё".repeat(256));
        assert!(res.is_err());
    }

    #[test]
    fn not_letters_are_rejected() {
        let res = Name::try_from("123".to_owned());
        assert!(res.is_err());
    }
}
