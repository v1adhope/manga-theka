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

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;
    use axum_extra::extract::{CookieJar, cookie::SameSite};
    use serde_json::json;
    use time::Duration;
    use uuid::Uuid;

    use super::{
        AccessTokenResp, StoreResp, json_data, json_data_response, json_response,
        session_tokens_response,
    };
    use crate::{fixtures::session_tokens, route::cookie::REFRESH_COOKIE};

    #[test]
    fn store_resp_carries_the_id_under_an_id_key() {
        let id = Uuid::now_v7();

        let value = serde_json::to_value(StoreResp { id }).unwrap();

        assert_eq!(value, json!({ "id": id }));
    }

    #[test]
    fn access_token_resp_uses_camel_case_keys() {
        let value = serde_json::to_value(AccessTokenResp {
            access_token: "at".to_owned(),
            expires_in: 900,
        })
        .unwrap();

        assert_eq!(value, json!({ "accessToken": "at", "expiresIn": 900 }));
    }

    #[test]
    fn json_response_pairs_the_status_with_the_untouched_body() {
        let id = Uuid::now_v7();

        let (status, body) = json_response(StatusCode::CREATED, StoreResp { id });

        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(body.0.id, id);
    }

    #[test]
    fn json_data_wraps_the_payload_in_a_data_envelope() {
        let body = json_data(json!({ "k": "v" }));

        assert_eq!(body.0, json!({ "data": { "k": "v" } }));
    }

    #[test]
    fn json_data_response_pairs_the_status_with_the_enveloped_payload() {
        let (status, body) = json_data_response(StatusCode::ACCEPTED, json!([1, 2]));

        assert_eq!(status, StatusCode::ACCEPTED);
        assert_eq!(body.0, json!({ "data": [1, 2] }));
    }

    #[test]
    fn session_tokens_response_body_exposes_only_the_access_token() {
        let (_, body) = session_tokens_response(CookieJar::new(), session_tokens());

        assert_eq!(
            body.0,
            json!({ "data": { "accessToken": "access-value", "expiresIn": 900 } })
        );
        assert!(
            !body.0.to_string().contains("refresh-value"),
            "the refresh token never reaches the response body"
        );
    }

    #[test]
    fn session_tokens_response_sets_a_hardened_refresh_cookie() {
        let (jar, _) = session_tokens_response(CookieJar::new(), session_tokens());

        let cookie = jar.get(REFRESH_COOKIE).expect("refresh cookie is set");

        assert_eq!(cookie.value(), "refresh-value");
        assert_eq!(cookie.max_age(), Some(Duration::seconds(1_209_600)));
        assert_eq!(cookie.http_only(), Some(true));
        assert_eq!(cookie.secure(), Some(true));
        assert_eq!(cookie.same_site(), Some(SameSite::Strict));
        assert_eq!(cookie.path(), Some("/sessions"));
    }
}
