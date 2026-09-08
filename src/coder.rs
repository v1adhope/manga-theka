use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    entity::{BookCursor, BookSort},
    error::{CoderError, EntityError},
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BookCursorDecode {
    id: Uuid,
    sort: BookSort,
    selection_hash: String,
}

impl TryFrom<BookCursorDecode> for BookCursor {
    type Error = EntityError;

    fn try_from(decode: BookCursorDecode) -> Result<Self, Self::Error> {
        Ok(Self {
            id: decode.id,
            sort: decode.sort,
            selection_hash: decode.selection_hash.try_into()?,
        })
    }
}

#[derive(Debug)]
pub struct Coder;

impl Coder {
    pub fn encode(cursor: BookCursor) -> Result<String, CoderError> {
        let json = serde_json::to_vec(&cursor)
            .map_err(CoderError::Serialize)
            .inspect_err(CoderError::log_internal)?;

        Ok(URL_SAFE_NO_PAD.encode(json))
    }

    pub fn decode(raw: &str) -> Result<BookCursor, CoderError> {
        let json = URL_SAFE_NO_PAD.decode(raw).map_err(CoderError::Base64)?;
        let decode: BookCursorDecode =
            serde_json::from_slice(&json).map_err(CoderError::Deserialize)?;

        BookCursor::try_from(decode).map_err(CoderError::Cursor)
    }
}

#[cfg(test)]
mod tests {
    use time::OffsetDateTime;
    use uuid::Uuid;

    use crate::{
        coder::Coder,
        entity::{BookCursor, BookSort, ShortHexHash, Timestamp},
    };

    fn hash() -> ShortHexHash {
        ShortHexHash::try_from("0123456789abcdef".to_owned()).unwrap()
    }

    fn every_sort_value() -> [BookSort; 3] {
        [
            BookSort::CreatedAt(Timestamp::from(
                OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap(),
            )),
            BookSort::Name("Solo Leveling".to_owned()),
            BookSort::PublicationYear(2016),
        ]
    }

    #[test]
    fn every_sort_field_round_trips_through_a_cursor() {
        for value in every_sort_value() {
            let cursor = BookCursor {
                id: Uuid::now_v7(),
                sort: value.clone(),
                selection_hash: hash(),
            };

            let encoded = Coder::encode(cursor.clone()).unwrap();
            let decoded = Coder::decode(&encoded).unwrap();

            assert_eq!(decoded, cursor, "{value:?}");
        }
    }

    #[test]
    fn an_encoded_cursor_is_url_safe_and_unpadded() {
        let cursor = BookCursor {
            id: Uuid::now_v7(),
            sort: BookSort::Name("a/b+c d".to_owned()),
            selection_hash: hash(),
        };

        let encoded = Coder::encode(cursor).unwrap();
        let is_url_safe = encoded
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');

        assert!(
            is_url_safe,
            "{encoded} must survive a query string untouched"
        );
    }

    #[test]
    fn a_sort_value_of_the_wrong_shape_is_rejected() {
        let raw = serde_json::json!({
            "id": Uuid::now_v7(),
            "sort": { "field": "PublicationYear", "value": "not a year" },
            "selectionHash": "0123456789abcdef",
        })
        .to_string();
        let encoded =
            base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, raw);

        assert!(Coder::decode(&encoded).is_err());
    }

    #[test]
    fn a_cursor_carrying_a_malformed_hash_is_rejected() {
        let raw = serde_json::json!({
            "id": Uuid::now_v7(),
            "sort": { "field": "PublicationYear", "value": 2016 },
            "selectionHash": "nope",
        })
        .to_string();
        let encoded =
            base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, raw);

        assert!(Coder::decode(&encoded).is_err());
    }

    #[test]
    fn a_non_base64_cursor_is_rejected() {
        assert!(Coder::decode("not base64!!").is_err());
    }

    #[test]
    fn a_truncated_cursor_is_rejected() {
        let cursor = BookCursor {
            id: Uuid::now_v7(),
            sort: every_sort_value()[0].clone(),
            selection_hash: hash(),
        };
        let encoded = Coder::encode(cursor).unwrap();

        assert!(Coder::decode(&encoded[..encoded.len() / 2]).is_err());
    }

    #[test]
    fn an_empty_cursor_is_rejected() {
        assert!(Coder::decode("").is_err());
    }
}
