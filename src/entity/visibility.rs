use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{entity::Text, error::EntityError};

pub const SUBMITTED_NOTE: &str = "Your submission has been received and is currently under review. \
                                  We'll process it within 3 business days - thanks for your patience!";

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
pub enum BookVisibility {
    Draft,
    PendingReview,
    Listed,
    Rejected,
    Hidden,
}

impl FromStr for BookVisibility {
    type Err = EntityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Draft" => Ok(Self::Draft),
            "PendingReview" => Ok(Self::PendingReview),
            "Listed" => Ok(Self::Listed),
            "Rejected" => Ok(Self::Rejected),
            "Hidden" => Ok(Self::Hidden),
            other => Err(EntityError::InvalidBookVisibility(other.to_owned())),
        }
    }
}

impl AsRef<str> for BookVisibility {
    fn as_ref(&self) -> &str {
        match self {
            Self::Draft => "Draft",
            Self::PendingReview => "PendingReview",
            Self::Listed => "Listed",
            Self::Rejected => "Rejected",
            Self::Hidden => "Hidden",
        }
    }
}

impl fmt::Display for BookVisibility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_ref())
    }
}

#[derive(Debug)]
pub struct VisibilityTransition {
    pub id: Uuid,
    pub visibility: BookVisibility,
    pub note: Option<Text>,
    pub now: OffsetDateTime,
}

#[derive(Debug)]
pub struct BookVisibilityUpdate {
    pub id: Uuid,
    pub from: BookVisibility,
    pub to: BookVisibility,
    pub note: Option<Text>,
    pub submitted_at: Option<OffsetDateTime>,
}

impl TryFrom<(BookVisibility, VisibilityTransition)> for BookVisibilityUpdate {
    type Error = EntityError;

