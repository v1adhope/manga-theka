use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use axum::response::Response;
use axum_test::multipart::{MultipartForm, Part};
use http_body_util::BodyExt;
use manga_theka::entity::{BookQuery, Chapter, Label, Role};
use serde::{Deserialize, de::DeserializeOwned};
use uuid::Uuid;

use super::app::TestApp;

#[derive(Deserialize, Debug)]
pub struct RespWrapper<T, C = Uuid> {
    pub data: T,
    #[serde(rename = "nextCursor", default)]
    pub next_cursor: Option<C>,
}

pub async fn assert_error(resp: Response, expected: StatusCode) {
    assert_eq!(resp.status(), expected);

    let body = resp.into_body().collect().await.unwrap().to_bytes();
    assert!(!body.is_empty(), "error response must carry a body");
}

pub async fn assert_stored(resp: Response) -> Uuid {
    assert_eq!(resp.status(), StatusCode::CREATED);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let id = v["data"]["id"]
        .as_str()
        .expect("created response must carry data.id");

    Uuid::parse_str(id).expect("data.id must be a uuid")
}

pub fn redirect_target(resp: &Response) -> &str {
    resp.headers()
        .get(header::LOCATION)
        .expect("a redirect must carry a location")
        .to_str()
        .expect("a location must be printable")
}

impl TestApp {
    pub async fn get_raw(&self, path: &str) -> Response {
        self.send_raw(Request::get(path).body(Body::empty()).unwrap())
            .await
    }

    async fn get_as(&self, path: &str, roles: &[Role]) -> Response {
        let req = Request::get(path)
            .header(
                header::AUTHORIZATION,
                self.bearer(Uuid::now_v7(), Uuid::now_v7(), roles),
            )
            .body(Body::empty())
            .unwrap();

        self.send_raw(req).await
    }

    pub async fn delete_authed(&self, path: &str) -> Response {
        self.send_authed(Request::delete(path).body(Body::empty()).unwrap())
            .await
    }

    pub async fn post_json(&self, path: &str, body: serde_json::Value) -> Response {
        self.json_authed(Method::POST, path, body).await
    }

    pub async fn put_json(&self, path: &str, body: serde_json::Value) -> Response {
        self.json_authed(Method::PUT, path, body).await
    }

