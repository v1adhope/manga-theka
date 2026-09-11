use std::sync::atomic::{AtomicU32, Ordering};

use bytes::Bytes;
use ed25519_dalek::{
    SigningKey,
    pkcs8::{EncodePrivateKey, EncodePublicKey, spki::der::pem::LineEnding},
};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::{
    config,
    entity::{
        AlternativeTitle, BookAccess, BookCursor, BookFilter, BookKinds, BookLabelIds, BookLink,
        BookLinkKind, BookName, BookSelection, BookSort, BookSortField, BookStatuses,
        BookVisibility, ChapterLocalization, ChapterName, CreatedAtRange, CreatorQuery,
        CreatorRole, FileName, FilterLookupIds, HexHash, Image, ImageContent, ImageExtension,
        LabelFilter, LabelsMode, Limit, LinkUrl, Name, PublicationDemographics,
        PublicationYearRange, ReleaseAccess, Role, Session, SessionTokens, ShortHexHash, SortOrder,
        Text, Timestamp, Token, UserClaims, VisibilityTransition,
    },
    hasher::Hasher,
    jwt::Jwt,
};

pub(crate) const PRIVATE_VISIBILITY: [BookVisibility; 3] = [
    BookVisibility::Draft,
    BookVisibility::PendingReview,
    BookVisibility::Rejected,
];

pub(crate) const ACCESS_TTL: i64 = 3600;
pub(crate) const REFRESH_TTL: i64 = 7200;

const SELECTION_HASH: &str = "0123456789abcdef";
pub(crate) const STALE_HASH: &str = "fedcba9876543210";

pub(crate) fn unfiltered_selection() -> BookSelection {
    BookSelection {
        visibility: BookVisibility::Listed,
        sort_field: BookSortField::CreatedAt,
        order: SortOrder::Desc,
        labels: LabelFilter {
            included: BookLabelIds::try_from(vec![]).unwrap(),
            mode: LabelsMode::And,
            excluded: BookLabelIds::try_from(vec![]).unwrap(),
        },
        kinds: BookKinds::try_from(vec![]).unwrap(),
        statuses: BookStatuses::try_from(vec![]).unwrap(),
        content_rating_ids: FilterLookupIds::try_from(vec![]).unwrap(),
        publication_language_ids: FilterLookupIds::try_from(vec![]).unwrap(),
        publication_demographics: PublicationDemographics::try_from(vec![]).unwrap(),
        available_translated_language_ids: FilterLookupIds::try_from(vec![]).unwrap(),
        publication_year: PublicationYearRange::try_new(None, None).unwrap(),
        created_at: CreatedAtRange::try_new(None, None).unwrap(),
    }
}

pub(crate) fn short_hash(hex: &str) -> ShortHexHash {
    ShortHexHash::try_from(hex.to_owned()).unwrap()
}

pub(crate) fn selection_hash() -> ShortHexHash {
    short_hash(SELECTION_HASH)
}

pub(crate) fn filter(selection: BookSelection) -> BookFilter {
    BookFilter {
        limit: Limit::DEFAULT,
        cursor: None,
        selection_hash: selection_hash(),
        selection,
    }
}

pub(crate) fn filter_paged(selection: BookSelection, cursor: BookCursor) -> BookFilter {
    BookFilter {
        limit: Limit::DEFAULT,
        cursor: Some(cursor),
        selection_hash: selection_hash(),
        selection,
    }
}

pub(crate) fn cursor_for() -> BookCursor {
    BookCursor {
        id: Uuid::now_v7(),
        sort: BookSort::CreatedAt(Timestamp::UNIX_EPOCH),
        selection_hash: selection_hash(),
    }
}

pub(crate) fn every_sort_value() -> [BookSort; 3] {
    [
        BookSort::CreatedAt(Timestamp::from(
            OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap(),
        )),
        BookSort::Name("Solo Leveling".to_owned()),
        BookSort::PublicationYear(2016),
    ]
}

pub(crate) fn sample_link() -> BookLink {
    BookLink {
        kind: BookLinkKind::WhereToRead,
        url: LinkUrl::try_from("https://example.com/read".to_owned()).unwrap(),
    }
}

