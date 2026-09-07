use std::ops::RangeInclusive;
use std::sync::LazyLock;

use fake::Dummy;
use fake::Fake;
use fake::faker::internet::en::{DomainSuffix, SafeEmail};
use fake::faker::lorem::en::{Sentence, Word};
use fake::faker::name::en::FirstName;
use fake::rand::RngExt;
use manga_theka::entity::{
    AlternativeTitle, BookKind, BookLink, BookLinkKind, BookName, BookQuery, BookStatus,
    BookVisibility, Chapter, ChapterLocalization, ChapterName, ChapterNumber, ChapterVolume,
    ContentRating, Creator, CreatorQuery, CreatorRole, Email, Feedback, FeedbackKind,
    FeedbackStatus, Label, LabelKind, Language, LinkUrl, Name, PasswordHash,
    PublicationDemographic, Role, Text, UserQuery, Username,
};
use time::OffsetDateTime;
use uuid::{Uuid, uuid};

use crate::helpers::KNOWN_PASSWORD_PHC;

pub const COVER_JPG: &[u8] = include_bytes!("fixtures/cover.jpg");
pub const COVER_PNG: &[u8] = include_bytes!("fixtures/cover.png");
pub const COVER_WEBP: &[u8] = include_bytes!("fixtures/cover.webp");

pub const EVERY_VISIBILITY: [BookVisibility; 5] = [
    BookVisibility::Draft,
    BookVisibility::PendingReview,
    BookVisibility::Listed,
    BookVisibility::Rejected,
    BookVisibility::Hidden,
];

pub static CONTENT_RATINGS: LazyLock<[ContentRating; 4]> = LazyLock::new(|| {
    [
        ContentRating {
            id: uuid!("019f124b-314f-73fc-8310-701df63eacea"),
            name: "Everyone".to_owned(),
            code: "E".to_owned(),
        },
        ContentRating {
            id: uuid!("019f125c-33ef-753d-984f-08781390d2f4"),
            name: "Teen".to_owned(),
            code: "T".to_owned(),
        },
        ContentRating {
            id: uuid!("019f125c-bf63-7578-a57d-16f2f593c365"),
            name: "Teen Plus".to_owned(),
            code: "T+".to_owned(),
        },
        ContentRating {
            id: uuid!("019f125d-2006-7a75-bcc2-4fc07e370d2d"),
            name: "Mature".to_owned(),
            code: "M".to_owned(),
        },
    ]
});

pub static LANGUAGES: LazyLock<[Language; 5]> = LazyLock::new(|| {
    [
        Language {
            id: uuid!("019f12ac-d1fc-78f2-a4bc-d0827c0f1578"),
            code: "ja".to_owned(),
            name: "Japanese".to_owned(),
        },
        Language {
            id: uuid!("019f12ac-f9f3-7b8c-ba3f-97033810d391"),
            code: "ko".to_owned(),
            name: "Korean".to_owned(),
        },
        Language {
            id: uuid!("019f12ad-0c41-7022-8877-50861d4ec2a4"),
            code: "zh".to_owned(),
            name: "Chinese".to_owned(),
        },
        Language {
            id: uuid!("019f12ad-1e26-7fdd-9318-b18ccd74e734"),
            code: "en".to_owned(),
            name: "English".to_owned(),
        },
        Language {
            id: uuid!("019f12ad-2e9e-762d-999c-b4bc9bbdc964"),
            code: "ru".to_owned(),
            name: "Russian".to_owned(),
        },
    ]
});

pub const ACTION: Uuid = uuid!("019febc2-01af-7d7a-9468-ad2779d97503");
pub const FANTASY: Uuid = uuid!("019febc2-01b3-75aa-9d15-98e0f3f847d6");
pub const ROMANCE: Uuid = uuid!("019febc2-01b9-763d-822a-4c21c88b4f6d");
pub const ISEKAI: Uuid = uuid!("019febc2-01b5-7ff6-8a68-81390867cd50");
pub const MAFIA: Uuid = uuid!("019febba-36fa-701d-97ed-e65d87fe8ddd");
pub const ZOMBIES: Uuid = uuid!("019febba-36fa-701d-97ed-eb6f2148a897");
pub const SCHOOL_LIFE: Uuid = uuid!("019febbf-0532-70e9-85dc-7929be8cafc9");
pub const LONG_STRIP: Uuid = uuid!("019cabd8-3822-7e25-b84c-08575e693b65");

