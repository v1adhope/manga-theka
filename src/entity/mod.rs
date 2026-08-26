use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::EntityError;

mod book;
mod chapter;
mod content_rating;
mod creator;
mod filter;
mod image;
mod label;
mod language;
mod release;

pub use book::*;
pub use chapter::*;
pub use content_rating::*;
pub use creator::*;
pub use filter::*;
pub use image::*;
pub use label::*;
pub use language::*;
pub use release::*;

pub trait Entity {
    const NAME: &'static str;
}

pub const DEFAULT_IMAGE_MAX_BYTES: usize = 5 * 1024 * 1024;

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

    pub fn content_disposition(&self, id: Uuid) -> String {
        format!("inline; filename=\"{id}.{}\"", self.as_ref())
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

fn validate_name(s: String) -> Result<String, EntityError> {
    if s.trim().is_empty() {
        return Err(EntityError::NameIsEmptyOrWhitespace);
    }
    if s.chars().count() > 255 {
        return Err(EntityError::NameExceedsCharLimit(s));
    }
    Ok(s)
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::entity::ImageExtension;

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

    #[test]
    fn content_disposition_is_inline_with_the_extension_suffixed_filename() {
        let disposition = ImageExtension::Png.content_disposition(Uuid::from_u128(1));

        assert_eq!(
            disposition,
            "inline; filename=\"00000000-0000-0000-0000-000000000001.png\""
        );
    }
}
