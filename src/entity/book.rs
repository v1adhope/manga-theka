use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use time::OffsetDateTime;
use url::Url;
use uuid::Uuid;

use crate::{
    entity::{
        Bounded, BoundedVec, ContentRating, CreatorQuery, CreatorRole, Entity, Filter, Image,
        ImageExtension, Label, Language, ResourceUrl, Text, validate_name,
    },
    error::EntityError,
};

pub const MAX_BOOK_LINKS: usize = 12;
pub const MAX_BOOK_TITLES: usize = 12;
// Matches the total number of rows seeded in the `labels` table.
pub const MAX_BOOK_LABELS: usize = 31;
pub const MAX_BOOK_CREATORS: usize = 20;

#[derive(Debug, PartialEq, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
pub enum BookVisibility {
    Draft,
    PendingReview,
    Listed,
    Rejected,
    Hidden,
}

impl FromStr for BookVisibility {
    type Err = EntityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Draft" => Ok(Self::Draft),
            "PendingReview" => Ok(Self::PendingReview),
            "Listed" => Ok(Self::Listed),
            "Rejected" => Ok(Self::Rejected),
            "Hidden" => Ok(Self::Hidden),
            other => Err(EntityError::InvalidBookVisibility(other.to_owned())),
        }
    }
}

impl AsRef<str> for BookVisibility {
    fn as_ref(&self) -> &str {
        match self {
            Self::Draft => "Draft",
            Self::PendingReview => "PendingReview",
            Self::Listed => "Listed",
            Self::Rejected => "Rejected",
            Self::Hidden => "Hidden",
        }
    }
}

impl fmt::Display for BookVisibility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_ref())
    }
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
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

pub const SUBMITTED_NOTE: &str = "Your submission has been received and is currently under review. \
                                  We'll process it within 3 business days - thanks for your patience!";

#[derive(Debug)]
pub struct VisibilityTransition {
    pub id: Uuid,
    pub visibility: BookVisibility,
    pub note: Option<Text>,
    pub now: OffsetDateTime,
}

#[derive(Debug)]
pub struct BookVisibilityUpdate {
    pub id: Uuid,
    pub from: BookVisibility,
    pub to: BookVisibility,
    pub note: Option<Text>,
    pub submitted_at: Option<OffsetDateTime>,
}

impl TryFrom<(BookVisibility, VisibilityTransition)> for BookVisibilityUpdate {
    type Error = EntityError;

    fn try_from(ctx: (BookVisibility, VisibilityTransition)) -> Result<Self, Self::Error> {
        use BookVisibility::{Draft, Hidden, Listed, PendingReview, Rejected};

        let (from, item) = ctx;
        let VisibilityTransition {
            id,
            visibility,
            note,
            now,
        } = item;

        if from != visibility {
            match (from, visibility) {
                (Draft, PendingReview | Rejected | Hidden)
                | (PendingReview, Draft | Listed | Rejected | Hidden)
                | (Listed, Hidden | Rejected)
                | (Hidden, Listed | Rejected) => {}
                _ => return Err(EntityError::IllegalVisibilityTransition(from, visibility)),
            }
        }

        let (note, submitted_at) = match visibility {
            PendingReview => (
                Some(Text::try_from(SUBMITTED_NOTE.to_owned())?),
                (from != visibility).then_some(now),
            ),
            Listed => (note, None),
            Draft | Rejected | Hidden => (
                Some(note.ok_or(EntityError::BookNoteRequired(visibility))?),
                None,
            ),
        };

        Ok(Self {
            id,
            from,
            to: visibility,
            note,
            submitted_at,
        })
    }
}

#[derive(Debug)]
pub struct BookFilter {
    pub page: Filter,
    pub visibility: Option<BookVisibility>,
}

