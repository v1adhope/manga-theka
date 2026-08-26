use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    entity::{DEFAULT_IMAGE_MAX_BYTES, Entity, Image, ImageExtension, Language},
    error::EntityError,
};

pub const MAX_PARTS_PER_REQUEST: usize = 10;
pub const MAX_COMMITTED_PAGES: usize = 200;
pub const MAX_RELEASE_ROWS: usize = 400;
pub const UPLOAD_MAX_BYTES: usize =
    MAX_PARTS_PER_REQUEST * (DEFAULT_IMAGE_MAX_BYTES + PART_HEADROOM_BYTES);

const PART_HEADROOM_BYTES: usize = 1024;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PageUrl(String);

impl From<(Uuid, i16)> for PageUrl {
    fn from((release_id, page_number): (Uuid, i16)) -> Self {
        Self(format!("/releases/{release_id}/pages/{page_number}"))
    }
}

impl AsRef<str> for PageUrl {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug)]
pub struct PageOrder(Vec<Uuid>);

impl TryFrom<Vec<Uuid>> for PageOrder {
    type Error = EntityError;

    fn try_from(ids: Vec<Uuid>) -> Result<Self, Self::Error> {
        if ids.is_empty() {
            return Err(EntityError::PageOrderIsEmpty);
        }
        if ids.len() > MAX_COMMITTED_PAGES {
            return Err(EntityError::PageOrderExceedsLimit(
                ids.len(),
                MAX_COMMITTED_PAGES,
            ));
        }

        let mut seen = HashSet::with_capacity(ids.len());
        for id in &ids {
            if !seen.insert(id) {
                return Err(EntityError::PageOrderHasDuplicates(*id));
            }
        }

        Ok(Self(ids))
    }
}

impl PageOrder {
    pub fn as_slice(&self) -> &[Uuid] {
        &self.0
    }
}

#[derive(Debug)]
pub struct ChapterRelease {
    pub id: Uuid,
    pub chapter_id: Uuid,
    pub language_id: Uuid,
}

impl Entity for ChapterRelease {
    const NAME: &'static str = "Chapter release";
}

impl ChapterRelease {
    pub fn ensure_row_capacity(existing: usize, incoming: usize) -> Result<(), EntityError> {
        let total = existing + incoming;
        if total > MAX_RELEASE_ROWS {
            return Err(EntityError::ReleaseRowsExceedLimit(total, MAX_RELEASE_ROWS));
        }

        Ok(())
    }

    pub fn ensure_part_capacity(count: usize) -> Result<(), EntityError> {
        if count > MAX_PARTS_PER_REQUEST {
            return Err(EntityError::UploadPartsExceedLimit(
                count,
                MAX_PARTS_PER_REQUEST,
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterReleaseQuery {
    pub id: Uuid,
    pub chapter_id: Uuid,
    pub language: Language,
    pub version: u16,
}

pub struct ChapterPage;

impl Entity for ChapterPage {
    const NAME: &'static str = "Chapter page";
}

#[derive(Debug)]
pub struct ChapterPages {
    pub release_id: Uuid,
    pub images: Vec<Image>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterPageQuery {
    pub id: Uuid,
    pub page_number: i16,
    pub extension: ImageExtension,
    pub url: PageUrl,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StagedPageQuery {
    pub id: Uuid,
    pub extension: ImageExtension,
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::entity::{
        ChapterRelease, MAX_COMMITTED_PAGES, MAX_PARTS_PER_REQUEST, MAX_RELEASE_ROWS, PageOrder,
        PageUrl,
    };

    #[test]
    fn row_capacity_at_the_ceiling_is_valid() {
        let res = ChapterRelease::ensure_row_capacity(MAX_RELEASE_ROWS - 1, 1);
        assert!(res.is_ok());
    }

    #[test]
    fn row_capacity_over_the_ceiling_is_rejected() {
        let res = ChapterRelease::ensure_row_capacity(MAX_RELEASE_ROWS, 1);
        assert!(res.is_err());
    }

    #[test]
    fn part_capacity_at_the_ceiling_is_valid() {
        let res = ChapterRelease::ensure_part_capacity(MAX_PARTS_PER_REQUEST);
        assert!(res.is_ok());
    }

    #[test]
    fn part_capacity_over_the_ceiling_is_rejected() {
        let res = ChapterRelease::ensure_part_capacity(MAX_PARTS_PER_REQUEST + 1);
        assert!(res.is_err());
    }

    #[test]
    fn page_url_points_at_the_position_addressed_reader_route() {
        let url = PageUrl::from((Uuid::from_u128(1), 4));

        assert_eq!(
            url.as_ref(),
            "/releases/00000000-0000-0000-0000-000000000001/pages/4"
        );
    }

    #[test]
    fn page_order_keeps_the_declared_sequence() {
        let ids = vec![Uuid::from_u128(2), Uuid::from_u128(1)];
        let order = PageOrder::try_from(ids.clone()).unwrap();

        assert_eq!(order.as_slice(), ids.as_slice());
    }

    #[test]
    fn empty_page_order_is_rejected() {
        let res = PageOrder::try_from(Vec::new());
        assert!(res.is_err());
    }

    #[test]
    fn page_order_with_duplicates_is_rejected() {
        let id = Uuid::from_u128(1);
        let res = PageOrder::try_from(vec![id, id]);
        assert!(res.is_err());
    }

    #[test]
    fn page_order_at_the_committed_ceiling_is_valid() {
        let ids: Vec<Uuid> = (1..=MAX_COMMITTED_PAGES as u128)
            .map(Uuid::from_u128)
            .collect();
        let res = PageOrder::try_from(ids);
        assert!(res.is_ok());
    }

    #[test]
    fn page_order_over_the_committed_ceiling_is_rejected() {
        let ids: Vec<Uuid> = (1..=MAX_COMMITTED_PAGES as u128 + 1)
            .map(Uuid::from_u128)
            .collect();
        let res = PageOrder::try_from(ids);
        assert!(res.is_err());
    }
}
