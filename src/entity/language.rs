use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Default, sqlx::FromRow, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Language {
    pub id: Uuid,
    pub code: String,
    pub name: String,
}
