use axum::{Json, http::StatusCode};
use axum_extra::extract::CookieJar;
use serde::Serialize;
use serde_json::json;
use uuid::Uuid;

use crate::{entity::SessionTokens, route::cookie::refresh_cookie};

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

pub fn session_tokens_response(
    jar: CookieJar,
    tokens: SessionTokens,
) -> (CookieJar, Json<serde_json::Value>) {
    let jar = jar.add(refresh_cookie(tokens.refresh.value, tokens.refresh.ttl));
    let body = json_data(AccessTokenResp {
        access_token: tokens.access.value,
        expires_in: tokens.access.ttl,
    });

    (jar, body)
}