pub static LABELS: LazyLock<[Label; 8]> = LazyLock::new(|| {
    [
        Label {
            id: ACTION,
            name: "Action".to_owned(),
            kind: LabelKind::Genre,
        },
        Label {
            id: FANTASY,
            name: "Fantasy".to_owned(),
            kind: LabelKind::Genre,
        },
        Label {
            id: ROMANCE,
            name: "Romance".to_owned(),
            kind: LabelKind::Genre,
        },
        Label {
            id: ISEKAI,
            name: "Isekai".to_owned(),
            kind: LabelKind::Genre,
        },
        Label {
            id: MAFIA,
            name: "Mafia".to_owned(),
            kind: LabelKind::Theme,
        },
        Label {
            id: ZOMBIES,
            name: "Zombies".to_owned(),
            kind: LabelKind::Theme,
        },
        Label {
            id: SCHOOL_LIFE,
            name: "School Life".to_owned(),
            kind: LabelKind::Theme,
        },
        Label {
            id: LONG_STRIP,
            name: "Long Strip".to_owned(),
            kind: LabelKind::Presentation,
        },
    ]
});

pub struct CreatorRoleFaker;

impl Dummy<CreatorRoleFaker> for CreatorRole {
    fn dummy_with_rng<R: RngExt + ?Sized>(_config: &CreatorRoleFaker, rng: &mut R) -> Self {
        if rng.random() {
            CreatorRole::Artist
        } else {
            CreatorRole::Author
        }
    }
}

pub struct NameFaker;

impl Dummy<NameFaker> for Name {
    fn dummy_with_rng<R: RngExt + ?Sized>(_config: &NameFaker, rng: &mut R) -> Self {
        let name = FirstName().fake_with_rng::<String, R>(rng);
        Name::try_from(name).unwrap()
    }
}

pub struct UserFaker {
    pub roles: Vec<Role>,
    pub verified: bool,
}

impl Default for UserFaker {
    fn default() -> Self {
        UserFaker {
            roles: vec![Role::Reader],
            verified: false,
        }
    }
}

impl Dummy<UserFaker> for UserQuery {
    fn dummy_with_rng<R: RngExt + ?Sized>(config: &UserFaker, _rng: &mut R) -> Self {
        let id = Uuid::now_v7();
        let handle = id.simple().to_string();

        UserQuery {
            id,
            email: Email::try_from(format!("{}@example.test", &handle[..12])).unwrap(),
            username: Username::try_from(format!("u{}", &handle[..16])).unwrap(),
            password_hash: PasswordHash::try_from(KNOWN_PASSWORD_PHC.to_owned()).unwrap(),
            roles: config.roles.clone(),
            verified_at: config
                .verified
                .then(|| (OffsetDateTime::now_utc() - time::Duration::hours(1)).into()),
            created_at: OffsetDateTime::now_utc().into(),
        }
    }
}

pub struct CreatorFaker;

impl Dummy<CreatorFaker> for Creator {
    fn dummy_with_rng<R: RngExt + ?Sized>(_config: &CreatorFaker, rng: &mut R) -> Self {
        Creator {
            id: Uuid::now_v7(),
            first_name: NameFaker.fake_with_rng(rng),
            last_name: NameFaker.fake_with_rng(rng),
            created_at: OffsetDateTime::now_utc(),
        }
    }
}

pub struct CreatorQueryFaker;

impl Dummy<CreatorQueryFaker> for CreatorQuery {
    fn dummy_with_rng<R: RngExt + ?Sized>(_config: &CreatorQueryFaker, rng: &mut R) -> Self {
        let mut roles = vec![CreatorRoleFaker.fake_with_rng::<CreatorRole, R>(rng)];
        if rng.random() {
            let other = match roles[0] {
                CreatorRole::Artist => CreatorRole::Author,
                CreatorRole::Author => CreatorRole::Artist,
            };
            roles.push(other);
        }

        CreatorQuery {
            id: Uuid::now_v7(),
            first_name: NameFaker.fake_with_rng(rng),
            last_name: NameFaker.fake_with_rng(rng),
            roles,
            created_at: OffsetDateTime::now_utc(),
        }
    }
}

