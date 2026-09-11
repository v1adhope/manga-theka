use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime, UtcOffset};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct Timestamp(#[serde(with = "time::serde::rfc3339")] OffsetDateTime);

impl From<OffsetDateTime> for Timestamp {
    fn from(at: OffsetDateTime) -> Self {
        Self(at.to_offset(UtcOffset::UTC))
    }
}

impl Timestamp {
    pub const UNIX_EPOCH: Self = Self(OffsetDateTime::UNIX_EPOCH);

    pub fn now() -> Self {
        OffsetDateTime::now_utc().into()
    }

    pub fn into_inner(self) -> OffsetDateTime {
        self.0
    }

    pub fn unix_timestamp(self) -> i64 {
        self.0.unix_timestamp()
    }
}

impl std::ops::Sub for Timestamp {
    type Output = Duration;

    fn sub(self, rhs: Self) -> Duration {
        self.0 - rhs.0
    }
}

impl std::ops::Sub<Duration> for Timestamp {
    type Output = Timestamp;

    fn sub(self, rhs: Duration) -> Timestamp {
        Self(self.0 - rhs)
    }
}

impl std::ops::Add<Duration> for Timestamp {
    type Output = Timestamp;

    fn add(self, rhs: Duration) -> Timestamp {
        Self(self.0 + rhs)
    }
}

#[cfg(test)]
mod tests {
    use time::Duration;

    use super::Timestamp;

    #[test]
    fn now_is_close_to_the_wall_clock() {
        let before = time::OffsetDateTime::now_utc();
        let now = Timestamp::now();

        assert!(now.into_inner() - before < Duration::seconds(1));
    }

    #[test]
    fn two_spellings_of_one_instant_are_equal() {
        let utc = Timestamp::UNIX_EPOCH.into_inner() + Duration::hours(12);
        let shifted = utc.to_offset(time::UtcOffset::from_hms(2, 0, 0).unwrap());

        assert_eq!(Timestamp::from(utc), Timestamp::from(shifted));
    }

    #[test]
    fn subtracting_two_timestamps_yields_their_elapsed_duration() {
        let earlier = Timestamp::UNIX_EPOCH;
        let later = earlier + Duration::hours(2);

        assert_eq!(later - earlier, Duration::hours(2));
    }

    #[test]
    fn subtracting_a_duration_moves_the_timestamp_back() {
        let now = Timestamp::now();

        assert_eq!(now - Duration::hours(1), now - Duration::minutes(60));
    }

    #[test]
    fn a_later_timestamp_sorts_after_an_earlier_one() {
        let earlier = Timestamp::UNIX_EPOCH;
        let later = earlier + Duration::days(1);

        assert!(later > earlier);
    }

    #[test]
    fn unix_timestamp_matches_the_inner_offset_date_time() {
        let at = Timestamp::UNIX_EPOCH + Duration::seconds(42);

        assert_eq!(at.unix_timestamp(), 42);
    }

    #[test]
    fn it_serializes_as_rfc3339() {
        let at = Timestamp::UNIX_EPOCH + Duration::hours(1);

        let json = serde_json::to_string(&at).unwrap();

        assert_eq!(json, "\"1970-01-01T01:00:00Z\"");
    }

    #[test]
    fn it_round_trips_through_json() {
        let at = Timestamp::now();

        let json = serde_json::to_string(&at).unwrap();
        let back: Timestamp = serde_json::from_str(&json).unwrap();

        assert_eq!(back, at);
    }
}
