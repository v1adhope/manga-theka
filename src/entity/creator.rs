use serde::{Deserialize, Serialize};
use std::str::FromStr;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    entity::{Entity, Name},
    error::EntityError,
};

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub enum CreatorRole {
    Artist,
    Author,
}

impl FromStr for CreatorRole {
    type Err = EntityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Artist" => Ok(Self::Artist),
            "Author" => Ok(Self::Author),
            other => Err(EntityError::InvalidCreatorRole(other.to_owned())),
        }
    }
}

impl AsRef<str> for CreatorRole {
    fn as_ref(&self) -> &str {
        match self {
            Self::Artist => "Artist",
            Self::Author => "Author",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Creator {
    pub id: Uuid,
    pub first_name: Name,
    pub last_name: Name,
    pub role: CreatorRole,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

impl Entity for Creator {
    const NAME: &'static str = "Creator";
}
