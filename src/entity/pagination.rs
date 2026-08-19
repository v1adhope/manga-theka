use serde::Deserialize;
use uuid::Uuid;

use crate::error::EntityError;

pub const DEFAULT_LIMIT: u32 = 20;
pub const MAX_LIMIT: u32 = 100;

#[derive(Debug, Clone, Copy)]
pub struct Limit(u32);

impl TryFrom<u32> for Limit {
    type Error = EntityError;

    fn try_from(v: u32) -> Result<Self, Self::Error> {
        if (1..=MAX_LIMIT).contains(&v) {
            return Ok(Self(v));
        }
        Err(EntityError::LimitOutOfRange(v, MAX_LIMIT))
    }
}

impl Limit {
    pub fn as_u32(self) -> u32 {
        self.0
    }
}

#[derive(Debug)]
pub struct Pagination {
    pub after: Option<Uuid>,
    pub limit: Option<Limit>,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub enum SortOrder {
    #[default]
    Asc,
    Desc,
}

#[cfg(test)]
mod tests {
    use crate::entity::pagination::{Limit, MAX_LIMIT};

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
        let res = Limit::try_from(MAX_LIMIT);
        assert!(res.is_ok());
    }

    #[test]
    fn limit_above_max_is_rejected() {
        let res = Limit::try_from(MAX_LIMIT + 1);
        assert!(res.is_err());
    }
}
