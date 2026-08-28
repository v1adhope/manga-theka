use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    entity::{Entity, ImageExtension, Images, Language, MAX_PARTS_PER_REQUEST},
    error::EntityError,
};

pub const MAX_COMMITTED_PAGES: usize = 200;
pub const MAX_RELEASE_ROWS: usize = 400;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PageUrl(String);

impl From<(Uuid, Ordinal)> for PageUrl {
    fn from((release_id, page_number): (Uuid, Ordinal)) -> Self {
        Self(format!(
            "/releases/{release_id}/pages/{}",
            page_number.as_i32()
        ))
    }
}

impl AsRef<str> for PageUrl {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "i32")]
pub struct Ordinal(i32);

impl TryFrom<i32> for Ordinal {
    type Error = EntityError;

    fn try_from(n: i32) -> Result<Self, Self::Error> {
        const MIN: i32 = 1;

        if n < MIN {
            return Err(EntityError::OrdinalOutOfRange(n, MIN));
        }

        Ok(Self(n))
    }
}

impl Ordinal {
    pub fn as_i32(self) -> i32 {
        self.0
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
    pub page_count: i64,
    pub version: Ordinal,
}

pub struct ChapterPage;

impl Entity for ChapterPage {
    const NAME: &'static str = "Chapter page";
}

#[derive(Debug)]
pub struct ChapterPages {
    pub release_id: Uuid,
    pub images: Images,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ChapterPageQuery {
    Committed {
        id: Uuid,
        page_number: Ordinal,
        extension: ImageExtension,
        url: PageUrl,
    },
    Staged {
        id: Uuid,
        extension: ImageExtension,
    },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PageStatus {
    Staged,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterPageParams {
    pub status: Option<PageStatus>,
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::entity::{
        ChapterRelease, MAX_COMMITTED_PAGES, MAX_PARTS_PER_REQUEST, MAX_RELEASE_ROWS, Ordinal,
        PageOrder, PageUrl,
    };

    #[test]
    fn ordinal_at_the_first_position_is_valid() {
        let ordinal = Ordinal::try_from(1).unwrap();

        assert_eq!(ordinal.as_i32(), 1);
    }

    #[test]
    fn non_positive_ordinal_is_rejected() {
        for n in [0, -1] {
            assert!(Ordinal::try_from(n).is_err());
        }
    }

    #[test]
    fn deserialized_non_positive_ordinal_is_rejected() {
        for n in ["0", "-1"] {
            assert!(serde_json::from_str::<Ordinal>(n).is_err());
        }
    }

    #[test]
    fn ordinal_serializes_as_a_bare_number() {
        let ordinal = Ordinal::try_from(1).unwrap();

        assert_eq!(serde_json::to_string(&ordinal).unwrap(), "1");
    }

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
        let url = PageUrl::from((Uuid::from_u128(1), Ordinal::try_from(4).unwrap()));

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