pub struct BookStatusFaker;

impl Dummy<BookStatusFaker> for BookStatus {
    fn dummy_with_rng<R: RngExt + ?Sized>(_config: &BookStatusFaker, rng: &mut R) -> Self {
        match rng.random_range(0..4) {
            0 => BookStatus::Ongoing,
            1 => BookStatus::Completed,
            2 => BookStatus::Hiatus,
            _ => BookStatus::Cancelled,
        }
    }
}

pub struct BookKindFaker;

impl Dummy<BookKindFaker> for BookKind {
    fn dummy_with_rng<R: RngExt + ?Sized>(_config: &BookKindFaker, rng: &mut R) -> Self {
        match rng.random_range(0..3) {
            0 => BookKind::Manga,
            1 => BookKind::Manhwa,
            _ => BookKind::Manhua,
        }
    }
}

pub struct PublicationDemographicFaker;

impl Dummy<PublicationDemographicFaker> for PublicationDemographic {
    fn dummy_with_rng<R: RngExt + ?Sized>(
        _config: &PublicationDemographicFaker,
        rng: &mut R,
    ) -> Self {
        match rng.random_range(0..5) {
            0 => PublicationDemographic::Shounen,
            1 => PublicationDemographic::Shoujo,
            2 => PublicationDemographic::Seinen,
            3 => PublicationDemographic::Josei,
            _ => PublicationDemographic::Kids,
        }
    }
}

pub struct BookNameFaker;

impl Dummy<BookNameFaker> for BookName {
    fn dummy_with_rng<R: RngExt + ?Sized>(_config: &BookNameFaker, rng: &mut R) -> Self {
        let name = Sentence(2..5).fake_with_rng::<String, R>(rng);
        BookName::try_from(name).unwrap()
    }
}

pub struct TextFaker;

impl Dummy<TextFaker> for Text {
    fn dummy_with_rng<R: RngExt + ?Sized>(_config: &TextFaker, rng: &mut R) -> Self {
        let description = Sentence(5..12).fake_with_rng::<String, R>(rng);
        Text::try_from(description).unwrap()
    }
}

pub struct EmailFaker;

impl Dummy<EmailFaker> for Email {
    fn dummy_with_rng<R: RngExt + ?Sized>(_config: &EmailFaker, rng: &mut R) -> Self {
        let address = SafeEmail().fake_with_rng::<String, R>(rng);
        Email::try_from(address).unwrap()
    }
}

pub struct FeedbackFaker {
    pub kind: FeedbackKind,
    pub status: FeedbackStatus,
    pub book_id: Option<Uuid>,
}

impl Default for FeedbackFaker {
    fn default() -> Self {
        FeedbackFaker {
            kind: FeedbackKind::General,
            status: FeedbackStatus::Open,
            book_id: None,
        }
    }
}

impl Dummy<FeedbackFaker> for Feedback {
    fn dummy_with_rng<R: RngExt + ?Sized>(config: &FeedbackFaker, rng: &mut R) -> Self {
        Feedback {
            id: Uuid::now_v7(),
            kind: match config.kind {
                FeedbackKind::Report => FeedbackKind::Report,
                FeedbackKind::Correction => FeedbackKind::Correction,
                FeedbackKind::General => FeedbackKind::General,
            },
            status: match config.status {
                FeedbackStatus::Open => FeedbackStatus::Open,
                FeedbackStatus::Resolved => FeedbackStatus::Resolved,
                FeedbackStatus::Dismissed => FeedbackStatus::Dismissed,
            },
            email: EmailFaker.fake_with_rng(rng),
            note: TextFaker.fake_with_rng(rng),
            book_id: config.book_id,
            updated_at: None,
            created_at: OffsetDateTime::UNIX_EPOCH,
        }
    }
}

pub struct ContentRatingFaker;

impl Dummy<ContentRatingFaker> for ContentRating {
    fn dummy_with_rng<R: RngExt + ?Sized>(_config: &ContentRatingFaker, rng: &mut R) -> Self {
        CONTENT_RATINGS[rng.random_range(0..CONTENT_RATINGS.len())].clone()
    }
}

pub struct LanguageFaker;

