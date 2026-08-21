use crate::error::EntityError;

mod book;
mod chapter;
mod content_rating;
mod creator;
mod filter;
mod label;
mod language;

pub use book::*;
pub use chapter::*;
pub use content_rating::*;
pub use creator::*;
pub use filter::*;
pub use label::*;
pub use language::*;

pub trait Entity {
    const NAME: &'static str;
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
