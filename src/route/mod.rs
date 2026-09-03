mod authz;
mod book;
mod chapter;
mod content_rating;
mod creator;
mod feedback;
mod healthz;
mod label;
mod language;
mod release;

pub use book::*;
pub use chapter::*;
pub use content_rating::*;
pub use creator::*;
pub use feedback::*;
pub use healthz::*;
pub use label::*;
pub use language::*;
pub use release::*;

use axum::{Json, extract::multipart::Field, http::StatusCode};
use bytes::{Bytes, BytesMut};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::{
    entity::{FileName, Filter, Image, ImageContent, ImageExtension, Limit, SortOrder},
    error::{AppError, EntityError, RouteError},
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreResp {
    pub id: Uuid,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginationQuery {
    pub after: Option<Uuid>,
    pub limit: Option<i64>,
}

impl TryFrom<PaginationQuery> for Filter {
    type Error = EntityError;

    fn try_from(q: PaginationQuery) -> Result<Self, Self::Error> {
        Ok(Self {
            after: q.after,
            limit: q
                .limit
                .map(Limit::try_from)
                .transpose()?
                .unwrap_or_default(),
            sort_order: SortOrder::default(),
        })
    }
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

struct ImageStage {
    id: Uuid,
    content: Bytes,
    file_name: Option<String>,
}

impl TryFrom<ImageStage> for Image {
    type Error = EntityError;

    fn try_from(stage: ImageStage) -> Result<Self, Self::Error> {
        let content = ImageContent::try_from(stage.content)?;
        let extension = ImageExtension::try_from(content.as_ref())?;
        let file_name = FileName::from_client_provided(stage.file_name)?;

        Ok(Self {
            id: stage.id,
            extension,
            content,
            file_name,
        })
    }
}

pub(crate) async fn collect_image_part(mut field: Field<'_>) -> Result<Image, AppError> {
    let file_name = field.file_name().map(str::to_owned);
    let mut content = BytesMut::new();

    while let Some(chunk) = field.chunk().await.map_err(RouteError::from)? {
        ImageContent::ensure_incoming_capacity(content.len(), chunk.len())?;

        content.extend_from_slice(&chunk);
    }

    let image = ImageStage {
        id: Uuid::now_v7(),
        content: content.freeze(),
        file_name,
    }
    .try_into()?;

    Ok(image)
}
