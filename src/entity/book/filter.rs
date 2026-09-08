use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    entity::{
        BookKind, BookLabelIds, BookStatus, BookVisibility, Bounded, BoundedVec, Limit,
        PublicationDemographic, Range, RangeBound, ShortHexHash, SortOrder, Timestamp,
    },
    error::EntityError,
};

// Open lookup sets: an anti-amplification ceiling clearing ISO 639-1, not a schema fact.
pub const MAX_FILTER_LOOKUP_VALUES: usize = 200;

// Closed vocabularies: one value per enum variant. Adding or removing a variant of `BookKind`,
// `BookStatus`, or `PublicationDemographic` means updating the matching constant here.
pub const MAX_BOOK_KINDS: usize = 3;
pub const MAX_BOOK_STATUSES: usize = 4;
pub const MAX_PUBLICATION_DEMOGRAPHICS: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
pub enum LabelsMode {
    And,
    Or,
}

#[derive(Debug, Serialize)]
pub struct LabelFilter {
    pub included: BookLabelIds,
    pub mode: LabelsMode,
    pub excluded: BookLabelIds,
}

pub struct PublicationYearBound;

impl RangeBound for PublicationYearBound {
    const UPPER_INCLUSIVE: bool = true;
    const NAME: &'static str = "publication year";
}

pub struct CreatedAtBound;

impl RangeBound for CreatedAtBound {
    const UPPER_INCLUSIVE: bool = false;
    const NAME: &'static str = "created at";
}

pub type PublicationYearRange = Range<i16, PublicationYearBound>;
pub type CreatedAtRange = Range<Timestamp, CreatedAtBound>;

pub struct BookKindsBound;

impl Bounded for BookKindsBound {
    const MAX: usize = MAX_BOOK_KINDS;
    const NAME: &'static str = "book kinds";
}

pub struct BookStatusesBound;

impl Bounded for BookStatusesBound {
    const MAX: usize = MAX_BOOK_STATUSES;
    const NAME: &'static str = "book statuses";
}

pub struct PublicationDemographicsBound;

impl Bounded for PublicationDemographicsBound {
    const MAX: usize = MAX_PUBLICATION_DEMOGRAPHICS;
    const NAME: &'static str = "publication demographics";
}

pub struct FilterLookupBound;

impl Bounded for FilterLookupBound {
    const MAX: usize = MAX_FILTER_LOOKUP_VALUES;
    const NAME: &'static str = "filter values";
}

pub type BookKinds = BoundedVec<BookKind, BookKindsBound>;
pub type BookStatuses = BoundedVec<BookStatus, BookStatusesBound>;
pub type PublicationDemographics = BoundedVec<PublicationDemographic, PublicationDemographicsBound>;
pub type FilterLookupIds = BoundedVec<Uuid, FilterLookupBound>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum BookSortField {
    CreatedAt,
    Name,
    PublicationYear,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(tag = "field", content = "value")]
pub enum BookSort {
    CreatedAt(Timestamp),
    Name(String),
    PublicationYear(i16),
}

