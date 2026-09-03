use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::EntityError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Limit(i64);

impl Limit {
    pub const DEFAULT: Self = Self(20);
    pub const MAX: Self = Self(100);

    pub fn as_i64(self) -> i64 {
        self.0
    }
}

impl Default for Limit {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl TryFrom<i64> for Limit {
    type Error = EntityError;

    fn try_from(v: i64) -> Result<Self, Self::Error> {
        let max = Self::MAX.as_i64();

        if (1..=max).contains(&v) {
            return Ok(Self(v));
        }
        Err(EntityError::LimitOutOfRange(v, max))
    }
}

#[derive(Debug)]
pub struct Filter {
    pub after: Option<Uuid>,
    pub limit: Limit,
    pub sort_order: SortOrder,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize, Default)]
pub enum SortOrder {
    Asc,
    #[default]
    Desc,
}

#[cfg(test)]
mod tests {
    use crate::entity::filter::{Limit, SortOrder};

    #[test]
    fn limit_zero_is_rejected() {
        let res = Limit::try_from(0);
        assert!(res.is_err());
    }

    #[test]
    fn limit_one_is_valid() {
        let res = Limit::try_from(1);
        assert!(res.is_ok());
    }

    #[test]
    fn limit_max_is_valid() {
        let res = Limit::try_from(Limit::MAX.as_i64());
        assert!(res.is_ok());
    }

    #[test]
    fn limit_above_max_is_rejected() {
        let res = Limit::try_from(Limit::MAX.as_i64() + 1);
        assert!(res.is_err());
    }

    #[test]
    fn limit_defaults_to_the_default_ceiling() {
        assert_eq!(Limit::default(), Limit::DEFAULT);
    }

    #[test]
    fn sort_order_defaults_to_descending() {
        assert!(matches!(SortOrder::default(), SortOrder::Desc));
    }
}
