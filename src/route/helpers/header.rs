use std::net::IpAddr;

use axum::http::{HeaderMap, header};

use crate::entity::ShortText;

pub fn user_agent(headers: &HeaderMap) -> Option<ShortText> {
    headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| ShortText::try_from(v.to_owned()).ok())
}

pub fn forwarded_ip(headers: &HeaderMap) -> Option<IpAddr> {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .and_then(|v| v.trim().parse().ok())
}
