use serde::{Deserialize, Serialize};
use std::str::FromStr;
use time::OffsetDateTime;
use url::Url;
use uuid::Uuid;

use crate::{
    entity::{
        Bounded, BoundedVec, ContentRating, CreatorQuery, CreatorRole, Entity, Image,
        ImageExtension, Label, Language, ResourceUrl, validate_name,
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
pub struct Description(String);

impl TryFrom<String> for Description {
    type Error = EntityError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        if s.trim().is_empty() {
            return Err(EntityError::DescriptionIsEmptyOrWhitespace);
        }
        if s.chars().count() > 2000 {
            return Err(EntityError::DescriptionExceedsCharLimit);
        }
        Ok(Self(s))
    }
}

impl AsRef<str> for Description {
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
    pub description: Description,
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookQuery {
    pub id: Uuid,
    pub name: BookName,
    pub description: Description,
    pub publication_year: i16,
    pub content_rating: ContentRating,
    pub status: BookStatus,
    pub kind: BookKind,
    pub publication_language: Language,
    pub labels: BookLabels,
    pub links: BookLinks,
    pub titles: BookTitles,
    pub creators: BookCreatorsQuery,
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
        BookName, BookTitles, CreatorQuery, CreatorRole, Description, LinkUrl, MAX_BOOK_CREATORS,
        MAX_BOOK_LABELS, MAX_BOOK_LINKS, MAX_BOOK_TITLES, Name,
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
    fn description_2000_chars_is_valid() {
        let res = Description::try_from("ё".repeat(2000));
        assert!(res.is_ok());
    }

    #[test]
    fn description_longer_2001_chars_is_rejected() {
        let res = Description::try_from("ё".repeat(2001));
        assert!(res.is_err());
    }

    #[test]
    fn whitespace_only_description_is_rejected() {
        let res = Description::try_from(" ".to_owned());
        assert!(res.is_err());
    }

    #[test]
    fn empty_description_is_rejected() {
        let res = Description::try_from(String::new());
        assert!(res.is_err());
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
