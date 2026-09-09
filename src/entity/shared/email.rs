use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::error::EntityError;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String")]
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

#[cfg(test)]
mod tests {
    use super::Email;

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
}
