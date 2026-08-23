use crate::error::EntityError;

mod book;
mod chapter;
mod content_rating;
mod creator;
mod filter;
mod label;
mod language;
mod release;

pub use book::*;
pub use chapter::*;
pub use content_rating::*;
pub use creator::*;
pub use filter::*;
pub use label::*;
pub use language::*;
pub use release::*;

pub trait Entity {
    const NAME: &'static str;
}

const JPEG_SOI: [u8; 3] = [0xFF, 0xD8, 0xFF];
const PNG_SIGNATURE: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
const RIFF_MAGIC: [u8; 4] = *b"RIFF";
const WEBP_FORM_TYPE: [u8; 4] = *b"WEBP";

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