pub(crate) fn sample_title() -> AlternativeTitle {
    AlternativeTitle {
        language_id: Uuid::now_v7(),
        name: BookName::try_from("Alt title".to_owned()).unwrap(),
    }
}

pub(crate) fn sample_creator() -> CreatorQuery {
    CreatorQuery {
        id: Uuid::now_v7(),
        first_name: Name::try_from("Jane".to_owned()).unwrap(),
        last_name: Name::try_from("Doe".to_owned()).unwrap(),
        roles: vec![CreatorRole::Author],
        created_at: Timestamp::now(),
    }
}

pub(crate) fn sample_localization() -> ChapterLocalization {
    ChapterLocalization {
        language_id: Uuid::now_v7(),
        name: ChapterName::try_from("Chapter 1".to_owned()).unwrap(),
    }
}

pub(crate) fn claims(id: Uuid, roles: &[Role]) -> UserClaims {
    UserClaims {
        id,
        sid: Uuid::now_v7(),
        roles: roles.to_vec(),
    }
}

pub(crate) fn access(visibility: BookVisibility, owner: Uuid) -> BookAccess {
    BookAccess {
        visibility,
        created_by: owner,
    }
}

pub(crate) fn release(visibility: BookVisibility, owner: Uuid) -> ReleaseAccess {
    ReleaseAccess {
        visibility,
        created_by: owner,
    }
}

pub(crate) fn transition(visibility: BookVisibility, note: Option<&str>) -> VisibilityTransition {
    VisibilityTransition {
        id: Uuid::now_v7(),
        visibility,
        note: note.map(|n| Text::try_from(n.to_owned()).unwrap()),
        now: Timestamp::now(),
    }
}

pub(crate) fn session(age: Duration) -> Session {
    let now = Timestamp::now();

    Session {
        sid: Uuid::now_v7(),
        jti: HexHash::try_from("a".repeat(64)).unwrap(),
        ua: None,
        ip: None,
        created_at: now - age,
        updated_at: now - age,
    }
}

pub(crate) fn session_tokens() -> SessionTokens {
    SessionTokens {
        access: Token {
            value: "access-value".to_owned(),
            ttl: 900,
        },
        refresh: Token {
            value: "refresh-value".to_owned(),
            ttl: 1_209_600,
        },
    }
}

pub(crate) fn image(content: ImageContent, file_name: FileName) -> Image {
    Image {
        id: Uuid::from_u128(1),
        extension: ImageExtension::Png,
        content,
        file_name,
    }
}

pub(crate) fn default_image() -> Image {
    const PNG_SIGNATURE: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

    let content = ImageContent::try_from(Bytes::from(PNG_SIGNATURE.to_vec())).unwrap();
    let file_name = FileName::try_from("page.png".to_owned()).unwrap();
    image(content, file_name)
}

pub(crate) fn hasher() -> Hasher {
    Hasher::new(19456, 2, 1, b"test-pepper").unwrap()
}

fn key_pair(ttl: i64) -> config::Jwt {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);

    let dir =
        std::env::temp_dir().join(format!("manga-theka-jwt-tokens-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();

    let signing = SigningKey::from_bytes(&[(n as u8).wrapping_add(1); 32]);
    let private_key = dir.join("private.pem");
    let public_key = dir.join("public.pem");
    std::fs::write(
        &private_key,
        signing.to_pkcs8_pem(LineEnding::LF).unwrap().as_bytes(),
    )
    .unwrap();
    std::fs::write(
        &public_key,
        signing
            .verifying_key()
            .to_public_key_pem(LineEnding::LF)
            .unwrap(),
    )
    .unwrap();

    config::Jwt {
        private_key,
        public_key,
        ttl,
    }
}

pub(crate) fn jwt() -> Jwt {
    Jwt::load(&key_pair(ACCESS_TTL), &key_pair(REFRESH_TTL))
}

pub(crate) fn jwt_shared() -> Jwt {
    let cfg = key_pair(ACCESS_TTL);
    Jwt::load(&cfg, &cfg)
}
