use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::error::EntityError;

mod book;
mod bounded_vec;
mod chapter;
mod content_rating;
mod creator;
mod feedback;
mod filter;
mod image;
mod label;
mod language;
mod release;
mod resource_url;
mod user;
mod visibility;

pub use book::*;
pub use bounded_vec::*;
pub use chapter::*;
pub use content_rating::*;
pub use creator::*;
pub use feedback::*;
pub use filter::*;
pub use image::*;
pub use label::*;
pub use language::*;
pub use release::*;
pub use resource_url::*;
pub use user::*;
pub use visibility::*;

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
    let s = s.trim();
    if s.is_empty() {
        return Err(EntityError::NameIsEmptyOrWhitespace);
    }
    if s.chars().count() > 255 {
        return Err(EntityError::NameExceedsCharLimit(s.to_owned()));
    }
    Ok(s.to_owned())
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Email(String);

impl TryFrom<String> for Email {
    type Error = EntityError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        static PATTERN: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+$").unwrap()
        });

        if s.chars().count() > 254 {
            return Err(EntityError::EmailExceedsCharLimit);
        }
        if !PATTERN.is_match(&s) {
            return Err(EntityError::EmailIsMalformed);
        }
        Ok(Self(s))
    }
}

impl AsRef<str> for Email {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Text(String);

impl TryFrom<String> for Text {
    type Error = EntityError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        let s = s.trim();
        if s.is_empty() {
            return Err(EntityError::TextIsEmptyOrWhitespace);
        }
        if s.chars().count() > 2000 {
            return Err(EntityError::TextExceedsCharLimit);
        }
        Ok(Self(s.to_owned()))
    }
}

impl AsRef<str> for Text {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::entity::{Email, Name, Ordinal, Text};

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
    fn name_is_trimmed() {
        let name = Name::try_from(" \tё\n ".to_owned()).unwrap();

        assert_eq!(name.as_ref(), "ё");
    }

    #[test]
    fn name_255_chars_with_surrounding_whitespace_is_valid() {
        let res = Name::try_from(format!(" {} ", "ё".repeat(255)));
        assert!(res.is_ok());
    }

    #[test]
    fn not_letters_are_rejected() {
        let res = Name::try_from("123".to_owned());
        assert!(res.is_err());
    }

    #[test]
    fn text_2000_chars_is_valid() {
        let res = Text::try_from("ё".repeat(2000));
        assert!(res.is_ok());
    }

    #[test]
    fn text_longer_2001_chars_is_rejected() {
        let res = Text::try_from("ё".repeat(2001));
        assert!(res.is_err());
    }

    #[test]
    fn text_is_trimmed() {
        let text = Text::try_from(" \tё\n ".to_owned()).unwrap();

        assert_eq!(text.as_ref(), "ё");
    }

    #[test]
    fn text_2000_chars_with_surrounding_whitespace_is_valid() {
        let res = Text::try_from(format!(" {} ", "ё".repeat(2000)));
        assert!(res.is_ok());
    }

    #[test]
    fn whitespace_only_text_is_rejected() {
        let res = Text::try_from(" ".to_owned());
        assert!(res.is_err());
    }

    #[test]
    fn empty_text_is_rejected() {
        let res = Text::try_from(String::new());
        assert!(res.is_err());
    }

    #[test]
    fn well_formed_emails_are_valid() {
        for s in ["a@b.co", "A@B.CO", "a.b+c-d_e@sub.example.co.uk"] {
            assert!(Email::try_from(s.to_owned()).is_ok(), "{s} must be valid");
        }
    }

    #[test]
    fn malformed_emails_are_rejected() {
        for s in ["", "nope", "a b@c.co", "a@b", "@b.co", "a@.co"] {
            assert!(
                Email::try_from(s.to_owned()).is_err(),
                "{s} must be rejected"
            );
        }
    }

    #[test]
    fn email_254_chars_is_valid() {
        let local = "a".repeat(249);
        let email = format!("{local}@b.co");
        assert_eq!(email.chars().count(), 254);

        let res = Email::try_from(email);
        assert!(res.is_ok());
    }

    #[test]
    fn email_longer_255_chars_is_rejected() {
        let local = "a".repeat(250);
        let email = format!("{local}@b.co");
        assert_eq!(email.chars().count(), 255);

        let res = Email::try_from(email);
        assert!(res.is_err());
    }
}
