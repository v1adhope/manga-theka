use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::response::Response;
use axum_test::multipart::{MultipartForm, Part};
use http_body_util::BodyExt;
use manga_theka::entity::{BookQuery, Chapter, Label, Role};
use serde::Deserialize;
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
    assert!(
        !resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .is_empty(),
        "error response must carry a body"
    );
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
    pub async fn put_visibility(&self, id: Uuid, visibility: &str, note: Option<&str>) -> Response {
        let body = match note {
            Some(note) => serde_json::json!({ "visibility": visibility, "note": note }),
            None => serde_json::json!({ "visibility": visibility }),
        }
        .to_string();
        let req = Request::put(format!("/books/{id}/visibility"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .unwrap();

        self.send(req).await
    }

    pub async fn get_books(&self, path: &str) -> Response {
        let req = Request::get(path).body(Body::empty()).unwrap();

        self.send_raw(req).await
    }

    pub async fn get_body(&self, path: &str) -> (StatusCode, String) {
        let req = Request::get(path).body(Body::empty()).unwrap();
        let resp = self.send_raw(req).await;
        let status = resp.status();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();

        (status, String::from_utf8_lossy(&bytes).into_owned())
    }

    pub async fn get_as_moderator(&self, path: &str) -> Response {
        let req = Request::get(path)
            .header(
                header::AUTHORIZATION,
                self.bearer(Uuid::now_v7(), Uuid::now_v7(), &[Role::Moderator]),
            )
            .body(Body::empty())
            .unwrap();

        self.send_raw(req).await
    }

    pub async fn post_cover(&self, book_id: Uuid, image: &[u8]) -> Response {
        let form = Self::multipart_body(&[image]);
        let req = Request::post(format!("/books/{book_id}/covers"))
            .header(header::CONTENT_TYPE, form.content_type())
            .body(Body::from(form))
            .unwrap();

        self.send(req).await
    }

    pub async fn get_covers(&self, book_id: Uuid) -> Response {
        let req = Request::get(format!("/books/{book_id}/covers"))
            .body(Body::empty())
            .unwrap();

        self.send_raw(req).await
    }

    pub async fn get_cover_image(&self, cover_id: Uuid) -> Response {
        let req = Request::get(format!("/covers/{cover_id}/image"))
            .body(Body::empty())
            .unwrap();

        self.send_raw(req).await
    }

    pub async fn put_main_cover(&self, book_id: Uuid, cover_id: Uuid) -> StatusCode {
        let body = serde_json::json!({ "coverId": cover_id }).to_string();
        let req = Request::put(format!("/books/{book_id}/main-cover"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .unwrap();

        self.send(req).await.status()
    }

    pub async fn delete_cover(&self, cover_id: Uuid) -> Response {
        let req = Request::delete(format!("/covers/{cover_id}"))
            .body(Body::empty())
            .unwrap();

        self.send(req).await
    }

    pub async fn get_creator(&self, id: Uuid) -> Response {
        let req = Request::get(format!("/creators/{id}"))
            .body(Body::empty())
            .unwrap();

        self.send_raw(req).await
    }

    pub async fn delete_creator(&self, id: Uuid) -> Response {
        let req = Request::delete(format!("/creators/{id}"))
            .body(Body::empty())
            .unwrap();

        self.send(req).await
    }

    pub async fn post_feedback(&self, body: serde_json::Value) -> Response {
        let req = Request::post("/feedbacks")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();

        self.send(req).await
    }

    pub async fn get_feedbacks(&self, path: &str) -> Response {
        let req = Request::get(path)
            .header(
                header::AUTHORIZATION,
                self.bearer(Uuid::now_v7(), Uuid::now_v7(), &[Role::Moderator]),
            )
            .body(Body::empty())
            .unwrap();

        self.send_raw(req).await
    }

    pub async fn get_feedback(&self, id: Uuid) -> Response {
        let req = Request::get(format!("/feedbacks/{id}"))
            .header(
                header::AUTHORIZATION,
                self.bearer(Uuid::now_v7(), Uuid::now_v7(), &[Role::Moderator]),
            )
            .body(Body::empty())
            .unwrap();

        self.send_raw(req).await
    }

    pub async fn put_feedback_status(&self, id: Uuid, status: &str) -> StatusCode {
        let body = serde_json::json!({ "status": status }).to_string();
        let req = Request::put(format!("/feedbacks/{id}/status"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .unwrap();

        self.send(req).await.status()
    }

    pub async fn delete_chapter(&self, id: Uuid) -> Response {
        let req = Request::delete(format!("/chapters/{id}"))
            .body(Body::empty())
            .unwrap();

        self.send(req).await
    }

    pub async fn get_books_page(&self, path: &str) -> RespWrapper<Vec<BookQuery>, String> {
        let req = Request::get(path).body(Body::empty()).unwrap();
        let resp = self.send_raw(req).await;
        assert_eq!(resp.status(), StatusCode::OK, "{path}");

        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).expect("failed to parse book page")
    }

    pub async fn get_labels(&self, path: &str) -> Vec<Label> {
        let req = Request::get(path).body(Body::empty()).unwrap();
        let resp = self.send_raw(req).await;
        assert_eq!(resp.status(), StatusCode::OK, "{path}");

        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let wrapper: RespWrapper<Vec<Label>> = serde_json::from_slice(&bytes).unwrap();

        wrapper.data
    }

    pub async fn get_chapters(&self, path: String) -> RespWrapper<Vec<Chapter>> {
        let req = Request::get(path).body(Body::empty()).unwrap();
        let resp = self.send_raw(req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).expect("failed to parse chapter page")
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

    pub async fn post_chapter(&self, book_id: Uuid, number: f32) -> Response {
        let body = serde_json::json!({ "number": number }).to_string();
        let req = Request::post(format!("/books/{book_id}/chapters"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .unwrap();

        self.send(req).await
    }

    pub async fn post_release(&self, chapter_id: Uuid, language_id: Uuid) -> Response {
        let body = serde_json::json!({ "languageId": language_id }).to_string();
        let req = Request::post(format!("/chapters/{chapter_id}/releases"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .unwrap();

        self.send(req).await
    }

    pub async fn get_releases(&self, chapter_id: Uuid) -> Response {
        let req = Request::get(format!("/chapters/{chapter_id}/releases"))
            .body(Body::empty())
            .unwrap();

        self.send_raw(req).await
    }

    pub async fn get_release(&self, id: Uuid) -> Response {
        let req = Request::get(format!("/releases/{id}"))
            .body(Body::empty())
            .unwrap();

        self.send_raw(req).await
    }

    pub async fn post_upload_pages(&self, release_id: Uuid, parts: &[&[u8]]) -> Response {
        let form = Self::multipart_body(parts);
        let req = Request::post(format!("/releases/{release_id}/upload"))
            .header(header::CONTENT_TYPE, form.content_type())
            .body(Body::from(form))
            .unwrap();

        self.send(req).await
    }

    pub async fn post_commit(&self, release_id: Uuid, page_order: &[Uuid]) -> Response {
        let body = serde_json::json!({ "pageOrder": page_order }).to_string();
        let req = Request::post(format!("/releases/{release_id}/commit"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .unwrap();

        self.send(req).await
    }

    pub async fn get_pages(&self, release_id: Uuid) -> Response {
        let req = Request::get(format!("/releases/{release_id}/pages"))
            .body(Body::empty())
            .unwrap();

        self.send_raw(req).await
    }

    pub async fn get_staged(&self, release_id: Uuid) -> Response {
        let req = Request::get(format!("/releases/{release_id}/pages?status=Staged"))
            .body(Body::empty())
            .unwrap();

        self.send_raw(req).await
    }

    pub async fn get_page_image(&self, release_id: Uuid, page_id: Uuid) -> Response {
        let req = Request::get(format!("/releases/{release_id}/pages/{page_id}/image"))
            .body(Body::empty())
            .unwrap();

        self.send_raw(req).await
    }

    pub async fn get_page(&self, release_id: Uuid, page_number: i32) -> Response {
        let req = Request::get(format!("/releases/{release_id}/pages/{page_number}"))
            .body(Body::empty())
            .unwrap();

        self.send_raw(req).await
    }

    pub async fn delete_book(&self, id: Uuid) -> Response {
        let req = Request::delete(format!("/books/{id}"))
            .body(Body::empty())
            .unwrap();

        self.send(req).await
    }

    pub async fn delete_release(&self, id: Uuid) -> Response {
        let req = Request::delete(format!("/releases/{id}"))
            .body(Body::empty())
            .unwrap();

        self.send(req).await
    }
}