    fn try_from(ctx: (BookVisibility, VisibilityTransition)) -> Result<Self, Self::Error> {
        use BookVisibility::{Draft, Hidden, Listed, PendingReview, Rejected};

        let (from, item) = ctx;
        let VisibilityTransition {
            id,
            visibility,
            note,
            now,
        } = item;

        if from != visibility {
            match (from, visibility) {
                (Draft, PendingReview | Rejected | Hidden)
                | (PendingReview, Draft | Listed | Rejected | Hidden)
                | (Listed, Hidden | Rejected)
                | (Hidden, Listed | Rejected) => {}
                _ => return Err(EntityError::IllegalVisibilityTransition(from, visibility)),
            }
        }

        let (note, submitted_at) = match visibility {
            PendingReview => (
                Some(Text::try_from(SUBMITTED_NOTE.to_owned())?),
                (from != visibility).then_some(now),
            ),
            Listed => (note, None),
            Draft | Rejected | Hidden => (
                Some(note.ok_or(EntityError::BookNoteRequired(visibility))?),
                None,
            ),
        };

        Ok(Self {
            id,
            from,
            to: visibility,
            note,
            submitted_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use time::OffsetDateTime;
    use uuid::Uuid;

    use crate::entity::{
        BookVisibility, BookVisibilityUpdate, SUBMITTED_NOTE, Text, VisibilityTransition,
    };

    const EVERY_VISIBILITY: [BookVisibility; 5] = [
        BookVisibility::Draft,
        BookVisibility::PendingReview,
        BookVisibility::Listed,
        BookVisibility::Rejected,
        BookVisibility::Hidden,
    ];

    fn transition(visibility: BookVisibility, note: Option<&str>) -> VisibilityTransition {
        VisibilityTransition {
            id: Uuid::now_v7(),
            visibility,
            note: note.map(|n| Text::try_from(n.to_owned()).unwrap()),
            now: OffsetDateTime::now_utc(),
        }
    }

    fn is_legal(from: BookVisibility, to: BookVisibility) -> bool {
        use BookVisibility::{Draft, Hidden, Listed, PendingReview, Rejected};

        matches!(
            (from, to),
            (Draft, Draft | PendingReview | Rejected | Hidden)
                | (
                    PendingReview,
                    PendingReview | Draft | Listed | Rejected | Hidden
                )
                | (Listed, Listed | Hidden | Rejected)
                | (Hidden, Hidden | Listed | Rejected)
                | (Rejected, Rejected)
        )
    }

    #[test]
    fn every_book_visibility_round_trips() {
        for visibility in EVERY_VISIBILITY {
            let parsed: BookVisibility = visibility.as_ref().parse().unwrap();
            assert_eq!(parsed, visibility);
        }
    }

    #[test]
    fn unknown_book_visibility_is_rejected() {
        let res = "Published".parse::<BookVisibility>();
        assert!(res.is_err());
    }

    #[test]
    fn only_the_tabled_transitions_are_accepted() {
        for from in EVERY_VISIBILITY {
            for to in EVERY_VISIBILITY {
                let res = BookVisibilityUpdate::try_from((from, transition(to, Some("why"))));

                assert_eq!(
                    res.is_ok(),
                    is_legal(from, to),
                    "{from} -> {to} was judged wrongly"
                );
            }
        }
    }

    #[test]
    fn submitting_stamps_the_code_authored_note() {
        let update = BookVisibilityUpdate::try_from((
            BookVisibility::Draft,
            transition(BookVisibility::PendingReview, Some("mine")),
        ))
        .unwrap();

        assert_eq!(update.note.unwrap().as_ref(), SUBMITTED_NOTE);
    }

    #[test]
    fn submitting_stamps_the_submission_time() {
        let update = BookVisibilityUpdate::try_from((
            BookVisibility::Draft,
            transition(BookVisibility::PendingReview, None),
        ))
        .unwrap();

        assert!(update.submitted_at.is_some());
    }

    #[test]
    fn a_book_already_in_review_is_not_restamped() {
        let update = BookVisibilityUpdate::try_from((
            BookVisibility::PendingReview,
            transition(BookVisibility::PendingReview, None),
        ))
        .unwrap();

        assert_eq!(update.submitted_at, None);
    }

    #[test]
    fn listing_without_a_note_clears_it() {
        let update = BookVisibilityUpdate::try_from((
            BookVisibility::PendingReview,
            transition(BookVisibility::Listed, None),
        ))
        .unwrap();

        assert_eq!(update.note, None);
    }

    #[test]
    fn listing_with_a_note_replaces_it() {
        let update = BookVisibilityUpdate::try_from((
            BookVisibility::PendingReview,
            transition(BookVisibility::Listed, Some("welcome")),
        ))
        .unwrap();

        assert_eq!(update.note.unwrap().as_ref(), "welcome");
    }

    #[test]
    fn moves_that_explain_themselves_require_a_note() {
        for to in [
            BookVisibility::Draft,
            BookVisibility::Rejected,
            BookVisibility::Hidden,
        ] {
            let res = BookVisibilityUpdate::try_from((
                BookVisibility::PendingReview,
                transition(to, None),
            ));

            assert!(res.is_err(), "{to} without a note must be rejected");
        }
    }

    #[test]
    fn a_self_transition_takes_its_own_note_rule() {
        let cleared = BookVisibilityUpdate::try_from((
            BookVisibility::Listed,
            transition(BookVisibility::Listed, None),
        ))
        .unwrap();
        let refused = BookVisibilityUpdate::try_from((
            BookVisibility::Hidden,
            transition(BookVisibility::Hidden, None),
        ));

        assert_eq!(cleared.note, None);
        assert!(refused.is_err(), "a hidden book still owes a reason");
    }

    #[test]
    fn a_self_transition_never_stamps_a_submission_time() {
        for visibility in EVERY_VISIBILITY {
            let Ok(update) =
                BookVisibilityUpdate::try_from((visibility, transition(visibility, Some("same"))))
            else {
                continue;
            };

            assert_eq!(update.submitted_at, None, "{visibility} restamped itself");
        }
    }
}
