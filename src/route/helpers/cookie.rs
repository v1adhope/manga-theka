use axum_extra::extract::{
    CookieJar,
    cookie::{Cookie, SameSite},
};
use time::Duration;

pub const REFRESH_COOKIE: &str = "refresh_token";
const REFRESH_COOKIE_PATH: &str = "/sessions";

pub fn refresh_cookie(value: String, ttl: i64) -> Cookie<'static> {
    Cookie::build((REFRESH_COOKIE, value))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Strict)
        .path(REFRESH_COOKIE_PATH)
        .max_age(Duration::seconds(ttl))
        .build()
}

pub fn clear_refresh_cookie(jar: CookieJar) -> CookieJar {
    jar.add(
        Cookie::build((REFRESH_COOKIE, ""))
            .http_only(true)
            .secure(true)
            .same_site(SameSite::Strict)
            .path(REFRESH_COOKIE_PATH)
            .max_age(Duration::ZERO)
            .build(),
    )
}
