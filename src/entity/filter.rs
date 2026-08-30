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
pub struct Filter {
    pub after: Option<Uuid>,
    pub limit: Option<Limit>,
    pub sort_order: Option<SortOrder>,
}

impl Filter {
    pub fn builder() -> FilterBuilder {
        FilterBuilder::default()
    }

    pub fn effective_limit(&self) -> u32 {
        self.limit.map_or(DEFAULT_LIMIT, Limit::as_u32)
    }

    pub fn effective_sort_order(&self) -> SortOrder {
        self.sort_order.unwrap_or(SortOrder::Desc)
    }
}

#[derive(Debug, Default)]
pub struct FilterBuilder {
    after: Option<Uuid>,
    limit: Option<u32>,
    sort_order: Option<SortOrder>,
}

impl FilterBuilder {
    pub fn after(mut self, v: Option<Uuid>) -> Self {
        self.after = v;
        self
    }

    pub fn limit(mut self, v: Option<u32>) -> Self {
        self.limit = v;
        self
    }

    pub fn sort_order(mut self, v: Option<SortOrder>) -> Self {
        self.sort_order = v;
        self
    }

    pub fn build(self) -> Result<Filter, EntityError> {
        Ok(Filter {
            after: self.after,
            limit: self.limit.map(Limit::try_from).transpose()?,
            sort_order: self.sort_order,
        })
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub enum SortOrder {
    Asc,
    Desc,
}

#[cfg(test)]
mod tests {
    use crate::entity::filter::{DEFAULT_LIMIT, Filter, Limit, MAX_LIMIT, SortOrder};

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

    #[test]
    fn omitted_limit_falls_back_to_the_default() {
        let filter = Filter::builder().build().unwrap();

        assert_eq!(filter.effective_limit(), DEFAULT_LIMIT);
    }

    #[test]
    fn provided_limit_wins_over_the_default() {
        let filter = Filter::builder().limit(Some(1)).build().unwrap();

        assert_eq!(filter.effective_limit(), 1);
    }

    #[test]
    fn omitted_sort_order_falls_back_to_descending() {
        let filter = Filter::builder().build().unwrap();

        assert!(matches!(filter.effective_sort_order(), SortOrder::Desc));
    }

    #[test]
    fn provided_sort_order_wins_over_the_default() {
        let filter = Filter::builder()
            .sort_order(Some(SortOrder::Asc))
            .build()
            .unwrap();

        assert!(matches!(filter.effective_sort_order(), SortOrder::Asc));
    }
}