impl BookSort {
    pub fn field(&self) -> BookSortField {
        match self {
            Self::CreatedAt(_) => BookSortField::CreatedAt,
            Self::Name(_) => BookSortField::Name,
            Self::PublicationYear(_) => BookSortField::PublicationYear,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BookCursor {
    pub id: Uuid,
    pub sort: BookSort,
    pub selection_hash: ShortHexHash,
}

#[derive(Debug, Serialize)]
pub struct BookSelection {
    pub visibility: BookVisibility,
    pub sort_field: BookSortField,
    pub order: SortOrder,
    pub labels: LabelFilter,
    pub kinds: BookKinds,
    pub statuses: BookStatuses,
    pub content_rating_ids: FilterLookupIds,
    pub publication_language_ids: FilterLookupIds,
    pub publication_demographics: PublicationDemographics,
    pub available_translated_language_ids: FilterLookupIds,
    pub publication_year: PublicationYearRange,
    pub created_at: CreatedAtRange,
}

#[derive(Debug)]
pub struct BookFilter {
    pub limit: Limit,
    pub cursor: Option<BookCursor>,
    pub selection: BookSelection,
    pub selection_hash: ShortHexHash,
}

impl BookFilter {
    pub fn ensure_cursor_fits(&self) -> Result<(), EntityError> {
        let Some(cursor) = &self.cursor else {
            return Ok(());
        };

        if cursor.sort.field() != self.selection.sort_field
            || cursor.selection_hash != self.selection_hash
        {
            return Err(EntityError::CursorSelectionMismatch);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use time::OffsetDateTime;
    use uuid::Uuid;

    use crate::{
        entity::{
            BookCursor, BookFilter, BookKind, BookKinds, BookLabelIds, BookSelection, BookSort,
            BookSortField, BookVisibility, CreatedAtRange, LabelFilter, LabelsMode, Limit,
            PublicationYearRange, ShortHexHash, SortOrder, Timestamp,
        },
        hasher::{Hasher, tests::stub},
    };

    fn filter(selection: BookSelection) -> BookFilter {
        BookFilter {
            limit: Limit::DEFAULT,
            cursor: None,
            selection_hash: hash(&selection),
            selection,
        }
    }

    fn filter_paged(selection: BookSelection, cursor: BookCursor) -> BookFilter {
        BookFilter {
            limit: Limit::DEFAULT,
            cursor: Some(cursor),
            selection_hash: hash(&selection),
            selection,
        }
    }

    fn cursor_for(selection: &BookSelection) -> BookCursor {
        BookCursor {
            id: Uuid::now_v7(),
            sort: BookSort::CreatedAt(Timestamp::from(OffsetDateTime::UNIX_EPOCH)),
            selection_hash: hash(selection),
        }
    }

    fn hash(selection: &BookSelection) -> ShortHexHash {
        Hasher::compute_hex_hash(selection).unwrap()
    }

    #[test]
    fn a_cursor_from_the_same_filter_is_accepted() {
        let selection = stub();
        let filter = filter_paged(stub(), cursor_for(&selection));

        assert!(filter.ensure_cursor_fits().is_ok());
    }

    #[test]
    fn no_cursor_is_always_accepted() {
        let filter = filter(stub());

        assert!(filter.ensure_cursor_fits().is_ok());
    }

    #[test]
    fn a_cursor_is_rejected_once_any_facet_moves() {
        let cursor = cursor_for(&stub());

        let changed = [
            BookSelection {
                labels: LabelFilter {
                    included: BookLabelIds::try_from(vec![Uuid::now_v7()]).unwrap(),
                    mode: LabelsMode::And,
                    excluded: BookLabelIds::try_from(vec![]).unwrap(),
                },
                ..stub()
            },
            BookSelection {
                kinds: BookKinds::try_from(vec![BookKind::Manhwa]).unwrap(),
                ..stub()
            },
            BookSelection {
                visibility: BookVisibility::Hidden,
                ..stub()
            },
            BookSelection {
                publication_year: PublicationYearRange::try_new(Some(2010), None).unwrap(),
                ..stub()
            },
            BookSelection {
                created_at: CreatedAtRange::try_new(
                    Some(Timestamp::from(OffsetDateTime::UNIX_EPOCH)),
                    None,
                )
                .unwrap(),
                ..stub()
            },
            BookSelection {
                order: SortOrder::Asc,
                ..stub()
            },
        ];

        for selection in changed {
            let filter = filter_paged(selection, cursor.clone());

            assert!(
                filter.ensure_cursor_fits().is_err(),
                "{filter:?} must reject a cursor minted before the change"
            );
        }
    }

    #[test]
    fn a_cursor_for_another_sort_field_is_rejected() {
        let selection = BookSelection {
            sort_field: BookSortField::Name,
            ..stub()
        };
        let cursor = cursor_for(&selection);
        let filter = filter_paged(selection, cursor);

        assert!(filter.ensure_cursor_fits().is_err());
    }

    #[test]
    fn page_size_is_outside_the_selection_hash() {
        let minted_under = stub();
        let cursor = cursor_for(&minted_under);
        let resized = BookFilter {
            limit: Limit::try_from(50).unwrap(),
            ..filter_paged(stub(), cursor)
        };

        assert!(
            resized.ensure_cursor_fits().is_ok(),
            "changing page size mid-paging is legitimate"
        );
    }

    #[test]
    fn a_sort_value_belongs_to_one_field() {
        let cases = [
            (
                BookSort::CreatedAt(Timestamp::from(OffsetDateTime::UNIX_EPOCH)),
                BookSortField::CreatedAt,
            ),
            (
                BookSort::Name("Solo Leveling".to_owned()),
                BookSortField::Name,
            ),
            (
                BookSort::PublicationYear(2016),
                BookSortField::PublicationYear,
            ),
        ];

        for (value, field) in cases {
            assert_eq!(value.field(), field, "{value:?}");
        }
    }

    #[test]
    fn a_sort_value_belongs_to_no_other_field() {
        let cases = [
            (
                BookSort::CreatedAt(Timestamp::from(OffsetDateTime::UNIX_EPOCH)),
                BookSortField::Name,
            ),
            (
                BookSort::CreatedAt(Timestamp::from(OffsetDateTime::UNIX_EPOCH)),
                BookSortField::PublicationYear,
            ),
            (
                BookSort::Name("Solo Leveling".to_owned()),
                BookSortField::CreatedAt,
            ),
            (
                BookSort::Name("Solo Leveling".to_owned()),
                BookSortField::PublicationYear,
            ),
            (BookSort::PublicationYear(2016), BookSortField::CreatedAt),
            (BookSort::PublicationYear(2016), BookSortField::Name),
        ];

        for (value, other) in cases {
            assert_ne!(
                value.field(),
                other,
                "{value:?} must not pass a cursor check for {other:?}"
            );
        }
    }
}
