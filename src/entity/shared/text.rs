use serde::{Deserialize, Serialize};

use crate::error::EntityError;

fn parse_text(s: String, max: usize) -> Result<String, EntityError> {
    let s = s.trim();
    if s.is_empty() {
        return Err(EntityError::TextIsEmptyOrWhitespace);
    }
    if s.chars().count() > max {
        return Err(EntityError::TextExceedsCharLimit(max));
    }
    Ok(s.to_owned())
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct Text(String);

impl TryFrom<String> for Text {
    type Error = EntityError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        const MAX: usize = 2000;

        Ok(Self(parse_text(s, MAX)?))
    }
}

impl AsRef<str> for Text {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct ShortText(String);

impl TryFrom<String> for ShortText {
    type Error = EntityError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        const MAX: usize = 255;

        Ok(Self(parse_text(s, MAX)?))
    }
}

impl AsRef<str> for ShortText {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::{ShortText, Text};

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
    fn short_text_255_chars_is_valid() {
        let res = ShortText::try_from("ё".repeat(255));
        assert!(res.is_ok());
    }

    #[test]
    fn short_text_longer_256_chars_is_rejected() {
        let res = ShortText::try_from("ё".repeat(256));
        assert!(res.is_err());
    }

    #[test]
    fn short_text_is_trimmed() {
        let text = ShortText::try_from(" \tё\n ".to_owned()).unwrap();

        assert_eq!(text.as_ref(), "ё");
    }

    #[test]
    fn whitespace_only_short_text_is_rejected() {
        let res = ShortText::try_from(" ".to_owned());
        assert!(res.is_err());
    }
}
