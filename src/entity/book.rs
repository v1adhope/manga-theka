use serde::{Deserialize, Serialize};
use std::str::FromStr;
use time::OffsetDateTime;
use url::Url;
use uuid::Uuid;

use crate::{
    entity::{
        BookCursor, BookSortField, BookVisibility, Bounded, BoundedVec, CanonicalFilter,
        ContentRating, CreatorQuery, CreatorRole, Entity, Filter, Image, ImageExtension, Label,
        Language, Range, RangeBound, ResourceUrl, Text, validate_name,
    },
    error::EntityError,
};

pub const MAX_BOOK_LINKS: usize = 12;
pub const MAX_BOOK_TITLES: usize = 12;
// Matches the total number of rows seeded in the `labels` table.
pub const MAX_BOOK_LABELS: usize = 70;
pub const MAX_BOOK_CREATORS: usize = 20;
// Open lookup sets with no fixed cardinality. Not a schema fact: 200 is a ceiling chosen to
// clear the ISO 639-1 alpha-2 code space and any plausible rating vocabulary, so that a
// repeated parameter cannot be turned into an amplification vector.
pub const MAX_FILTER_LOOKUP_VALUES: usize = 200;

pub const EVERY_BOOK_STATUS: [BookStatus; 4] = [
    BookStatus::Ongoing,
    BookStatus::Completed,
    BookStatus::Hiatus,
    BookStatus::Cancelled,
];

pub const EVERY_BOOK_KIND: [BookKind; 3] = [BookKind::Manga, BookKind::Manhwa, BookKind::Manhua];

pub const EVERY_PUBLICATION_DEMOGRAPHIC: [PublicationDemographic; 5] = [
    PublicationDemographic::Shounen,
    PublicationDemographic::Shoujo,
    PublicationDemographic::Seinen,
    PublicationDemographic::Josei,
    PublicationDemographic::Kids,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub enum BookStatus {
    Ongoing,
    Completed,
    Hiatus,
    Cancelled,
}

impl FromStr for BookStatus {
    type Err = EntityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Ongoing" => Ok(Self::Ongoing),
            "Completed" => Ok(Self::Completed),
            "Hiatus" => Ok(Self::Hiatus),
            "Cancelled" => Ok(Self::Cancelled),
            other => Err(EntityError::InvalidBookStatus(other.to_owned())),
        }
    }
}

