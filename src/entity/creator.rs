use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

use crate::{
    entity::{Entity, Name, Timestamp},
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
    pub created_at: Timestamp,
}

impl Entity for Creator {
    const NAME: &'static str = "Creator";
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatorQuery {
    pub id: Uuid,
    pub first_name: Name,
    pub last_name: Name,
    pub roles: Vec<CreatorRole>,
    pub created_at: Timestamp,
}
