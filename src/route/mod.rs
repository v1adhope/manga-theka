mod book;
mod chapter;
mod content_rating;
mod creator;
mod healthz;
mod label;
mod language;

pub use book::*;
pub use chapter::*;
pub use content_rating::*;
pub use creator::*;
pub use healthz::*;
pub use label::*;
pub use language::*;

use axum::{Json, http::StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::{
    entity::{Limit, Pagination},
    error::EntityError,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreResp {
    pub id: Uuid,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginationQuery {
    pub after: Option<Uuid>,
    pub limit: Option<u32>,
}

impl TryFrom<PaginationQuery> for Pagination {
    type Error = EntityError;

    fn try_from(q: PaginationQuery) -> Result<Self, Self::Error> {
        Ok(Self {
            after: q.after,
            limit: q.limit.map(Limit::try_from).transpose()?,
        })
    }
}

pub fn json_response<T: Serialize>(status: StatusCode, body: T) -> (StatusCode, Json<T>) {
    (status, Json(body))
}

pub fn json_data_response<T: Serialize>(
    status: StatusCode,
    data: T,
) -> (StatusCode, Json<serde_json::Value>) {
    (status, Json(json!({ "data": data })))
}
