use serde::Deserialize;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    entity::{
        BookCursor, BookKind, BookLabelIds, BookSortField, BookStatus, BookVisibility, Bounded,
        BoundedVec, CanonicalFilter, Filter, PublicationDemographic, Range, RangeBound,
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

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub enum LabelsMode {
    And,
    Or,
}

impl AsRef<str> for LabelsMode {
    fn as_ref(&self) -> &str {
        match self {
            Self::And => "And",
            Self::Or => "Or",
        }
    }
}

#[derive(Debug)]
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
pub type CreatedAtRange = Range<OffsetDateTime, CreatedAtBound>;

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

#[derive(Debug)]
pub struct BookFilter {
    pub page: Filter,
    pub visibility: Option<BookVisibility>,
    pub sort: Option<BookSortField>,
    pub cursor: Option<BookCursor>,
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

impl BookFilter {
    pub fn effective_visibility(&self) -> BookVisibility {
        self.visibility.unwrap_or(BookVisibility::Listed)
    }

    pub fn effective_sort(&self) -> BookSortField {
        self.sort.unwrap_or(BookSortField::CreatedAt)
    }

    /// Every parameter that changes which rows match, and in what order, rendered in one fixed
    /// order. `limit` is deliberately absent: changing page size mid-paging is legitimate.
    pub fn canonical(&self) -> CanonicalFilter {
        fn uuids(ids: &FilterLookupIds) -> Vec<String> {
            ids.as_slice().iter().map(Uuid::to_string).collect()
        }

        CanonicalFilter::new()
            .sort("sort", self.effective_sort())
            .field("order", self.page.effective_sort_order().as_ref())
            .field("visibility", self.effective_visibility().as_ref())
            .set(
                "labels",
                self.labels
                    .included
                    .as_slice()
                    .iter()
                    .map(Uuid::to_string)
                    .collect::<Vec<_>>(),
            )
            .field("labelsMode", self.labels.mode.as_ref())
            .set(
                "excludedLabels",
                self.labels
                    .excluded
                    .as_slice()
                    .iter()
                    .map(Uuid::to_string)
                    .collect::<Vec<_>>(),
            )
            .set(
                "kind",
                self.kinds
                    .as_slice()
                    .iter()
                    .map(|k| k.as_ref().to_owned())
                    .collect::<Vec<_>>(),
            )
            .set(
                "status",
                self.statuses
                    .as_slice()
                    .iter()
                    .map(|s| s.as_ref().to_owned())
                    .collect::<Vec<_>>(),
            )
            .set("contentRating", uuids(&self.content_rating_ids))
            .set("publicationLanguage", uuids(&self.publication_language_ids))
            .set(
                "publicationDemographic",
                self.publication_demographics
                    .as_slice()
                    .iter()
                    .map(|d| d.as_ref().to_owned())
                    .collect::<Vec<_>>(),
            )
            .set(
                "availableTranslatedLanguage",
                uuids(&self.available_translated_language_ids),
            )
            .field("publicationYearFrom", &bound(self.publication_year.from()))
            .field("publicationYearTo", &bound(self.publication_year.to()))
            .field("createdAtFrom", &instant(self.created_at.from()))
            .field("createdAtTo", &instant(self.created_at.to()))
    }

    /// A cursor is only correct for the filter and sort it was minted under. Without this the
    /// caller pages through quietly wrong results rather than seeing an error.
    pub fn ensure_cursor_matches(&self) -> Result<(), EntityError> {
        let Some(cursor) = &self.cursor else {
            return Ok(());
        };

        if cursor.field() != self.effective_sort() || cursor.filter_hash != self.canonical().hash()
        {
            return Err(EntityError::CursorFilterMismatch);
        }

        Ok(())
    }
}

fn bound(v: Option<&i16>) -> String {
    v.map(i16::to_string).unwrap_or_default()
}

// Rendered as an instant rather than as text, so two spellings of one moment hash alike.
fn instant(v: Option<&OffsetDateTime>) -> String {
    v.map(|at| at.unix_timestamp_nanos().to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use time::{Duration, OffsetDateTime};
    use uuid::Uuid;

    use crate::entity::{
        BookCursor, BookFilter, BookKind, BookKinds, BookLabelIds, BookSortField, BookSortValue,
        BookStatuses, BookVisibility, CreatedAtRange, Filter, FilterLookupIds, LabelFilter,
        LabelsMode, PublicationDemographics, PublicationYearRange,
    };

    fn filter() -> BookFilter {
        BookFilter {
            page: Filter::builder().build().unwrap(),
            visibility: None,
            sort: None,
            cursor: None,
            labels: LabelFilter {
                included: BookLabelIds::try_from(vec![]).unwrap(),
                mode: LabelsMode::And,
                excluded: BookLabelIds::try_from(vec![]).unwrap(),
            },
            kinds: BookKinds::try_from(vec![]).unwrap(),
            statuses: BookStatuses::try_from(vec![]).unwrap(),
            content_rating_ids: FilterLookupIds::try_from(vec![]).unwrap(),
            publication_language_ids: FilterLookupIds::try_from(vec![]).unwrap(),
            publication_demographics: PublicationDemographics::try_from(vec![]).unwrap(),
            available_translated_language_ids: FilterLookupIds::try_from(vec![]).unwrap(),
            publication_year: PublicationYearRange::try_new(None, None).unwrap(),
            created_at: CreatedAtRange::try_new(None, None).unwrap(),
        }
    }

    fn cursor_for(filter: &BookFilter) -> BookCursor {
        BookCursor::new(
            BookSortValue::CreatedAt(OffsetDateTime::UNIX_EPOCH),
            Uuid::now_v7(),
            filter.canonical().hash(),
        )
    }

    #[test]
    fn omitted_sort_falls_back_to_creation_time() {
        assert_eq!(filter().effective_sort(), BookSortField::CreatedAt);
    }

    #[test]
    fn provided_sort_wins_over_the_default() {
        let filter = BookFilter {
            sort: Some(BookSortField::Name),
            ..filter()
        };

        assert_eq!(filter.effective_sort(), BookSortField::Name);
    }

    #[test]
    fn a_cursor_from_the_same_filter_is_accepted() {
        let mut filter = filter();
        filter.cursor = Some(cursor_for(&filter));

        assert!(filter.ensure_cursor_matches().is_ok());
    }

    #[test]
    fn no_cursor_is_always_accepted() {
        assert!(filter().ensure_cursor_matches().is_ok());
    }

    #[test]
    fn a_cursor_is_rejected_once_any_facet_moves() {
        let base = filter();
        let cursor = cursor_for(&base);

        let changed = [
            BookFilter {
                labels: LabelFilter {
                    included: BookLabelIds::try_from(vec![Uuid::now_v7()]).unwrap(),
                    mode: LabelsMode::And,
                    excluded: BookLabelIds::try_from(vec![]).unwrap(),
                },
                cursor: Some(cursor.clone()),
                ..filter()
            },
            BookFilter {
                kinds: BookKinds::try_from(vec![BookKind::Manhwa]).unwrap(),
                cursor: Some(cursor.clone()),
                ..filter()
            },
            BookFilter {
                visibility: Some(BookVisibility::Hidden),
                cursor: Some(cursor.clone()),
                ..filter()
            },
            BookFilter {
                publication_year: PublicationYearRange::try_new(Some(2010), None).unwrap(),
                cursor: Some(cursor.clone()),
                ..filter()
            },
            BookFilter {
                created_at: CreatedAtRange::try_new(Some(OffsetDateTime::UNIX_EPOCH), None)
                    .unwrap(),
                cursor: Some(cursor.clone()),
                ..filter()
            },
            BookFilter {
                page: Filter::builder()
                    .sort_order(Some(crate::entity::SortOrder::Asc))
                    .build()
                    .unwrap(),
                cursor: Some(cursor.clone()),
                ..filter()
            },
        ];

        for filter in changed {
            assert!(
                filter.ensure_cursor_matches().is_err(),
                "{filter:?} must reject a cursor minted before the change"
            );
        }
    }

    #[test]
    fn a_cursor_for_another_sort_field_is_rejected() {
        let filter = BookFilter {
            sort: Some(BookSortField::Name),
            ..filter()
        };
        // Minted under this exact filter, so the hash agrees and only the field disagrees.
        let cursor = BookCursor::new(
            BookSortValue::CreatedAt(OffsetDateTime::UNIX_EPOCH),
            Uuid::now_v7(),
            filter.canonical().hash(),
        );

        let filter = BookFilter {
            cursor: Some(cursor),
            ..filter
        };

        assert!(filter.ensure_cursor_matches().is_err());
    }

    #[test]
    fn page_size_is_outside_the_filter_hash() {
        let one = BookFilter {
            page: Filter::builder().limit(Some(5)).build().unwrap(),
            ..filter()
        };
        let other = BookFilter {
            page: Filter::builder().limit(Some(50)).build().unwrap(),
            ..filter()
        };

        assert_eq!(
            one.canonical().hash(),
            other.canonical().hash(),
            "changing page size mid-paging is legitimate"
        );
    }

    #[test]
    fn two_spellings_of_one_instant_are_the_same_filter() {
        let utc = OffsetDateTime::UNIX_EPOCH + Duration::hours(12);
        let shifted = utc.to_offset(time::UtcOffset::from_hms(2, 0, 0).unwrap());

        let one = BookFilter {
            created_at: CreatedAtRange::try_new(Some(utc), None).unwrap(),
            ..filter()
        };
        let other = BookFilter {
            created_at: CreatedAtRange::try_new(Some(shifted), None).unwrap(),
            ..filter()
        };

        assert_eq!(one.canonical().hash(), other.canonical().hash());
    }

    #[test]
    fn label_mode_is_part_of_the_filter() {
        let and = BookFilter {
            labels: LabelFilter {
                included: BookLabelIds::try_from(vec![Uuid::nil()]).unwrap(),
                mode: LabelsMode::And,
                excluded: BookLabelIds::try_from(vec![]).unwrap(),
            },
            ..filter()
        };
        let or = BookFilter {
            labels: LabelFilter {
                included: BookLabelIds::try_from(vec![Uuid::nil()]).unwrap(),
                mode: LabelsMode::Or,
                excluded: BookLabelIds::try_from(vec![]).unwrap(),
            },
            ..filter()
        };

        assert_ne!(
            and.canonical().hash(),
            or.canonical().hash(),
            "AND and OR select different books, so a cursor must not cross between them"
        );
    }
}