    pub async fn post_raw(&self, path: &str, body: &str) -> Response {
        let req = Request::post(path)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_owned()))
            .unwrap();

        self.send_authed(req).await
    }

    async fn json_authed(&self, method: Method, path: &str, body: serde_json::Value) -> Response {
        let req = Request::builder()
            .method(method)
            .uri(path)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();

        self.send_authed(req).await
    }

    async fn multipart_authed(&self, path: &str, parts: &[&[u8]]) -> Response {
        let form = Self::multipart_body(parts);
        let req = Request::post(path)
            .header(header::CONTENT_TYPE, form.content_type())
            .body(Body::from(form))
            .unwrap();

        self.send_authed(req).await
    }

    fn multipart_body(parts: &[&[u8]]) -> MultipartForm {
        parts
            .iter()
            .enumerate()
            .fold(MultipartForm::new(), |form, (i, part)| {
                form.add_part(
                    format!("page{i}"),
                    Part::bytes(part.to_vec()).file_name(format!("page{i}")),
                )
            })
    }

    pub async fn get_ok_json<T: DeserializeOwned>(&self, path: &str) -> T {
        let resp = self.get_raw(path).await;
        assert_eq!(resp.status(), StatusCode::OK, "{path}");

        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).expect("failed to parse response body")
    }

    pub async fn get_ok_json_as<T: DeserializeOwned>(&self, path: &str, roles: &[Role]) -> T {
        let resp = self.get_as(path, roles).await;
        assert_eq!(resp.status(), StatusCode::OK, "{path}");

        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).expect("failed to parse response body")
    }

    pub async fn put_visibility(&self, id: Uuid, visibility: &str, note: Option<&str>) -> Response {
        let body = match note {
            Some(note) => serde_json::json!({ "visibility": visibility, "note": note }),
            None => serde_json::json!({ "visibility": visibility }),
        };

        self.json_authed(Method::PUT, &format!("/books/{id}/visibility"), body)
            .await
    }

    pub async fn get_body(&self, path: &str) -> (StatusCode, String) {
        let resp = self.get_raw(path).await;
        let status = resp.status();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();

        (status, String::from_utf8_lossy(&bytes).into_owned())
    }

    pub async fn get_body_as(&self, path: &str, roles: &[Role]) -> (StatusCode, String) {
        let resp = self.get_as(path, roles).await;
        let status = resp.status();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();

        (status, String::from_utf8_lossy(&bytes).into_owned())
    }

    pub async fn get_as_moderator(&self, path: &str) -> Response {
        self.get_as(path, &[Role::Moderator]).await
    }

    pub async fn post_cover(&self, book_id: Uuid, image: &[u8]) -> Response {
        self.multipart_authed(&format!("/books/{book_id}/covers"), &[image])
            .await
    }

    pub async fn get_covers(&self, book_id: Uuid) -> Response {
        self.get_raw(&format!("/books/{book_id}/covers")).await
    }

    pub async fn get_cover_image(&self, cover_id: Uuid) -> Response {
        self.get_raw(&format!("/covers/{cover_id}/image")).await
    }

    pub async fn put_main_cover(&self, book_id: Uuid, cover_id: Uuid) -> StatusCode {
        self.json_authed(
            Method::PUT,
            &format!("/books/{book_id}/main-cover"),
            serde_json::json!({ "coverId": cover_id }),
        )
        .await
        .status()
    }

    pub async fn delete_cover(&self, cover_id: Uuid) -> Response {
        self.delete_authed(&format!("/covers/{cover_id}")).await
    }

    pub async fn get_creator(&self, id: Uuid) -> Response {
        self.get_raw(&format!("/creators/{id}")).await
    }

    pub async fn delete_creator(&self, id: Uuid) -> Response {
        self.delete_authed(&format!("/creators/{id}")).await
    }

    pub async fn post_feedback(&self, body: serde_json::Value) -> Response {
        self.json_authed(Method::POST, "/feedbacks", body).await
    }

    pub async fn get_feedbacks(&self, path: &str) -> Response {
        self.get_as(path, &[Role::Moderator]).await
    }

    pub async fn get_feedback(&self, id: Uuid) -> Response {
        self.get_as(&format!("/feedbacks/{id}"), &[Role::Moderator])
            .await
    }

    pub async fn put_feedback_status(&self, id: Uuid, status: &str) -> StatusCode {
        self.json_authed(
            Method::PUT,
            &format!("/feedbacks/{id}/status"),
            serde_json::json!({ "status": status }),
        )
        .await
        .status()
    }

    pub async fn delete_chapter(&self, id: Uuid) -> Response {
        self.delete_authed(&format!("/chapters/{id}")).await
    }

    pub async fn get_books_page(&self, path: &str) -> RespWrapper<Vec<BookQuery>, String> {
        self.get_ok_json(path).await
    }

    pub async fn get_labels(&self, path: &str) -> Vec<Label> {
        self.get_ok_json::<RespWrapper<Vec<Label>>>(path).await.data
    }

    pub async fn get_chapters(&self, path: &str) -> RespWrapper<Vec<Chapter>> {
        self.get_ok_json(path).await
    }

    pub async fn post_chapter(&self, book_id: Uuid, number: f32) -> Response {
        self.json_authed(
            Method::POST,
            &format!("/books/{book_id}/chapters"),
            serde_json::json!({ "number": number }),
        )
        .await
    }

    pub async fn post_release(&self, chapter_id: Uuid, language_id: Uuid) -> Response {
        self.json_authed(
            Method::POST,
            &format!("/chapters/{chapter_id}/releases"),
            serde_json::json!({ "languageId": language_id }),
        )
        .await
    }

    pub async fn get_releases(&self, chapter_id: Uuid) -> Response {
        self.get_raw(&format!("/chapters/{chapter_id}/releases"))
            .await
    }

    pub async fn get_release(&self, id: Uuid) -> Response {
        self.get_raw(&format!("/releases/{id}")).await
    }

    pub async fn post_upload_pages(&self, release_id: Uuid, parts: &[&[u8]]) -> Response {
        self.multipart_authed(&format!("/releases/{release_id}/upload"), parts)
            .await
    }

    pub async fn post_commit(&self, release_id: Uuid, page_order: &[Uuid]) -> Response {
        self.json_authed(
            Method::POST,
            &format!("/releases/{release_id}/commit"),
            serde_json::json!({ "pageOrder": page_order }),
        )
        .await
    }

    pub async fn get_pages(&self, release_id: Uuid) -> Response {
        self.get_raw(&format!("/releases/{release_id}/pages")).await
    }

    pub async fn get_staged(&self, release_id: Uuid) -> Response {
        self.get_raw(&format!("/releases/{release_id}/pages?status=Staged"))
            .await
    }

    pub async fn get_page_image(&self, release_id: Uuid, page_id: Uuid) -> Response {
        self.get_raw(&format!("/releases/{release_id}/pages/{page_id}/image"))
            .await
    }

    pub async fn get_page(&self, release_id: Uuid, page_number: i32) -> Response {
        self.get_raw(&format!("/releases/{release_id}/pages/{page_number}"))
            .await
    }

    pub async fn delete_book(&self, id: Uuid) -> Response {
        self.delete_authed(&format!("/books/{id}")).await
    }

    pub async fn delete_release(&self, id: Uuid) -> Response {
        self.delete_authed(&format!("/releases/{id}")).await
    }
}