impl Dummy<LanguageFaker> for Language {
    fn dummy_with_rng<R: RngExt + ?Sized>(_config: &LanguageFaker, rng: &mut R) -> Self {
        LANGUAGES[rng.random_range(0..LANGUAGES.len())].clone()
    }
}

pub struct LabelFaker;

impl Dummy<LabelFaker> for Label {
    fn dummy_with_rng<R: RngExt + ?Sized>(_config: &LabelFaker, rng: &mut R) -> Self {
        LABELS[rng.random_range(0..LABELS.len())].clone()
    }
}

pub struct BookLinkKindFaker;

impl Dummy<BookLinkKindFaker> for BookLinkKind {
    fn dummy_with_rng<R: RngExt + ?Sized>(_config: &BookLinkKindFaker, rng: &mut R) -> Self {
        match rng.random_range(0..3) {
            0 => BookLinkKind::WhereToRead,
            1 => BookLinkKind::WhereToBuy,
            _ => BookLinkKind::Track,
        }
    }
}

pub struct LinkUrlFaker;

impl Dummy<LinkUrlFaker> for LinkUrl {
    fn dummy_with_rng<R: RngExt + ?Sized>(_config: &LinkUrlFaker, rng: &mut R) -> Self {
        let domain = Word().fake_with_rng::<String, R>(rng);
        let suffix = DomainSuffix().fake_with_rng::<String, R>(rng);
        let path = Word().fake_with_rng::<String, R>(rng);
        let id = Uuid::now_v7();

        LinkUrl::try_from(format!("https://{domain}.{suffix}/{path}/{id}")).unwrap()
    }
}

pub struct BookLinkFaker;

impl Dummy<BookLinkFaker> for BookLink {
    fn dummy_with_rng<R: RngExt + ?Sized>(_config: &BookLinkFaker, rng: &mut R) -> Self {
        BookLink {
            kind: BookLinkKindFaker.fake_with_rng(rng),
            url: LinkUrlFaker.fake_with_rng(rng),
        }
    }
}

pub struct AlternativeTitleFaker;

impl Dummy<AlternativeTitleFaker> for AlternativeTitle {
    fn dummy_with_rng<R: RngExt + ?Sized>(_config: &AlternativeTitleFaker, rng: &mut R) -> Self {
        let language: Language = LanguageFaker.fake_with_rng(rng);

        AlternativeTitle {
            language_id: language.id,
            name: BookNameFaker.fake_with_rng(rng),
        }
    }
}

pub struct BookFaker {
    pub labels: RangeInclusive<usize>,
    pub links: RangeInclusive<usize>,
    pub titles: RangeInclusive<usize>,
    pub creators: RangeInclusive<usize>,
    pub visibility: BookVisibility,
    pub exact_labels: Option<Vec<Label>>,
    pub name: Option<String>,
    pub status: Option<BookStatus>,
    pub kind: Option<BookKind>,
    pub content_rating: Option<ContentRating>,
    pub publication_language: Option<Language>,
    pub publication_demographic: Option<PublicationDemographic>,
    pub publication_year: Option<i16>,
    pub created_at: Option<OffsetDateTime>,
}

impl Default for BookFaker {
    fn default() -> Self {
        BookFaker {
            labels: 0..=5,
            links: 0..=5,
            titles: 0..=5,
            creators: 0..=5,
            visibility: BookVisibility::Listed,
            exact_labels: None,
            name: None,
            status: None,
            kind: None,
            content_rating: None,
            publication_language: None,
            publication_demographic: None,
            publication_year: None,
            created_at: None,
        }
    }
}

