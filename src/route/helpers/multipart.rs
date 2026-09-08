use axum::extract::multipart::Field;
use bytes::{Bytes, BytesMut};
use uuid::Uuid;

use crate::{
    entity::{FileName, Image, ImageContent, ImageExtension},
    error::{AppError, EntityError, RouteError},
};

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

pub async fn collect_image_part(mut field: Field<'_>) -> Result<Image, AppError> {
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
