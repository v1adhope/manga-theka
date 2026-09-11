use serde::Deserialize;
use uuid::Uuid;

use crate::{
    entity::{Filter, Limit, SortOrder},
    error::EntityError,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginationQuery {
    pub after: Option<Uuid>,
    pub limit: Option<i64>,
    pub order: Option<SortOrder>,
}

impl TryFrom<PaginationQuery> for Filter {
    type Error = EntityError;

    fn try_from(q: PaginationQuery) -> Result<Self, Self::Error> {
        Ok(Self {
            after: q.after,
            limit: q
                .limit
                .map(Limit::try_from)
                .transpose()?
                .unwrap_or_default(),
            sort_order: q.order.unwrap_or_default(),
        })
    }
}
