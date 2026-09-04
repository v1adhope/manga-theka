use crate::{
    entity::{BookSelection, HexHash},
    error::HasherError,
};

#[derive(Debug, Clone)]
pub struct Hasher;

impl Hasher {
    pub fn compute_hex_hash(target: &BookSelection) -> Result<HexHash, HasherError> {
        const LEN: usize = 16;

        let json = serde_json::to_string(target)
            .map_err(HasherError::Serialize)
            .inspect_err(HasherError::log_internal)?;
        let hex = blake3::hash(json.as_bytes()).to_hex();

        HexHash::try_from(hex[..LEN].to_owned())
            .map_err(HasherError::Digest)
            .inspect_err(HasherError::log_internal)
    }
}

#[cfg(test)]
pub mod tests {
    use time::{Duration, OffsetDateTime};

    use crate::{
        entity::{
            BookKinds, BookLabelIds, BookSelection, BookSortField, BookStatuses, BookVisibility,
            CreatedAtRange, FilterLookupIds, LabelFilter, LabelsMode, PublicationDemographics,
            PublicationYearRange, SortOrder, Timestamp,
        },
        hasher::Hasher,
    };

    pub fn stub() -> BookSelection {
        BookSelection {
            visibility: BookVisibility::Listed,
            sort_field: BookSortField::CreatedAt,
            sort_order: SortOrder::Desc,
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

    #[test]
    fn a_hash_is_stable_across_runs() {
        assert_eq!(
            Hasher::compute_hex_hash(&stub()).unwrap(),
            Hasher::compute_hex_hash(&stub()).unwrap(),
        );
    }

    #[test]
    fn a_moved_facet_hashes_differently() {
        let moved = BookSelection {
            visibility: BookVisibility::Hidden,
            ..stub()
        };

        assert_ne!(
            Hasher::compute_hex_hash(&stub()).unwrap(),
            Hasher::compute_hex_hash(&moved).unwrap(),
        );
    }

    #[test]
    fn two_spellings_of_one_instant_are_the_same_filter() {
        let utc = OffsetDateTime::UNIX_EPOCH + Duration::hours(12);
        let shifted = utc.to_offset(time::UtcOffset::from_hms(2, 0, 0).unwrap());

        let one = BookSelection {
            created_at: CreatedAtRange::try_new(Some(Timestamp::from(utc)), None).unwrap(),
            ..stub()
        };
        let other = BookSelection {
            created_at: CreatedAtRange::try_new(Some(Timestamp::from(shifted)), None).unwrap(),
            ..stub()
        };

        assert_eq!(
            Hasher::compute_hex_hash(&one).unwrap(),
            Hasher::compute_hex_hash(&other).unwrap(),
        );
    }
}
