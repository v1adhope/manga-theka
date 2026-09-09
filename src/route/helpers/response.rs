use axum::{Json, http::StatusCode};
use serde::Serialize;
use serde_json::json;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreResp {
    pub id: Uuid,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessTokenResp {
    pub access_token: String,
    pub expires_in: i64,
}

pub fn json_response<T: Serialize>(status: StatusCode, body: T) -> (StatusCode, Json<T>) {
    (status, Json(body))
}

pub fn json_data<T: Serialize>(data: T) -> Json<serde_json::Value> {
    Json(json!({ "data": data }))
}

pub fn json_data_response<T: Serialize>(
    status: StatusCode,
    data: T,
) -> (StatusCode, Json<serde_json::Value>) {
    (status, json_data(data))
}
