use serde::{Deserialize, Serialize};

use crate::error::EntityError;

mod book;
mod bounded_vec;
mod chapter;
mod content_rating;
mod creator;
mod filter;
mod image;
mod label;
mod language;
mod release;
mod resource_url;

pub use book::*;
pub use bounded_vec::*;
pub use chapter::*;
pub use content_rating::*;
pub use creator::*;
pub use filter::*;
pub use image::*;
pub use label::*;
pub use language::*;
pub use release::*;
pub use resource_url::*;

pub trait Entity {
    const NAME: &'static str;
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "i32")]
pub struct Ordinal(i32);

impl TryFrom<i32> for Ordinal {
    type Error = EntityError;

    fn try_from(n: i32) -> Result<Self, Self::Error> {
        const MIN: i32 = 1;

        if n < MIN {
            return Err(EntityError::OrdinalOutOfRange(n, MIN));
        }

        Ok(Self(n))
    }
}

impl Ordinal {
    pub fn as_i32(self) -> i32 {
        self.0
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Name(String);

impl TryFrom<String> for Name {
    type Error = EntityError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        let s = validate_name(s)?;
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

fn validate_name(s: String) -> Result<String, EntityError> {
    if s.trim().is_empty() {
        return Err(EntityError::NameIsEmptyOrWhitespace);
    }
    if s.chars().count() > 255 {
        return Err(EntityError::NameExceedsCharLimit(s));
    }
    Ok(s)
}

#[cfg(test)]
mod tests {
    use crate::entity::{Name, Ordinal};

    #[test]
    fn ordinal_at_the_first_position_is_valid() {
        let ordinal = Ordinal::try_from(1).unwrap();

        assert_eq!(ordinal.as_i32(), 1);
    }

    #[test]
    fn non_positive_ordinal_is_rejected() {
        for n in [0, -1] {
            assert!(Ordinal::try_from(n).is_err());
        }
    }

    #[test]
    fn deserialized_non_positive_ordinal_is_rejected() {
        for n in ["0", "-1"] {
            assert!(serde_json::from_str::<Ordinal>(n).is_err());
        }
    }

    #[test]
    fn ordinal_serializes_as_a_bare_number() {
        let ordinal = Ordinal::try_from(1).unwrap();

        assert_eq!(serde_json::to_string(&ordinal).unwrap(), "1");
    }

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
    fn name_255_chars_is_valid() {
        let res = Name::try_from("ё".repeat(255));
        assert!(res.is_ok());
    }

    #[test]
    fn name_longer_256_chars_is_rejected() {
        let res = Name::try_from("ё".repeat(256));
        assert!(res.is_err());
    }

    #[test]
    fn not_letters_are_rejected() {
        let res = Name::try_from("123".to_owned());
        assert!(res.is_err());
    }
}
