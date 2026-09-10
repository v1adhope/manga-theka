use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, UtcOffset};

// TODO: replace all OffsetDateTime to Timestamp
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct Timestamp(#[serde(with = "time::serde::rfc3339")] OffsetDateTime);

impl From<OffsetDateTime> for Timestamp {
    fn from(at: OffsetDateTime) -> Self {
        Self(at.to_offset(UtcOffset::UTC))
    }
}

impl Timestamp {
    pub fn into_inner(self) -> OffsetDateTime {
        self.0
    }
}