impl AsRef<str> for BookStatus {
    fn as_ref(&self) -> &str {
        match self {
            Self::Ongoing => "Ongoing",
            Self::Completed => "Completed",
            Self::Hiatus => "Hiatus",
            Self::Cancelled => "Cancelled",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub enum BookKind {
    Manga,
    Manhwa,
    Manhua,
}

impl FromStr for BookKind {
    type Err = EntityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Manga" => Ok(Self::Manga),
            "Manhwa" => Ok(Self::Manhwa),
            "Manhua" => Ok(Self::Manhua),
            other => Err(EntityError::InvalidBookKind(other.to_owned())),
        }
    }
}

impl AsRef<str> for BookKind {
    fn as_ref(&self) -> &str {
        match self {
            Self::Manga => "Manga",
            Self::Manhwa => "Manhwa",
            Self::Manhua => "Manhua",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub enum PublicationDemographic {
    Shounen,
    Shoujo,
    Seinen,
    Josei,
    Kids,
}

impl FromStr for PublicationDemographic {
    type Err = EntityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Shounen" => Ok(Self::Shounen),
            "Shoujo" => Ok(Self::Shoujo),
            "Seinen" => Ok(Self::Seinen),
            "Josei" => Ok(Self::Josei),
            "Kids" => Ok(Self::Kids),
            other => Err(EntityError::InvalidPublicationDemographic(other.to_owned())),
        }
    }
}

impl AsRef<str> for PublicationDemographic {
    fn as_ref(&self) -> &str {
        match self {
            Self::Shounen => "Shounen",
            Self::Shoujo => "Shoujo",
            Self::Seinen => "Seinen",
            Self::Josei => "Josei",
            Self::Kids => "Kids",
        }
    }
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub enum BookLinkKind {
    WhereToRead,
    WhereToBuy,
    Track,
}

impl FromStr for BookLinkKind {
    type Err = EntityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "WhereToRead" => Ok(Self::WhereToRead),
            "WhereToBuy" => Ok(Self::WhereToBuy),
            "Track" => Ok(Self::Track),
            other => Err(EntityError::InvalidBookLinkKind(other.to_owned())),
        }
    }
}

impl AsRef<str> for BookLinkKind {
    fn as_ref(&self) -> &str {
        match self {
            Self::WhereToRead => "WhereToRead",
            Self::WhereToBuy => "WhereToBuy",
            Self::Track => "Track",
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BookName(String);

impl TryFrom<String> for BookName {
    type Error = EntityError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Ok(Self(validate_name(s)?))
    }
}

impl AsRef<str> for BookName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LinkUrl(Url);

impl TryFrom<String> for LinkUrl {
    type Error = EntityError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        let url = match Url::parse(&s) {
            Ok(url) => url,
            Err(e) => return Err(EntityError::LinkUrlIsMalformed(e, s)),
        };
        if url.as_str().chars().count() > 2048 {
            return Err(EntityError::LinkUrlExceedsCharLimit(
                url.as_str().chars().take(64).collect(),
            ));
        }
        if !matches!(url.scheme(), "http" | "https") {
            return Err(EntityError::LinkUrlSchemeNotAllowed(s));
        }
        Ok(Self(url))
    }
}

impl AsRef<str> for LinkUrl {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

#[derive(Debug)]
pub struct Book {
    pub id: Uuid,
    pub name: BookName,
    pub description: Text,
    pub publication_year: i16,
    pub content_rating_id: Uuid,
    pub status: BookStatus,
    pub kind: BookKind,
    pub publication_language_id: Uuid,
    pub publication_demographic: PublicationDemographic,
    pub label_ids: BookLabelIds,
    pub links: BookLinks,
    pub titles: BookTitles,
    pub creators: BookCreators,
    pub updated_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
}

impl Entity for Book {
    const NAME: &'static str = "Book";
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum LabelsMode {
    #[default]
    And,
    Or,
}

/// The label facet, whole. The mode says nothing without the set it applies to, and exclusion
/// is a blocklist that deliberately takes no mode of its own, so the three travel together and
/// the rule is read off one value rather than reassembled from scattered fields.
#[derive(Debug)]
pub struct LabelFilter {
    pub included: BookLabelIds,
    pub mode: LabelsMode,
    pub excluded: BookLabelIds,
}

pub struct PublicationYearBound;

impl RangeBound for PublicationYearBound {
    // A year is a discrete ordinal, so 2010..2015 means what a reader thinks it means.
    const UPPER_INCLUSIVE: bool = true;
    const NAME: &'static str = "publication year";
}

pub struct CreatedAtBound;

impl RangeBound for CreatedAtBound {
    // A timestamp is continuous, so a half-open interval lets adjacent ranges tile without a
    // book on the boundary landing in both.
    const UPPER_INCLUSIVE: bool = false;
    const NAME: &'static str = "created at";
}

pub type PublicationYearRange = Range<i16, PublicationYearBound>;
pub type CreatedAtRange = Range<OffsetDateTime, CreatedAtBound>;

pub struct BookKindsBound;

impl Bounded for BookKindsBound {
    const MAX: usize = EVERY_BOOK_KIND.len();
    const NAME: &'static str = "book kinds";
}

pub struct BookStatusesBound;

impl Bounded for BookStatusesBound {
    const MAX: usize = EVERY_BOOK_STATUS.len();
    const NAME: &'static str = "book statuses";
}

pub struct PublicationDemographicsBound;

impl Bounded for PublicationDemographicsBound {
    const MAX: usize = EVERY_PUBLICATION_DEMOGRAPHIC.len();
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
        self.sort.unwrap_or_default()
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

impl AsRef<str> for LabelsMode {
    fn as_ref(&self) -> &str {
        match self {
            Self::And => "AND",
            Self::Or => "OR",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookQuery {
    pub id: Uuid,
    pub name: BookName,
    pub description: Text,
    pub publication_year: i16,
    pub content_rating: ContentRating,
    pub status: BookStatus,
    pub kind: BookKind,
    pub publication_language: Language,
    pub publication_demographic: PublicationDemographic,
    pub labels: BookLabels,
    pub links: BookLinks,
    pub titles: BookTitles,
    pub creators: BookCreatorsQuery,
    pub visibility: BookVisibility,
    pub note: Option<Text>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub submitted_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub updated_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookLink {
    pub kind: BookLinkKind,
    pub url: LinkUrl,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlternativeTitle {
    pub language_id: Uuid,
    pub name: BookName,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookCreator {
    pub creator_id: Uuid,
    pub role: CreatorRole,
}

pub struct BookLinksBound;

impl Bounded for BookLinksBound {
    const MAX: usize = MAX_BOOK_LINKS;
    const NAME: &'static str = "book links";
}

pub type BookLinks = BoundedVec<BookLink, BookLinksBound>;

pub struct BookTitlesBound;

impl Bounded for BookTitlesBound {
    const MAX: usize = MAX_BOOK_TITLES;
    const NAME: &'static str = "alternative titles";
}

pub type BookTitles = BoundedVec<AlternativeTitle, BookTitlesBound>;

pub struct BookLabelsBound;

impl Bounded for BookLabelsBound {
    const MAX: usize = MAX_BOOK_LABELS;
    const NAME: &'static str = "book labels";
}

pub type BookLabels = BoundedVec<Label, BookLabelsBound>;
pub type BookLabelIds = BoundedVec<Uuid, BookLabelsBound>;

pub struct BookCreatorsBound;

impl Bounded for BookCreatorsBound {
    const MAX: usize = MAX_BOOK_CREATORS;
    const NAME: &'static str = "book creators";
}

pub type BookCreatorsQuery = BoundedVec<CreatorQuery, BookCreatorsBound>;
pub type BookCreators = BoundedVec<BookCreator, BookCreatorsBound>;

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

#[cfg(test)]
mod tests {
    use time::OffsetDateTime;
    use uuid::Uuid;

    use crate::entity::BookKind;

    use time::Duration;

    use crate::entity::{
        AlternativeTitle, BookCreatorsQuery, BookCursor, BookFilter, BookKinds, BookLabelIds,
        BookLink, BookLinkKind, BookLinks, BookName, BookSortField, BookSortValue, BookStatuses,
        BookTitles, BookVisibility, CreatedAtRange, CreatorQuery, CreatorRole, Filter,
        FilterLookupIds, LabelFilter, LabelsMode, LinkUrl, MAX_BOOK_CREATORS, MAX_BOOK_LABELS,
        MAX_BOOK_LINKS, MAX_BOOK_TITLES, Name, PublicationDemographic, PublicationDemographics,
        PublicationYearRange,
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

    #[test]
    fn labels_mode_defaults_to_and() {
        assert_eq!(LabelsMode::default(), LabelsMode::And);
    }

    fn sample_link() -> BookLink {
        BookLink {
            kind: BookLinkKind::WhereToRead,
            url: LinkUrl::try_from("https://example.com/read".to_owned()).unwrap(),
        }
    }

    fn sample_title() -> AlternativeTitle {
        AlternativeTitle {
            language_id: Uuid::now_v7(),
            name: BookName::try_from("Alt title".to_owned()).unwrap(),
        }
    }

    fn sample_creator() -> CreatorQuery {
        CreatorQuery {
            id: Uuid::now_v7(),
            first_name: Name::try_from("Jane".to_owned()).unwrap(),
            last_name: Name::try_from("Doe".to_owned()).unwrap(),
            roles: vec![CreatorRole::Author],
            created_at: OffsetDateTime::now_utc(),
        }
    }

    #[test]
    fn every_publication_demographic_round_trips() {
        for s in ["Shounen", "Shoujo", "Seinen", "Josei", "Kids"] {
            let demographic: PublicationDemographic = s.parse().unwrap();
            assert_eq!(demographic.as_ref(), s);
        }
    }

    #[test]
    fn unknown_publication_demographic_is_rejected() {
        let res = "Unknown".parse::<PublicationDemographic>();
        assert!(res.is_err());
    }

    #[test]
    fn book_links_at_the_ceiling_is_valid() {
        let links: Vec<BookLink> = (0..MAX_BOOK_LINKS).map(|_| sample_link()).collect();

        assert!(BookLinks::try_from(links).is_ok());
    }

    #[test]
    fn book_links_over_the_ceiling_is_rejected() {
        let links: Vec<BookLink> = (0..=MAX_BOOK_LINKS).map(|_| sample_link()).collect();

        assert!(BookLinks::try_from(links).is_err());
    }

    #[test]
    fn book_titles_at_the_ceiling_is_valid() {
        let titles: Vec<AlternativeTitle> = (0..MAX_BOOK_TITLES).map(|_| sample_title()).collect();

        assert!(BookTitles::try_from(titles).is_ok());
    }

    #[test]
    fn book_titles_over_the_ceiling_is_rejected() {
        let titles: Vec<AlternativeTitle> = (0..=MAX_BOOK_TITLES).map(|_| sample_title()).collect();

        assert!(BookTitles::try_from(titles).is_err());
    }

    #[test]
    fn book_label_ids_at_the_ceiling_is_valid() {
        let ids: Vec<Uuid> = (0..MAX_BOOK_LABELS).map(|_| Uuid::now_v7()).collect();

        assert!(BookLabelIds::try_from(ids).is_ok());
    }

    #[test]
    fn book_label_ids_over_the_ceiling_is_rejected() {
        let ids: Vec<Uuid> = (0..=MAX_BOOK_LABELS).map(|_| Uuid::now_v7()).collect();

        assert!(BookLabelIds::try_from(ids).is_err());
    }

    #[test]
    fn book_creators_at_the_ceiling_is_valid() {
        let creators: Vec<CreatorQuery> =
            (0..MAX_BOOK_CREATORS).map(|_| sample_creator()).collect();

        assert!(BookCreatorsQuery::try_from(creators).is_ok());
    }

    #[test]
    fn book_creators_over_the_ceiling_is_rejected() {
        let creators: Vec<CreatorQuery> =
            (0..=MAX_BOOK_CREATORS).map(|_| sample_creator()).collect();

        assert!(BookCreatorsQuery::try_from(creators).is_err());
    }

    #[test]
    fn book_name_255_chars_is_valid() {
        let res = BookName::try_from("ё".repeat(255));
        assert!(res.is_ok());
    }

    #[test]
    fn book_name_longer_256_chars_is_rejected() {
        let res = BookName::try_from("ё".repeat(256));
        assert!(res.is_err());
    }

    #[test]
    fn book_name_255_decomposed_letters_is_rejected() {
        let res = BookName::try_from("е\u{0308}".repeat(255));
        assert!(res.is_err());
    }

    #[test]
    fn book_name_255_zwj_sequences_is_rejected() {
        let res = BookName::try_from("👨‍👩‍👧‍👦".repeat(255));
        assert!(res.is_err());
    }

    #[test]
    fn whitespace_only_book_name_is_rejected() {
        let res = BookName::try_from(" ".to_owned());
        assert!(res.is_err());
    }

    #[test]
    fn book_name_with_digits_and_punctuation_is_valid() {
        let res = BookName::try_from("Re:Zero - Chapter 12.5!".to_owned());
        assert!(res.is_ok());
    }

    #[test]
    fn link_url_2048_chars_is_valid() {
        let url = format!("https://a.co/{}", "b".repeat(2035));
        assert_eq!(url.len(), 2048);
        let res = LinkUrl::try_from(url);
        assert!(res.is_ok());
    }

    #[test]
    fn link_url_longer_2049_chars_is_rejected() {
        let url = format!("https://a.co/{}", "b".repeat(2036));
        let res = LinkUrl::try_from(url);
        assert!(res.is_err());
    }

    #[test]
    fn link_url_over_limit_once_percent_encoded_is_rejected() {
        let url = format!("https://a.co/{}", "日".repeat(679));
        assert_eq!(url.chars().count(), 692);
        let res = LinkUrl::try_from(url);
        assert!(res.is_err());
    }

    #[test]
    fn link_url_at_limit_gaining_a_trailing_slash_is_rejected() {
        let url = format!("https://{}.co", "a".repeat(2037));
        assert_eq!(url.len(), 2048);
        let res = LinkUrl::try_from(url);
        assert!(res.is_err());
    }

    #[test]
    fn link_url_over_limit_before_normalization_is_valid() {
        let url = format!("https://a.co:443/{}", "b".repeat(2032));
        assert_eq!(url.len(), 2049);
        let res = LinkUrl::try_from(url);
        assert!(res.is_ok());
    }

    #[test]
    fn link_url_without_http_scheme_is_rejected() {
        let res = LinkUrl::try_from("javascript:alert(1)".to_owned());
        assert!(res.is_err());
    }

    #[test]
    fn link_url_with_http_scheme_is_valid() {
        let res = LinkUrl::try_from("http://example.com/read".to_owned());
        assert!(res.is_ok());
    }

    #[test]
    fn malformed_link_url_is_rejected() {
        let res = LinkUrl::try_from("https://".to_owned());
        assert!(res.is_err());
    }

    #[test]
    fn relative_link_url_is_rejected() {
        let res = LinkUrl::try_from("/read/1".to_owned());
        assert!(res.is_err());
    }
}
