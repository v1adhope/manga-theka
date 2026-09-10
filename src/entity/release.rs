use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    entity::{
        BookVisibility, Bounded, BoundedVec, Entity, ImageExtension, Images, Language,
        MAX_PARTS_PER_REQUEST, Ordinal, ResourceUrl, UserClaims,
    },
    error::EntityError,
};

pub const MAX_COMMITTED_PAGES: usize = 200;
pub const MAX_RELEASE_ROWS: usize = 400;

#[derive(Debug)]
pub struct PageUrl {
    pub release_id: Uuid,
    pub page_number: Ordinal,
}

pub struct PageOrderBound;

impl Bounded for PageOrderBound {
    const MAX: usize = MAX_COMMITTED_PAGES;
    const NAME: &'static str = "page order";
}

#[derive(Debug)]
pub struct PageOrder(BoundedVec<Uuid, PageOrderBound>);

impl TryFrom<Vec<Uuid>> for PageOrder {
    type Error = EntityError;

    fn try_from(ids: Vec<Uuid>) -> Result<Self, Self::Error> {
        if ids.is_empty() {
            return Err(EntityError::PageOrderIsEmpty);
        }

        let ids = BoundedVec::try_from(ids)?;

        let mut seen = HashSet::with_capacity(ids.len());
        for id in ids.as_slice() {
            if !seen.insert(id) {
                return Err(EntityError::PageOrderHasDuplicates(*id));
            }
        }

        Ok(Self(ids))
    }
}

impl PageOrder {
    pub fn as_slice(&self) -> &[Uuid] {
        self.0.as_slice()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Debug)]
pub struct ChapterRelease {
    pub id: Uuid,
    pub chapter_id: Uuid,
    pub language_id: Uuid,
    pub created_by: Uuid,
}

impl Entity for ChapterRelease {
    const NAME: &'static str = "Chapter release";
}

#[derive(Debug)]
pub struct ReleaseAccess {
    pub visibility: BookVisibility,
    pub created_by: Uuid,
}

impl ReleaseAccess {
    pub fn ensure_mutable(&self, claims: &UserClaims) -> Result<(), EntityError> {
        if claims.id != self.created_by && !claims.can_moderate() {
            return Err(EntityError::Forbidden);
        }

        self.visibility.ensure_content_writable()
    }
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
    pub created_by: Uuid,
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
#[serde(tag = "status", rename_all_fields = "camelCase")]
pub enum ChapterPageQuery {
    Committed {
        id: Uuid,
        page_number: Ordinal,
        extension: ImageExtension,
        url: ResourceUrl,
    },
    Staged {
        id: Uuid,
        extension: ImageExtension,
    },
}

#[derive(Debug, Deserialize)]
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

    use crate::{
        entity::{
            BookVisibility, ChapterRelease, MAX_COMMITTED_PAGES, MAX_PARTS_PER_REQUEST,
            MAX_RELEASE_ROWS, PageOrder, ReleaseAccess, Role, UserClaims,
        },
        error::EntityError,
    };

    fn claims(id: Uuid, roles: &[Role]) -> UserClaims {
        UserClaims {
            id,
            sid: Uuid::now_v7(),
            roles: roles.to_vec(),
        }
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

    #[test]
    fn the_creator_may_mutate_a_listed_release() {
        let owner = Uuid::now_v7();
        let access = ReleaseAccess {
            visibility: BookVisibility::Listed,
            created_by: owner,
        };

        let res = access.ensure_mutable(&claims(owner, &[Role::Uploader]));

        assert!(res.is_ok());
    }

    #[test]
    fn a_moderator_may_mutate_a_release_they_did_not_create() {
        let access = ReleaseAccess {
            visibility: BookVisibility::Listed,
            created_by: Uuid::now_v7(),
        };

        let res = access.ensure_mutable(&claims(Uuid::now_v7(), &[Role::Moderator]));

        assert!(res.is_ok());
    }

    #[test]
    fn a_non_creator_without_moderation_is_forbidden() {
        let access = ReleaseAccess {
            visibility: BookVisibility::Listed,
            created_by: Uuid::now_v7(),
        };

        let res = access.ensure_mutable(&claims(Uuid::now_v7(), &[Role::Uploader]));

        assert!(matches!(res, Err(EntityError::Forbidden)));
    }

    #[test]
    fn forbidden_outranks_a_non_listed_book() {
        let access = ReleaseAccess {
            visibility: BookVisibility::Draft,
            created_by: Uuid::now_v7(),
        };

        let res = access.ensure_mutable(&claims(Uuid::now_v7(), &[Role::Uploader]));

        assert!(matches!(res, Err(EntityError::Forbidden)));
    }

    #[test]
    fn the_creator_still_cannot_mutate_a_release_whose_book_is_not_listed() {
        let owner = Uuid::now_v7();
        let access = ReleaseAccess {
            visibility: BookVisibility::Hidden,
            created_by: owner,
        };

        let res = access.ensure_mutable(&claims(owner, &[Role::Uploader]));

        assert!(matches!(res, Err(EntityError::BookNotWritable(_))));
    }
}
