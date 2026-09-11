use rstest::fixture;

use crate::entity::{
    BookKinds, BookLabelIds, BookSelection, BookSortField, BookStatuses, BookVisibility,
    CreatedAtRange, FilterLookupIds, LabelFilter, LabelsMode, PublicationDemographics,
    PublicationYearRange, SortOrder,
};

#[fixture]
pub(crate) fn unfiltered_selection() -> BookSelection {
    BookSelection {
        visibility: BookVisibility::Listed,
        sort_field: BookSortField::CreatedAt,
        order: SortOrder::Desc,
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
