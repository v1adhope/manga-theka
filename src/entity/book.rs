use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::{str::FromStr, time::Duration};
use time::OffsetDateTime;
use url::Url;
use uuid::Uuid;

use crate::{
    entity::{ContentRating, Creator, Label, Language},
    error::EntityError,
};

pub const COVER_MAX_BYTES: usize = 5 * 1024 * 1024;
pub const COVER_PRESIGN_TTL: Duration = Duration::from_secs(300);

const JPEG_SOI: [u8; 3] = [0xFF, 0xD8, 0xFF];
const PNG_SIGNATURE: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
const RIFF_MAGIC: [u8; 4] = *b"RIFF";
const WEBP_FORM_TYPE: [u8; 4] = *b"WEBP";

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

#[derive(Debug, PartialEq, Serialize)]
pub enum CoverExtension {
    Jpg,
    Png,
    Webp,
}

impl CoverExtension {
    pub fn content_type(&self) -> &str {
        match self {
            Self::Jpg => "image/jpeg",
            Self::Png => "image/png",
            Self::Webp => "image/webp",
        }
    }

    pub fn content_disposition(&self, id: Uuid) -> String {
        format!("inline; filename=\"{id}.{}\"", self.as_ref())
    }
}

impl TryFrom<&[u8]> for CoverExtension {
    type Error = EntityError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.starts_with(&JPEG_SOI) {
            return Ok(Self::Jpg);
        }
        if bytes.starts_with(&PNG_SIGNATURE) {
            return Ok(Self::Png);
        }
        if bytes.len() >= 12 && bytes.starts_with(&RIFF_MAGIC) && bytes[8..12] == WEBP_FORM_TYPE {
            return Ok(Self::Webp);
        }

        Err(EntityError::UnsupportedImageFormat)
    }
}

impl FromStr for CoverExtension {
    type Err = EntityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "jpg" => Ok(Self::Jpg),
            "png" => Ok(Self::Png),
            "webp" => Ok(Self::Webp),
            other => Err(EntityError::InvalidCoverExtension(other.to_owned())),
        }
    }
}

impl AsRef<str> for CoverExtension {
    fn as_ref(&self) -> &str {
        match self {
            Self::Jpg => "jpg",
            Self::Png => "png",
            Self::Webp => "webp",
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BookName(String);

impl TryFrom<String> for BookName {
    type Error = EntityError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        if s.trim().is_empty() {
            return Err(EntityError::NameIsEmptyOrWhitespace);
        }
        if s.chars().count() > 255 {
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Book {
    pub id: Uuid,
    pub name: BookName,
    pub description: Description,
    pub publication_year: i16,
    pub content_rating: ContentRating,
    pub status: BookStatus,
    pub kind: BookKind,
    pub publication_language: Language,
    pub labels: Vec<Label>,
    pub links: Vec<BookLink>,
    pub titles: Vec<AlternativeTitle>,
    pub creators: Vec<Creator>,
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

#[derive(Debug)]
pub struct BookCover {
    pub id: Uuid,
    pub book_id: Uuid,
    pub extension: CoverExtension,
    pub content: Bytes,
    pub is_main: bool,
}

impl BookCover {
    pub fn content_disposition(&self) -> String {
        self.extension.content_disposition(self.id)
    }
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(transparent)]
pub struct CoverUrl(String);

impl From<(Uuid, Uuid)> for CoverUrl {
    fn from((book_id, id): (Uuid, Uuid)) -> Self {
        Self(format!("/books/{book_id}/covers/{id}/image"))
    }
}

impl AsRef<str> for CoverUrl {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BookCoverQuery {
    pub id: Uuid,
    pub extension: CoverExtension,
    pub is_main: bool,
    pub url: CoverUrl,
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::entity::{BookCover, BookName, CoverExtension, CoverUrl, Description, LinkUrl};
    use bytes::Bytes;

    #[test]
    fn content_disposition_is_inline_with_the_extension_suffixed_filename() {
        let cover = BookCover {
            id: Uuid::from_u128(1),
            book_id: Uuid::from_u128(2),
            extension: CoverExtension::Webp,
            content: Bytes::new(),
            is_main: false,
        };

        assert_eq!(
            cover.content_disposition(),
            "inline; filename=\"00000000-0000-0000-0000-000000000001.webp\""
        );
    }

    #[test]
    fn cover_url_points_at_the_book_scoped_image_route() {
        let url = CoverUrl::from((Uuid::from_u128(1), Uuid::from_u128(2)));

        assert_eq!(
            url.as_ref(),
            "/books/00000000-0000-0000-0000-000000000001/covers/00000000-0000-0000-0000-000000000002/image"
        );
    }

    #[test]
    fn jpeg_magic_bytes_are_sniffed() {
        let res = CoverExtension::try_from([0xFF, 0xD8, 0xFF, 0xE0].as_slice());
        assert_eq!(res.unwrap(), CoverExtension::Jpg);
    }

    #[test]
    fn png_magic_bytes_are_sniffed() {
        let res =
            CoverExtension::try_from([0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A].as_slice());
        assert_eq!(res.unwrap(), CoverExtension::Png);
    }

    #[test]
    fn webp_magic_bytes_are_sniffed() {
        let res = CoverExtension::try_from(b"RIFF\x34\x00\x00\x00WEBPVP8 ".as_slice());
        assert_eq!(res.unwrap(), CoverExtension::Webp);
    }

    #[test]
    fn riff_without_webp_is_rejected() {
        let res = CoverExtension::try_from(b"RIFF\x34\x00\x00\x00WAVEfmt ".as_slice());
        assert!(res.is_err());
    }

    #[test]
    fn truncated_riff_header_is_rejected() {
        let res = CoverExtension::try_from(b"RIFF\x34\x00\x00".as_slice());
        assert!(res.is_err());
    }

    #[test]
    fn empty_body_is_rejected() {
        let res = CoverExtension::try_from([].as_slice());
        assert!(res.is_err());
    }

    #[test]
    fn gif_magic_bytes_are_rejected() {
        let res = CoverExtension::try_from(b"GIF89a".as_slice());
        assert!(res.is_err());
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
