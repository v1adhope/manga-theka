use std::{collections::HashSet, fmt, str::FromStr, time::Duration};

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    entity::{Entity, Language, is_jpeg, is_png, is_webp},
    error::EntityError,
};

pub const PAGE_MAX_BYTES: usize = 5 * 1024 * 1024;
pub const PAGE_PRESIGN_TTL: Duration = Duration::from_secs(300);
pub const MAX_PARTS_PER_REQUEST: usize = 10;
pub const MAX_COMMITTED_PAGES: usize = 200;
pub const MAX_RELEASE_ROWS: usize = 400;
pub const UPLOAD_MAX_BYTES: usize = MAX_PARTS_PER_REQUEST * (PAGE_MAX_BYTES + PART_HEADROOM_BYTES);

const PART_HEADROOM_BYTES: usize = 1024;

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub enum PageExtension {
    Jpg,
    Png,
    Webp,
}

impl PageExtension {
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

impl TryFrom<&[u8]> for PageExtension {
    type Error = EntityError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if is_jpeg(bytes) {
            return Ok(Self::Jpg);
        }
        if is_png(bytes) {
            return Ok(Self::Png);
        }
        if is_webp(bytes) {
            return Ok(Self::Webp);
        }

        Err(EntityError::UnsupportedImageFormat)
    }
}

impl FromStr for PageExtension {
    type Err = EntityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "jpg" => Ok(Self::Jpg),
            "png" => Ok(Self::Png),
            "webp" => Ok(Self::Webp),
            other => Err(EntityError::InvalidPageExtension(other.to_owned())),
        }
    }
}

impl AsRef<str> for PageExtension {
    fn as_ref(&self) -> &str {
        match self {
            Self::Jpg => "jpg",
            Self::Png => "png",
            Self::Webp => "webp",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ReleaseVersion(i32);

impl TryFrom<i32> for ReleaseVersion {
    type Error = EntityError;

    fn try_from(v: i32) -> Result<Self, Self::Error> {
        if v < 0 {
            return Err(EntityError::ReleaseVersionOutOfRange(v));
        }

        Ok(Self(v))
    }
}

impl FromStr for ReleaseVersion {
    type Err = EntityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let malformed = || EntityError::ReleaseVersionIsMalformed(s.to_owned());

        let digits = s
            .strip_prefix('"')
            .and_then(|rest| rest.strip_suffix('"'))
            .filter(|rest| !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit()))
            .ok_or_else(malformed)?;

        let v: i32 = digits.parse().map_err(|_| malformed())?;

        Self::try_from(v).map_err(|_| malformed())
    }
}

impl fmt::Display for ReleaseVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"{}\"", self.0)
    }
}

impl ReleaseVersion {
    pub fn as_i32(self) -> i32 {
        self.0
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PageUrl(String);

impl From<(Uuid, i16)> for PageUrl {
    fn from((release_id, page_number): (Uuid, i16)) -> Self {
        Self(format!("/releases/{release_id}/pages/{page_number}"))
    }
}

impl AsRef<str> for PageUrl {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug)]
pub struct PageOrder(Vec<Uuid>);

impl TryFrom<Vec<Uuid>> for PageOrder {
    type Error = EntityError;

    fn try_from(ids: Vec<Uuid>) -> Result<Self, Self::Error> {
        if ids.is_empty() {
            return Err(EntityError::PageOrderIsEmpty);
        }
        if ids.len() > MAX_COMMITTED_PAGES {
            return Err(EntityError::PageOrderExceedsLimit(
                ids.len(),
                MAX_COMMITTED_PAGES,
            ));
        }

        let mut seen = HashSet::with_capacity(ids.len());
        for id in &ids {
            if !seen.insert(id) {
                return Err(EntityError::PageOrderHasDuplicates(*id));
            }
        }

        Ok(Self(ids))
    }
}

impl PageOrder {
    pub fn as_slice(&self) -> &[Uuid] {
        &self.0
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
}

impl Entity for ChapterRelease {
    const NAME: &'static str = "Chapter release";
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterReleaseQuery {
    pub id: Uuid,
    pub chapter_id: Uuid,
    pub language: Language,
    pub page_count: i64,
    pub version: ReleaseVersion,
}

#[derive(Debug)]
pub struct ChapterPage {
    pub id: Uuid,
    pub release_id: Uuid,
    pub extension: PageExtension,
    pub content: Bytes,
}

impl Entity for ChapterPage {
    const NAME: &'static str = "Chapter page";
}

impl ChapterPage {
    pub fn content_disposition(&self) -> String {
        self.extension.content_disposition(self.id)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterPageQuery {
    pub id: Uuid,
    pub page_number: i16,
    pub extension: PageExtension,
    pub url: PageUrl,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StagedPageQuery {
    pub id: Uuid,
    pub extension: PageExtension,
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::entity::{MAX_COMMITTED_PAGES, PageExtension, PageOrder, PageUrl, ReleaseVersion};

    #[test]
    fn page_format_is_sniffed_from_magic_bytes() {
        let res = PageExtension::try_from(b"RIFF\x34\x00\x00\x00WEBPVP8 ".as_slice());
        assert_eq!(res.unwrap(), PageExtension::Webp);
    }

    #[test]
    fn unrecognised_page_format_is_rejected() {
        let res = PageExtension::try_from(b"GIF89a".as_slice());
        assert!(res.is_err());
    }

    #[test]
    fn page_content_disposition_is_inline_with_the_extension_suffixed_filename() {
        let disposition = PageExtension::Png.content_disposition(Uuid::from_u128(1));

        assert_eq!(
            disposition,
            "inline; filename=\"00000000-0000-0000-0000-000000000001.png\""
        );
    }

    #[test]
    fn release_version_parses_a_strong_entity_tag() {
        let version: ReleaseVersion = "\"7\"".parse().unwrap();
        assert_eq!(version.as_i32(), 7);
    }

    #[test]
    fn release_version_renders_as_a_quoted_entity_tag() {
        let version = ReleaseVersion::try_from(7).unwrap();
        assert_eq!(version.to_string(), "\"7\"");
    }

    #[test]
    fn unquoted_release_version_is_rejected() {
        let res = "7".parse::<ReleaseVersion>();
        assert!(res.is_err());
    }

    #[test]
    fn signed_release_version_is_rejected() {
        let res = "\"+0\"".parse::<ReleaseVersion>();
        assert!(res.is_err());
    }

    #[test]
    fn empty_release_version_is_rejected() {
        let res = "\"\"".parse::<ReleaseVersion>();
        assert!(res.is_err());
    }

    #[test]
    fn any_release_version_is_rejected() {
        let res = "*".parse::<ReleaseVersion>();
        assert!(res.is_err());
    }

    #[test]
    fn weak_release_version_is_rejected() {
        let res = "W/\"7\"".parse::<ReleaseVersion>();
        assert!(res.is_err());
    }

    #[test]
    fn negative_release_version_is_rejected() {
        let res = ReleaseVersion::try_from(-1);
        assert!(res.is_err());
    }

    #[test]
    fn page_url_points_at_the_position_addressed_reader_route() {
        let url = PageUrl::from((Uuid::from_u128(1), 4));

        assert_eq!(
            url.as_ref(),
            "/releases/00000000-0000-0000-0000-000000000001/pages/4"
        );
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
}
