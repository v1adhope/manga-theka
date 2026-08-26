use std::str::FromStr;

use bytes::Bytes;
use sanitize_filename::Options;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::EntityError;

pub const DEFAULT_IMAGE_MAX_BYTES: usize = 5 * 1024 * 1024;
pub const MAX_PARTS_PER_REQUEST: usize = 10;
pub const UPLOAD_MAX_BYTES: usize =
    MAX_PARTS_PER_REQUEST * (DEFAULT_IMAGE_MAX_BYTES + PART_HEADROOM_BYTES);

const PART_HEADROOM_BYTES: usize = 1024;

// Magic bytes for sniffing the real format, since client-supplied extension/content-type can't be trusted.
const JPEG_SOI: [u8; 3] = [0xFF, 0xD8, 0xFF];
const PNG_SIGNATURE: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
const RIFF_MAGIC: [u8; 4] = *b"RIFF";
const WEBP_FORM_TYPE: [u8; 4] = *b"WEBP";

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub enum ImageExtension {
    Jpg,
    Png,
    Webp,
}

impl ImageExtension {
    pub fn content_type(&self) -> &str {
        match self {
            Self::Jpg => "image/jpeg",
            Self::Png => "image/png",
            Self::Webp => "image/webp",
        }
    }
}

impl TryFrom<&[u8]> for ImageExtension {
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

impl FromStr for ImageExtension {
    type Err = EntityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "jpg" => Ok(Self::Jpg),
            "png" => Ok(Self::Png),
            "webp" => Ok(Self::Webp),
            other => Err(EntityError::InvalidImageExtension(other.to_owned())),
        }
    }
}

impl AsRef<str> for ImageExtension {
    fn as_ref(&self) -> &str {
        match self {
            Self::Jpg => "jpg",
            Self::Png => "png",
            Self::Webp => "webp",
        }
    }
}

fn is_jpeg(bytes: &[u8]) -> bool {
    bytes.starts_with(&JPEG_SOI)
}

fn is_png(bytes: &[u8]) -> bool {
    bytes.starts_with(&PNG_SIGNATURE)
}

fn is_webp(bytes: &[u8]) -> bool {
    bytes.len() >= 12 && bytes.starts_with(&RIFF_MAGIC) && bytes[8..12] == WEBP_FORM_TYPE
}

#[derive(Debug, Clone)]
pub struct ImageContent(Bytes);

impl TryFrom<Bytes> for ImageContent {
    type Error = EntityError;

    fn try_from(bytes: Bytes) -> Result<Self, Self::Error> {
        if bytes.len() > DEFAULT_IMAGE_MAX_BYTES {
            return Err(EntityError::ImageExceedsByteLimit(DEFAULT_IMAGE_MAX_BYTES));
        }

        Ok(Self(bytes))
    }
}

impl AsRef<[u8]> for ImageContent {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl ImageContent {
    pub fn to_bytes(&self) -> Bytes {
        self.0.clone()
    }

