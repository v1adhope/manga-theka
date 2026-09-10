use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

use crate::error::EntityError;

fn parse_hex(s: String, len: usize) -> Result<String, EntityError> {
    static PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[0-9a-f]+$").unwrap());

    if s.len() != len || !PATTERN.is_match(&s) {
        return Err(EntityError::HexHashIsMalformed(len));
    }
    Ok(s)
}

#[derive(Debug, Clone, PartialEq, Serialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HexHash(String);

impl HexHash {
    pub fn into_inner(self) -> String {
        self.0
    }
}

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

#[cfg(test)]
mod tests {
    use super::{HexHash, ShortHexHash};

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
