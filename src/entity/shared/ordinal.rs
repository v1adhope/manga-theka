use serde::{Deserialize, Serialize};

use crate::error::EntityError;

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

#[cfg(test)]
mod tests {
    use super::Ordinal;

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
}
