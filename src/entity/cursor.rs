use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use crate::error::EntityError;

const CURSOR_VERSION: u8 = 1;
const FILTER_HASH_HEX_LEN: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BookSortField {
    #[default]
    CreatedAt,
    Name,
    PublicationYear,
}

impl BookSortField {
    pub fn column(self) -> &'static str {
        match self {
            Self::CreatedAt => "created_at",
            Self::Name => "name",
            Self::PublicationYear => "publication_year",
        }
    }

    fn wire(self) -> &'static str {
        match self {
            Self::CreatedAt => "createdAt",
            Self::Name => "name",
            Self::PublicationYear => "publicationYear",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BookSortValue {
    CreatedAt(OffsetDateTime),
    Name(String),
    PublicationYear(i16),
}

impl BookSortValue {
    fn field(&self) -> BookSortField {
        match self {
            Self::CreatedAt(_) => BookSortField::CreatedAt,
            Self::Name(_) => BookSortField::Name,
            Self::PublicationYear(_) => BookSortField::PublicationYear,
        }
    }

    fn encode(&self) -> Result<Value, EntityError> {
        match self {
            Self::CreatedAt(at) => at
                .format(&Rfc3339)
                .map(Value::String)
                .map_err(|_| EntityError::CursorIsMalformed),
            Self::Name(name) => Ok(Value::String(name.clone())),
            Self::PublicationYear(year) => Ok(Value::Number((*year).into())),
        }
    }

    fn decode(field: BookSortField, value: &Value) -> Result<Self, EntityError> {
        match field {
            BookSortField::CreatedAt => value
                .as_str()
                .and_then(|s| OffsetDateTime::parse(s, &Rfc3339).ok())
                .map(Self::CreatedAt),
            BookSortField::Name => value.as_str().map(|s| Self::Name(s.to_owned())),
            BookSortField::PublicationYear => value
                .as_i64()
                .and_then(|n| i16::try_from(n).ok())
                .map(Self::PublicationYear),
        }
        .ok_or(EntityError::CursorIsMalformed)
    }
}

/// The leading 8 bytes of a blake3 hash of the filter a cursor was minted under, hex-rendered.
///
/// Unkeyed, so it is an accident detector rather than a signature: it catches a cursor carried
/// across a changed facet, which would otherwise page through quietly wrong results.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterHash(String);

impl FilterHash {
    pub fn of(canonical: &str) -> Self {
        let hex = blake3::hash(canonical.as_bytes()).to_hex();

        Self(hex[..FILTER_HASH_HEX_LEN].to_owned())
    }
}

#[derive(Deserialize, Serialize)]
struct CursorPayload {
    v: u8,
    f: BookSortField,
    s: Value,
    i: Uuid,
    h: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BookCursor {
    pub value: BookSortValue,
    pub id: Uuid,
    pub filter_hash: FilterHash,
}

impl BookCursor {
    pub fn new(value: BookSortValue, id: Uuid, filter_hash: FilterHash) -> Self {
        Self {
            value,
            id,
            filter_hash,
        }
    }

    pub fn encode(&self) -> Result<String, EntityError> {
        let payload = CursorPayload {
            v: CURSOR_VERSION,
            f: self.value.field(),
            s: self.value.encode()?,
            i: self.id,
            h: self.filter_hash.0.clone(),
        };
        let json = serde_json::to_vec(&payload).map_err(|_| EntityError::CursorIsMalformed)?;

        Ok(URL_SAFE_NO_PAD.encode(json))
    }

    pub fn decode(raw: &str) -> Result<Self, EntityError> {
        let json = URL_SAFE_NO_PAD
            .decode(raw)
            .map_err(|_| EntityError::CursorIsMalformed)?;
        let payload: CursorPayload =
            serde_json::from_slice(&json).map_err(|_| EntityError::CursorIsMalformed)?;

        if payload.v != CURSOR_VERSION {
            return Err(EntityError::CursorIsMalformed);
        }

        Ok(Self {
            value: BookSortValue::decode(payload.f, &payload.s)?,
            id: payload.i,
            filter_hash: FilterHash(payload.h),
        })
    }

    pub fn field(&self) -> BookSortField {
        self.value.field()
    }
}

/// Renders one filter parameter into the canonical string a [`FilterHash`] is taken over.
///
/// Every facet contributes a line whether or not it was supplied, so an absent facet and an
/// empty one hash alike and no two different filters can collide by omission.
pub struct CanonicalFilter(String);

impl CanonicalFilter {
    pub fn new() -> Self {
        Self(String::new())
    }

    pub fn field(mut self, key: &str, value: &str) -> Self {
        self.0.push_str(key);
        self.0.push('=');
        self.0.push_str(value);
        self.0.push('\n');

        self
    }

    pub fn sort(self, key: &str, field: BookSortField) -> Self {
        self.field(key, field.wire())
    }

    /// The values must already be deduplicated and ordered; every facet is a set on the wire,
    /// so parameter order must not change the hash.
    pub fn set(self, key: &str, values: impl IntoIterator<Item = String>) -> Self {
        let joined: Vec<String> = values.into_iter().collect();

        self.field(key, &joined.join(","))
    }

    pub fn hash(&self) -> FilterHash {
        FilterHash::of(&self.0)
    }
}

impl Default for CanonicalFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use time::OffsetDateTime;
    use uuid::Uuid;

    use crate::entity::{BookCursor, BookSortField, BookSortValue, CanonicalFilter, FilterHash};

    fn hash() -> FilterHash {
        FilterHash::of("sort=createdAt\norder=Desc\n")
    }

    fn every_sort_value() -> [BookSortValue; 3] {
        [
            BookSortValue::CreatedAt(OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap()),
            BookSortValue::Name("Solo Leveling".to_owned()),
            BookSortValue::PublicationYear(2016),
        ]
    }

    #[test]
    fn every_sort_field_round_trips_through_a_cursor() {
        for value in every_sort_value() {
            let cursor = BookCursor::new(value.clone(), Uuid::now_v7(), hash());

            let decoded = BookCursor::decode(&cursor.encode().unwrap()).unwrap();

            assert_eq!(decoded, cursor, "{value:?}");
        }
    }

    #[test]
    fn a_cursor_carries_the_field_its_value_belongs_to() {
        let pairs = [
            (BookSortField::CreatedAt, 0),
            (BookSortField::Name, 1),
            (BookSortField::PublicationYear, 2),
        ];

        for (field, i) in pairs {
            let cursor = BookCursor::new(every_sort_value()[i].clone(), Uuid::now_v7(), hash());

            assert_eq!(cursor.field(), field);
        }
    }

    #[test]
    fn an_encoded_cursor_is_url_safe_and_unpadded() {
        let cursor = BookCursor::new(
            BookSortValue::Name("a/b+c d".to_owned()),
            Uuid::now_v7(),
            hash(),
        );

        let encoded = cursor.encode().unwrap();

        assert!(
            encoded
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
            "{encoded} must survive a query string untouched"
        );
    }

    #[test]
    fn a_cursor_from_another_version_is_rejected() {
        let raw = serde_json::json!({
            "v": 2,
            "f": "createdAt",
            "s": "2023-11-14T22:13:20Z",
            "i": Uuid::now_v7(),
            "h": "0123456789abcdef",
        })
        .to_string();
        let encoded =
            base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, raw);

        assert!(BookCursor::decode(&encoded).is_err());
    }

    #[test]
    fn a_sort_value_of_the_wrong_shape_is_rejected() {
        let raw = serde_json::json!({
            "v": 1,
            "f": "publicationYear",
            "s": "not a year",
            "i": Uuid::now_v7(),
            "h": "0123456789abcdef",
        })
        .to_string();
        let encoded =
            base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, raw);

        assert!(BookCursor::decode(&encoded).is_err());
    }

    #[test]
    fn a_non_base64_cursor_is_rejected() {
        assert!(BookCursor::decode("not base64!!").is_err());
    }

    #[test]
    fn a_truncated_cursor_is_rejected() {
        let cursor = BookCursor::new(every_sort_value()[0].clone(), Uuid::now_v7(), hash());
        let encoded = cursor.encode().unwrap();

        assert!(BookCursor::decode(&encoded[..encoded.len() / 2]).is_err());
    }

    #[test]
    fn an_empty_cursor_is_rejected() {
        assert!(BookCursor::decode("").is_err());
    }

    #[test]
    fn the_same_filter_hashes_the_same() {
        let one = CanonicalFilter::new()
            .sort("sort", BookSortField::Name)
            .set("labels", ["a".to_owned(), "b".to_owned()]);
        let other = CanonicalFilter::new()
            .sort("sort", BookSortField::Name)
            .set("labels", ["a".to_owned(), "b".to_owned()]);

        assert_eq!(one.hash(), other.hash());
    }

    #[test]
    fn a_changed_facet_changes_the_hash() {
        let one = CanonicalFilter::new().set("labels", ["a".to_owned()]);
        let other = CanonicalFilter::new().set("labels", ["b".to_owned()]);

        assert_ne!(one.hash(), other.hash());
    }

    #[test]
    fn an_absent_facet_and_an_empty_one_are_the_same_filter() {
        let absent = CanonicalFilter::new().set("labels", []);
        let empty = CanonicalFilter::new().set("labels", Vec::new());

        assert_eq!(absent.hash(), empty.hash());
    }

    #[test]
    fn a_value_moved_between_facets_changes_the_hash() {
        let one = CanonicalFilter::new()
            .set("labels", ["a".to_owned()])
            .set("excludedLabels", []);
        let other = CanonicalFilter::new()
            .set("labels", [])
            .set("excludedLabels", ["a".to_owned()]);

        assert_ne!(
            one.hash(),
            other.hash(),
            "a facet boundary must not be crossable by concatenation"
        );
    }

    #[test]
    fn a_filter_hash_is_stable_across_runs() {
        assert_eq!(
            FilterHash::of("sort=createdAt\norder=Desc\n"),
            FilterHash::of("sort=createdAt\norder=Desc\n"),
        );
    }
}
