use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entity::{Entity, Image, ImageExtension, ResourceUrl};

#[derive(Debug)]
pub struct BookCover {
    pub book_id: Uuid,
    pub image: Image,
}

impl Entity for BookCover {
    const NAME: &'static str = "Book cover";
}

#[derive(Debug)]
pub struct CoverUrl {
    pub cover_id: Uuid,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BookCoverQuery {
    pub id: Uuid,
    pub book_id: Uuid,
    pub extension: ImageExtension,
    pub is_main: bool,
    pub url: ResourceUrl,
}