    pub fn ensure_incoming_capacity(current: usize, incoming: usize) -> Result<(), EntityError> {
        if current + incoming > DEFAULT_IMAGE_MAX_BYTES {
            return Err(EntityError::ImageExceedsByteLimit(DEFAULT_IMAGE_MAX_BYTES));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FileName(String);

impl TryFrom<String> for FileName {
    type Error = EntityError;

    fn try_from(name: String) -> Result<Self, Self::Error> {
        const OPTS: Options<'static> = Options {
            windows: true,
            truncate: true,
            replacement: "",
        };

        let sanitized = sanitize_filename::sanitize_with_options(name, OPTS);

        if sanitized.is_empty() {
            return Err(EntityError::NameIsEmptyOrWhitespace);
        }

        Ok(Self(sanitized))
    }
}

impl AsRef<str> for FileName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl FileName {
    pub fn from_client_provided(name: Option<String>) -> Result<Self, EntityError> {
        Self::try_from(name.ok_or(EntityError::FileNameMissing)?)
    }
}

#[derive(Debug)]
pub struct Image {
    pub id: Uuid,
    pub extension: ImageExtension,
    pub content: ImageContent,
    pub file_name: FileName,
}

impl Image {
    // Uses the client-supplied file_name as-is; the extension here is
    // cosmetic (download hint only), so we deliberately don't override it
    // with the sniffed `extension`, which is what actually decides storage
    // and content type.
    pub fn content_disposition(&self) -> String {
        format!("inline; filename=\"{}\"", self.file_name.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use uuid::Uuid;

    use crate::entity::{DEFAULT_IMAGE_MAX_BYTES, FileName, Image, ImageContent, ImageExtension};

    use super::PNG_SIGNATURE;

    #[test]
    fn content_at_the_byte_ceiling_is_valid() {
        let bytes = Bytes::from(vec![0u8; DEFAULT_IMAGE_MAX_BYTES]);
        assert!(ImageContent::try_from(bytes).is_ok());
    }

    #[test]
    fn content_over_the_byte_ceiling_is_rejected() {
        let bytes = Bytes::from(vec![0u8; DEFAULT_IMAGE_MAX_BYTES + 1]);
        assert!(ImageContent::try_from(bytes).is_err());
    }

    #[test]
    fn incoming_capacity_at_the_ceiling_is_valid() {
        let res = ImageContent::ensure_incoming_capacity(DEFAULT_IMAGE_MAX_BYTES - 1, 1);
        assert!(res.is_ok());
    }

    #[test]
    fn incoming_capacity_over_the_ceiling_is_rejected() {
        let res = ImageContent::ensure_incoming_capacity(DEFAULT_IMAGE_MAX_BYTES, 1);
        assert!(res.is_err());
    }

    fn image(content: ImageContent, file_name: FileName) -> Image {
        Image {
            id: Uuid::from_u128(1),
            extension: ImageExtension::Png,
            content,
            file_name,
        }
    }

    #[test]
    fn content_disposition_uses_the_client_file_name_when_present() {
        let content = ImageContent::try_from(Bytes::from(PNG_SIGNATURE.to_vec())).unwrap();
        let file_name = FileName::try_from("cover art.png".to_owned()).unwrap();
        let image = image(content, file_name);

        assert_eq!(
            image.content_disposition(),
            "inline; filename=\"cover art.png\""
        );
    }

    #[test]
    fn content_disposition_strips_quotes_backslashes_and_control_characters() {
        let content = ImageContent::try_from(Bytes::from(PNG_SIGNATURE.to_vec())).unwrap();
        let file_name = FileName::try_from("evil\"\\\r\nname.png".to_owned()).unwrap();
        let image = image(content, file_name);

        assert_eq!(
            image.content_disposition(),
            "inline; filename=\"evilname.png\""
        );
    }

    #[test]
    fn file_name_is_rejected_when_it_sanitizes_to_empty() {
        assert!(FileName::try_from("\"\\".to_owned()).is_err());
    }

    #[test]
    fn client_provided_file_name_is_accepted_when_present() {
        let res = FileName::from_client_provided(Some("cover.png".to_owned()));
        assert!(res.is_ok());
    }

    #[test]
    fn client_provided_file_name_is_rejected_when_absent() {
        let res = FileName::from_client_provided(None);
        assert!(res.is_err());
    }

    #[test]
    fn content_disposition_truncates_a_client_file_name_over_the_char_limit() {
        let content = ImageContent::try_from(Bytes::from(PNG_SIGNATURE.to_vec())).unwrap();
        let long_name = "a".repeat(300);
        let file_name = FileName::try_from(long_name).unwrap();
        let image = image(content, file_name);

        let expected = format!("inline; filename=\"{}\"", "a".repeat(255));
        assert_eq!(image.content_disposition(), expected);
    }

    #[test]
    fn jpeg_magic_bytes_are_sniffed() {
        let res = ImageExtension::try_from([0xFF, 0xD8, 0xFF, 0xE0].as_slice());
        assert_eq!(res.unwrap(), ImageExtension::Jpg);
    }

    #[test]
    fn png_magic_bytes_are_sniffed() {
        let res =
            ImageExtension::try_from([0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A].as_slice());
        assert_eq!(res.unwrap(), ImageExtension::Png);
    }

    #[test]
    fn webp_magic_bytes_are_sniffed() {
        let res = ImageExtension::try_from(b"RIFF\x34\x00\x00\x00WEBPVP8 ".as_slice());
        assert_eq!(res.unwrap(), ImageExtension::Webp);
    }

    #[test]
    fn riff_without_webp_is_rejected() {
        let res = ImageExtension::try_from(b"RIFF\x34\x00\x00\x00WAVEfmt ".as_slice());
        assert!(res.is_err());
    }

    #[test]
    fn truncated_riff_header_is_rejected() {
        let res = ImageExtension::try_from(b"RIFF\x34\x00\x00".as_slice());
        assert!(res.is_err());
    }

    #[test]
    fn empty_body_is_rejected() {
        let res = ImageExtension::try_from([].as_slice());
        assert!(res.is_err());
    }

    #[test]
    fn gif_magic_bytes_are_rejected() {
        let res = ImageExtension::try_from(b"GIF89a".as_slice());
        assert!(res.is_err());
    }
}
