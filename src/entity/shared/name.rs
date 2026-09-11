use serde::{Deserialize, Serialize};

use crate::error::EntityError;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String")]
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

pub(crate) fn validate_name(s: String) -> Result<String, EntityError> {
    let s = s.trim();
    if s.is_empty() {
        return Err(EntityError::NameIsEmptyOrWhitespace);
    }
    if s.chars().count() > 255 {
        return Err(EntityError::NameExceedsCharLimit(s.to_owned()));
    }
    Ok(s.to_owned())
}

#[cfg(test)]
mod tests {
    use super::Name;

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
}
