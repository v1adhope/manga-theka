use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, sqlx::FromRow, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentRating {
    pub id: Uuid,
    pub name: String,
    pub code: String,
}
