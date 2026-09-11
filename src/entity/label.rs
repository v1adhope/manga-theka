use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

use crate::error::EntityError;

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum LabelKind {
    Genre,
    Theme,
    Presentation,
}

impl FromStr for LabelKind {
    type Err = EntityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Genre" => Ok(Self::Genre),
            "Theme" => Ok(Self::Theme),
            "Presentation" => Ok(Self::Presentation),
            other => Err(EntityError::InvalidLabelKind(other.to_owned())),
        }
    }
}

impl AsRef<str> for LabelKind {
    fn as_ref(&self) -> &str {
        match self {
            Self::Genre => "Genre",
            Self::Theme => "Theme",
            Self::Presentation => "Presentation",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Label {
    pub id: Uuid,
    pub name: String,
    pub kind: LabelKind,
}

#[cfg(test)]
mod tests {
    use crate::entity::LabelKind;

    #[test]
    fn every_label_kind_round_trips() {
        for s in ["Genre", "Theme", "Presentation"] {
            let kind: LabelKind = s.parse().unwrap();
            assert_eq!(kind.as_ref(), s);
        }
    }

    #[test]
    fn unknown_label_kind_is_rejected() {
        let res = "Tag".parse::<LabelKind>();
        assert!(res.is_err());
    }
}
