use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    entity::{Email, Entity, Filter, Text, Timestamp},
    error::EntityError,
};

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub enum FeedbackKind {
    Report,
    Correction,
    General,
}

impl FromStr for FeedbackKind {
    type Err = EntityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Report" => Ok(Self::Report),
            "Correction" => Ok(Self::Correction),
            "General" => Ok(Self::General),
            other => Err(EntityError::InvalidFeedbackKind(other.to_owned())),
        }
    }
}

impl AsRef<str> for FeedbackKind {
    fn as_ref(&self) -> &str {
        match self {
            Self::Report => "Report",
            Self::Correction => "Correction",
            Self::General => "General",
        }
    }
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub enum FeedbackStatus {
    Open,
    Resolved,
    Dismissed,
}

impl FromStr for FeedbackStatus {
    type Err = EntityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Open" => Ok(Self::Open),
            "Resolved" => Ok(Self::Resolved),
            "Dismissed" => Ok(Self::Dismissed),
            other => Err(EntityError::InvalidFeedbackStatus(other.to_owned())),
        }
    }
}

impl AsRef<str> for FeedbackStatus {
    fn as_ref(&self) -> &str {
        match self {
            Self::Open => "Open",
            Self::Resolved => "Resolved",
            Self::Dismissed => "Dismissed",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Feedback {
    pub id: Uuid,
    pub kind: FeedbackKind,
    pub status: FeedbackStatus,
    pub email: Email,
    pub note: Text,
    pub book_id: Option<Uuid>,
    pub updated_at: Option<Timestamp>,
    pub created_at: Timestamp,
}

impl Entity for Feedback {
    const NAME: &'static str = "Feedback";
}

#[derive(Debug)]
pub struct FeedbackStatusUpdate {
    pub id: Uuid,
    pub status: FeedbackStatus,
    pub updated_at: Timestamp,
}

#[derive(Debug)]
pub struct FeedbackFilter {
    pub page: Filter,
    pub kind: Option<FeedbackKind>,
    pub status: Option<FeedbackStatus>,
}

#[cfg(test)]
mod tests {
    use crate::entity::{FeedbackKind, FeedbackStatus};

    #[test]
    fn every_feedback_kind_round_trips() {
        for s in ["Report", "Correction", "General"] {
            let kind: FeedbackKind = s.parse().unwrap();
            assert_eq!(kind.as_ref(), s);
        }
    }

    #[test]
    fn unknown_feedback_kind_is_rejected() {
        let res = "Complaint".parse::<FeedbackKind>();
        assert!(res.is_err());
    }

    #[test]
    fn every_feedback_status_round_trips() {
        for s in ["Open", "Resolved", "Dismissed"] {
            let status: FeedbackStatus = s.parse().unwrap();
            assert_eq!(status.as_ref(), s);
        }
    }

    #[test]
    fn unknown_feedback_status_is_rejected() {
        let res = "Closed".parse::<FeedbackStatus>();
        assert!(res.is_err());
    }
}
