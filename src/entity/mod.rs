use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, UtcOffset};

use crate::error::EntityError;

#[path = "book/book.rs"]
mod book;
#[path = "book/cover.rs"]
mod book_cover;
#[path = "book/filter.rs"]
mod book_filter;
mod bounded_vec;
mod chapter;
mod content_rating;
mod creator;
mod feedback;
mod filter;
mod image;
mod label;
mod language;
mod range;
mod release;
mod resource_url;
mod session;
mod user;
mod visibility;

pub use book::*;
pub use book_cover::*;
pub use book_filter::*;
pub use bounded_vec::*;
pub use chapter::*;
pub use content_rating::*;
pub use creator::*;
pub use feedback::*;
pub use filter::*;
pub use image::*;
pub use label::*;
pub use language::*;
pub use range::*;
pub use release::*;
pub use resource_url::*;
pub use session::*;
pub use user::*;
pub use visibility::*;

/// Implement on the canonical command struct, not its `*Query` read sibling.
pub trait Entity {
    const NAME: &'static str;
}

// TODO: replace all OffsetDateTime to Timestamp
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct Timestamp(#[serde(with = "time::serde::rfc3339")] OffsetDateTime);

impl From<OffsetDateTime> for Timestamp {
    fn from(at: OffsetDateTime) -> Self {
        Self(at.to_offset(UtcOffset::UTC))
    }
}

impl Timestamp {
    pub const fn into_inner(self) -> OffsetDateTime {
        self.0
    }
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

fn parse_hex(s: String, len: usize) -> Result<String, EntityError> {
    static PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[0-9a-f]+$").unwrap());

    if s.len() != len || !PATTERN.is_match(&s) {
        return Err(EntityError::HexHashIsMalformed(len));
    }
    Ok(s)
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(transparent)]
pub struct ShortHexHash(String);

impl ShortHexHash {
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl TryFrom<String> for ShortHexHash {
    type Error = EntityError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        const LEN: usize = 16;

        Ok(Self(parse_hex(s, LEN)?))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HexHash(String);

impl TryFrom<String> for HexHash {
    type Error = EntityError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        const LEN: usize = 64;

        Ok(Self(parse_hex(s, LEN)?))
    }
}

impl AsRef<str> for HexHash {
    fn as_ref(&self) -> &str {
        &self.0
    }
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

        let s = s.trim().to_lowercase();

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
    use crate::entity::{Email, HexHash, Name, Ordinal, ShortHexHash, Text};

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
    fn email_is_trimmed_and_lowercased() {
        let email = Email::try_from(" A@B.CO ".to_owned()).unwrap();

        assert_eq!(email.as_ref(), "a@b.co");
    }

    #[test]
    fn email_254_char_boundary_measures_the_trimmed_value() {
        let local = "a".repeat(249);
        let email = format!("  {local}@b.co  ");

        let res = Email::try_from(email);
        assert!(
            res.is_ok(),
            "surrounding whitespace must not count toward the limit"
        );
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

    #[test]
    fn a_lowercase_short_hex_hash_of_the_exact_length_is_valid() {
        let res = ShortHexHash::try_from("0123456789abcdef".to_owned());
        assert!(res.is_ok());
    }

    #[test]
    fn a_short_hex_hash_of_another_length_is_rejected() {
        for s in ["", "0123456789abcde", "0123456789abcdef0"] {
            assert!(
                ShortHexHash::try_from(s.to_owned()).is_err(),
                "{s} must be rejected"
            );
        }
    }

    #[test]
    fn a_short_hex_hash_outside_the_lowercase_alphabet_is_rejected() {
        for s in ["0123456789ABCDEF", "0123456789abcdeg", "0123456789abcde "] {
            assert!(
                ShortHexHash::try_from(s.to_owned()).is_err(),
                "{s} must be rejected"
            );
        }
    }

    #[test]
    fn a_full_length_hex_hash_is_valid() {
        let res = HexHash::try_from("0123456789abcdef".repeat(4));
        assert!(res.is_ok());
    }

    #[test]
    fn a_hex_hash_of_another_length_is_rejected() {
        for s in [String::new(), "ab".repeat(31), "ab".repeat(33)] {
            assert!(
                HexHash::try_from(s.clone()).is_err(),
                "{s:?} must be rejected"
            );
        }
    }
}