impl Dummy<BookFaker> for BookQuery {
    fn dummy_with_rng<R: RngExt + ?Sized>(config: &BookFaker, rng: &mut R) -> Self {
        let mut labels: Vec<Label> = match &config.exact_labels {
            Some(labels) => labels.clone(),
            None => (0..rng.random_range(config.labels.clone()))
                .map(|_| LabelFaker.fake_with_rng(rng))
                .collect(),
        };
        labels.sort_by_key(|l| l.id);
        labels.dedup_by_key(|l| l.id);

        let links: Vec<BookLink> = (0..rng.random_range(config.links.clone()))
            .map(|_| BookLinkFaker.fake_with_rng(rng))
            .collect();
        let titles: Vec<AlternativeTitle> = (0..rng.random_range(config.titles.clone()))
            .map(|_| AlternativeTitleFaker.fake_with_rng(rng))
            .collect();
        let creators: Vec<CreatorQuery> = (0..rng.random_range(config.creators.clone()))
            .map(|_| CreatorQueryFaker.fake_with_rng(rng))
            .collect();

        BookQuery {
            id: Uuid::now_v7(),
            name: match &config.name {
                Some(name) => BookName::try_from(name.clone()).expect("faker name must be valid"),
                None => BookNameFaker.fake_with_rng(rng),
            },
            description: TextFaker.fake_with_rng(rng),
            publication_year: config
                .publication_year
                .unwrap_or_else(|| rng.random_range(1950..=2026)),
            content_rating: config
                .content_rating
                .clone()
                .unwrap_or_else(|| ContentRatingFaker.fake_with_rng(rng)),
            status: config
                .status
                .unwrap_or_else(|| BookStatusFaker.fake_with_rng(rng)),
            kind: config
                .kind
                .unwrap_or_else(|| BookKindFaker.fake_with_rng(rng)),
            publication_language: config
                .publication_language
                .clone()
                .unwrap_or_else(|| LanguageFaker.fake_with_rng(rng)),
            publication_demographic: config
                .publication_demographic
                .unwrap_or_else(|| PublicationDemographicFaker.fake_with_rng(rng)),
            labels: labels.try_into().expect("too many labels in faker"),
            links: links.try_into().expect("too many links in faker"),
            titles: titles.try_into().expect("too many titles in faker"),
            creators: creators.try_into().expect("too many creators in faker"),
            visibility: config.visibility,
            note: None,
            submitted_at: None,
            updated_at: None,
            created_at: config.created_at.unwrap_or(OffsetDateTime::UNIX_EPOCH),
        }
    }
}

pub struct ChapterNumberFaker;

impl Dummy<ChapterNumberFaker> for ChapterNumber {
    fn dummy_with_rng<R: RngExt + ?Sized>(_config: &ChapterNumberFaker, rng: &mut R) -> Self {
        let hundredths = rng.random_range(0..=9_999_999);
        ChapterNumber::try_from(hundredths as f32 / 100.0).unwrap()
    }
}

pub struct ChapterNameFaker;

impl Dummy<ChapterNameFaker> for ChapterName {
    fn dummy_with_rng<R: RngExt + ?Sized>(_config: &ChapterNameFaker, rng: &mut R) -> Self {
        let name = Sentence(2..5).fake_with_rng::<String, R>(rng);
        ChapterName::try_from(name).unwrap()
    }
}

pub struct ChapterLocalizationFaker;

impl Dummy<ChapterLocalizationFaker> for ChapterLocalization {
    fn dummy_with_rng<R: RngExt + ?Sized>(_config: &ChapterLocalizationFaker, rng: &mut R) -> Self {
        let language: Language = LanguageFaker.fake_with_rng(rng);

        ChapterLocalization {
            language_id: language.id,
            name: ChapterNameFaker.fake_with_rng(rng),
        }
    }
}

pub struct ChapterFaker {
    pub book_id: Uuid,
    pub localizations: RangeInclusive<usize>,
}

impl ChapterFaker {
    pub fn new(book_id: Uuid) -> Self {
        ChapterFaker {
            book_id,
            localizations: 0..=3,
        }
    }
}

impl Dummy<ChapterFaker> for Chapter {
    fn dummy_with_rng<R: RngExt + ?Sized>(config: &ChapterFaker, rng: &mut R) -> Self {
        let mut localizations: Vec<ChapterLocalization> = (0..rng
            .random_range(config.localizations.clone()))
            .map(|_| ChapterLocalizationFaker.fake_with_rng(rng))
            .collect();
        localizations.sort_by_key(|l| l.language_id);
        localizations.dedup_by_key(|l| l.language_id);

        Chapter {
            id: Uuid::now_v7(),
            book_id: config.book_id,
            number: ChapterNumberFaker.fake_with_rng(rng),
            name: Some(ChapterNameFaker.fake_with_rng(rng)),
            volume: Some(ChapterVolume::try_from(rng.random_range(0..=200)).unwrap()),
            localizations: localizations
                .try_into()
                .expect("too many localizations in faker"),
            updated_at: None,
            created_at: OffsetDateTime::UNIX_EPOCH,
        }
    }
}
