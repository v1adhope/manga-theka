use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entity::{HexHash, Timestamp};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum BookSortField {
    CreatedAt,
    Name,
    PublicationYear,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(tag = "field", content = "value")]
pub enum BookSort {
    CreatedAt(Timestamp),
    Name(String),
    PublicationYear(i16),
}

impl BookSort {
    pub fn field(&self) -> BookSortField {
        match self {
            Self::CreatedAt(_) => BookSortField::CreatedAt,
            Self::Name(_) => BookSortField::Name,
            Self::PublicationYear(_) => BookSortField::PublicationYear,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BookCursor {
    pub id: Uuid,
    pub sort: BookSort,
    pub selection_hash: HexHash,
}

#[cfg(test)]
mod tests {
    use time::OffsetDateTime;

    use crate::entity::{BookSort, BookSortField, Timestamp};

    #[test]
    fn a_sort_value_belongs_to_one_field() {
        let cases = [
            (
                BookSort::CreatedAt(Timestamp::from(OffsetDateTime::UNIX_EPOCH)),
                BookSortField::CreatedAt,
            ),
            (
                BookSort::Name("Solo Leveling".to_owned()),
                BookSortField::Name,
            ),
            (
                BookSort::PublicationYear(2016),
                BookSortField::PublicationYear,
            ),
        ];

        for (value, field) in cases {
            assert_eq!(value.field(), field, "{value:?}");
        }
    }

    #[test]
    fn a_sort_value_belongs_to_no_other_field() {
        let cases = [
            (
                BookSort::CreatedAt(Timestamp::from(OffsetDateTime::UNIX_EPOCH)),
                BookSortField::Name,
            ),
            (
                BookSort::CreatedAt(Timestamp::from(OffsetDateTime::UNIX_EPOCH)),
                BookSortField::PublicationYear,
            ),
            (
                BookSort::Name("Solo Leveling".to_owned()),
                BookSortField::CreatedAt,
            ),
            (
                BookSort::Name("Solo Leveling".to_owned()),
                BookSortField::PublicationYear,
            ),
            (BookSort::PublicationYear(2016), BookSortField::CreatedAt),
            (BookSort::PublicationYear(2016), BookSortField::Name),
        ];

        for (value, other) in cases {
            assert_ne!(
                value.field(),
                other,
                "{value:?} must not pass a cursor check for {other:?}"
            );
        }
    }
}
