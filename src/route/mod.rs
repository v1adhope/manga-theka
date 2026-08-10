mod content_rating;
mod creator;
mod healthz;
mod label;
mod language;

pub use content_rating::*;
pub use creator::*;
pub use healthz::*;
pub use label::*;
pub use language::*;

use axum::{Json, http::StatusCode};
use serde::Serialize;
use serde_json::json;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreResp {
    pub id: Uuid,
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