impl BookFilter {
    pub fn effective_visibility(&self) -> BookVisibility {
        self.visibility.unwrap_or(BookVisibility::Listed)
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

    use crate::entity::{
        AlternativeTitle, BookCreatorsQuery, BookLabelIds, BookLink, BookLinkKind, BookLinks,
        BookName, BookTitles, BookVisibility, BookVisibilityUpdate, CreatorQuery, CreatorRole,
        LinkUrl, MAX_BOOK_CREATORS, MAX_BOOK_LABELS, MAX_BOOK_LINKS, MAX_BOOK_TITLES, Name,
        SUBMITTED_NOTE, Text, VisibilityTransition,
    };

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

    const EVERY_VISIBILITY: [BookVisibility; 5] = [
        BookVisibility::Draft,
        BookVisibility::PendingReview,
        BookVisibility::Listed,
        BookVisibility::Rejected,
        BookVisibility::Hidden,
    ];

    fn transition(visibility: BookVisibility, note: Option<&str>) -> VisibilityTransition {
        VisibilityTransition {
            id: Uuid::now_v7(),
            visibility,
            note: note.map(|n| Text::try_from(n.to_owned()).unwrap()),
            now: OffsetDateTime::now_utc(),
        }
    }

    fn is_legal(from: BookVisibility, to: BookVisibility) -> bool {
        use BookVisibility::{Draft, Hidden, Listed, PendingReview, Rejected};

        matches!(
            (from, to),
            (Draft, Draft | PendingReview | Rejected | Hidden)
                | (
                    PendingReview,
                    PendingReview | Draft | Listed | Rejected | Hidden
                )
                | (Listed, Listed | Hidden | Rejected)
                | (Hidden, Hidden | Listed | Rejected)
                | (Rejected, Rejected)
        )
    }

    #[test]
    fn every_book_visibility_round_trips() {
        for visibility in EVERY_VISIBILITY {
            let parsed: BookVisibility = visibility.as_ref().parse().unwrap();
            assert_eq!(parsed, visibility);
        }
    }

    #[test]
    fn unknown_book_visibility_is_rejected() {
        let res = "Published".parse::<BookVisibility>();
        assert!(res.is_err());
    }

    #[test]
    fn only_the_tabled_transitions_are_accepted() {
        for from in EVERY_VISIBILITY {
            for to in EVERY_VISIBILITY {
                let res = BookVisibilityUpdate::try_from((from, transition(to, Some("why"))));

                assert_eq!(
                    res.is_ok(),
                    is_legal(from, to),
                    "{from} -> {to} was judged wrongly"
                );
            }
        }
    }

    #[test]
    fn submitting_stamps_the_code_authored_note() {
        let update = BookVisibilityUpdate::try_from((
            BookVisibility::Draft,
            transition(BookVisibility::PendingReview, Some("mine")),
        ))
        .unwrap();

        assert_eq!(update.note.unwrap().as_ref(), SUBMITTED_NOTE);
    }

    #[test]
    fn submitting_stamps_the_submission_time() {
        let update = BookVisibilityUpdate::try_from((
            BookVisibility::Draft,
            transition(BookVisibility::PendingReview, None),
        ))
        .unwrap();

        assert!(update.submitted_at.is_some());
    }

    #[test]
    fn a_book_already_in_review_is_not_restamped() {
        let update = BookVisibilityUpdate::try_from((
            BookVisibility::PendingReview,
            transition(BookVisibility::PendingReview, None),
        ))
        .unwrap();

        assert_eq!(update.submitted_at, None);
    }

    #[test]
    fn listing_without_a_note_clears_it() {
        let update = BookVisibilityUpdate::try_from((
            BookVisibility::PendingReview,
            transition(BookVisibility::Listed, None),
        ))
        .unwrap();

        assert_eq!(update.note, None);
    }

    #[test]
    fn listing_with_a_note_replaces_it() {
        let update = BookVisibilityUpdate::try_from((
            BookVisibility::PendingReview,
            transition(BookVisibility::Listed, Some("welcome")),
        ))
        .unwrap();

        assert_eq!(update.note.unwrap().as_ref(), "welcome");
    }

    #[test]
    fn moves_that_explain_themselves_require_a_note() {
        for to in [
            BookVisibility::Draft,
            BookVisibility::Rejected,
            BookVisibility::Hidden,
        ] {
            let res = BookVisibilityUpdate::try_from((
                BookVisibility::PendingReview,
                transition(to, None),
            ));

            assert!(res.is_err(), "{to} without a note must be rejected");
        }
    }

    #[test]
    fn a_self_transition_takes_its_own_note_rule() {
        let cleared = BookVisibilityUpdate::try_from((
            BookVisibility::Listed,
            transition(BookVisibility::Listed, None),
        ))
        .unwrap();
        let refused = BookVisibilityUpdate::try_from((
            BookVisibility::Hidden,
            transition(BookVisibility::Hidden, None),
        ));

        assert_eq!(cleared.note, None);
        assert!(refused.is_err(), "a hidden book still owes a reason");
    }

    #[test]
    fn a_self_transition_never_stamps_a_submission_time() {
        for visibility in EVERY_VISIBILITY {
            let Ok(update) =
                BookVisibilityUpdate::try_from((visibility, transition(visibility, Some("same"))))
            else {
                continue;
            };

            assert_eq!(update.submitted_at, None, "{visibility} restamped itself");
        }
    }
}
