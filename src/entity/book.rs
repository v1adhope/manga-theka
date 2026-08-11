use serde::{Deserialize, Serialize};
use std::str::FromStr;
use time::OffsetDateTime;
use unicode_segmentation::UnicodeSegmentation;
use uuid::Uuid;

use crate::{entity::Label, error::EntityError};

pub const MAX_TITLES: usize = 12;

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

/// A `Book Name` or an `Alternative Title`: both are free-form names of a book
/// under the same rules, unlike a `Creator`'s letters-only `Name`.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BookName(String);

impl TryFrom<String> for BookName {
    type Error = EntityError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        if s.trim().is_empty() {
            return Err(EntityError::NameIsEmptyOrWhitespace(s));
        }
        if s.graphemes(true).count() > 255 {
            return Err(EntityError::NameExceedsCharLimit(s));
        }
        Ok(Self(s))
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
        if s.graphemes(true).count() > 2000 {
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
pub struct LinkUrl(String);

impl TryFrom<String> for LinkUrl {
    type Error = EntityError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        if s.graphemes(true).count() > 2048 {
            return Err(EntityError::LinkUrlExceedsCharLimit);
        }
        // Allow-list the schemes a client can safely render as a link, per
        // docs/security.md.
        if !s.starts_with("http://") && !s.starts_with("https://") {
            return Err(EntityError::LinkUrlSchemeNotAllowed(s));
        }
        Ok(Self(s))
    }
}

impl AsRef<str> for LinkUrl {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Book {
    pub id: Uuid,
    pub name: BookName,
    pub description: Description,
    pub publication_year: i16,
    pub content_rating: Uuid,
    pub status: BookStatus,
    #[serde(rename = "type")]
    pub kind: BookKind,
    pub publication_language: Uuid,
    pub author: Uuid,
    pub artist: Uuid,
    #[serde(with = "time::serde::rfc3339::option")]
    pub updated_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookLink {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub kind: BookLinkKind,
    pub url: LinkUrl,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlternativeTitle {
    pub id: Uuid,
    pub language_id: Uuid,
    pub name: BookName,
}

#[derive(Debug)]
pub struct Titles(Vec<AlternativeTitle>);

impl TryFrom<Vec<AlternativeTitle>> for Titles {
    type Error = EntityError;

    fn try_from(titles: Vec<AlternativeTitle>) -> Result<Self, Self::Error> {
        if titles.len() > MAX_TITLES {
            return Err(EntityError::TitlesExceedLimit(titles.len(), MAX_TITLES));
        }
        Ok(Self(titles))
    }
}

impl Titles {
    pub fn as_slice(&self) -> &[AlternativeTitle] {
        &self.0
    }
}

/// A book together with the arrays a write replaces wholesale (ADR-0005).
#[derive(Debug)]
pub struct BookWrite {
    pub book: Book,
    pub label_ids: Vec<Uuid>,
    pub links: Vec<BookLink>,
    pub titles: Titles,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BookDetails {
    #[serde(flatten)]
    pub book: Book,
    pub labels: Vec<Label>,
    pub links: Vec<BookLink>,
    pub titles: Vec<AlternativeTitle>,
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::entity::{AlternativeTitle, BookName, Description, LinkUrl, MAX_TITLES, Titles};

    fn title() -> AlternativeTitle {
        AlternativeTitle {
            id: Uuid::now_v7(),
            language_id: Uuid::now_v7(),
            name: BookName::try_from("Alt".to_owned()).unwrap(),
        }
    }

    #[test]
    fn titles_at_the_limit_are_valid() {
        let titles: Vec<AlternativeTitle> = (0..MAX_TITLES).map(|_| title()).collect();
        let res = Titles::try_from(titles);
        assert!(res.is_ok());
    }

    #[test]
    fn titles_above_the_limit_are_rejected() {
        let titles: Vec<AlternativeTitle> = (0..MAX_TITLES + 1).map(|_| title()).collect();
        let res = Titles::try_from(titles);
        assert!(res.is_err());
    }

    #[test]
    fn book_name_255_graphemes_is_valid() {
        let res = BookName::try_from("ё".repeat(255));
        assert!(res.is_ok());
    }

    #[test]
    fn book_name_longer_256_graphemes_is_rejected() {
        let res = BookName::try_from("ё".repeat(256));
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
    fn description_2000_graphemes_is_valid() {
        let res = Description::try_from("ё".repeat(2000));
        assert!(res.is_ok());
    }

    #[test]
    fn description_longer_2001_graphemes_is_rejected() {
        let res = Description::try_from("ё".repeat(2001));
        assert!(res.is_err());
    }

    #[test]
    fn link_url_2048_graphemes_is_valid() {
        let url = format!("https://a.co/{}", "b".repeat(2035));
        assert_eq!(url.len(), 2048);
        let res = LinkUrl::try_from(url);
        assert!(res.is_ok());
    }

    #[test]
    fn link_url_longer_2049_graphemes_is_rejected() {
        let url = format!("https://a.co/{}", "b".repeat(2036));
        let res = LinkUrl::try_from(url);
        assert!(res.is_err());
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
}
