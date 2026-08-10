use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

use crate::error::EntityError;

#[derive(Debug, Deserialize, Serialize)]
pub enum LabelKind {
    Genre,
    Tag,
}

impl FromStr for LabelKind {
    type Err = EntityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Genre" => Ok(Self::Genre),
            "Tag" => Ok(Self::Tag),
            other => Err(EntityError::InvalidLabelKind(other.to_owned())),
        }
    }
}

impl AsRef<str> for LabelKind {
    fn as_ref(&self) -> &str {
        match self {
            Self::Genre => "Genre",
            Self::Tag => "Tag",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Label {
    pub id: Uuid,
    pub name: String,
    pub kind: LabelKind,
}
