use std::ops::RangeInclusive;
use std::sync::LazyLock;

use fake::Dummy;
use fake::Fake;
use fake::faker::internet::en::DomainSuffix;
use fake::faker::lorem::en::{Sentence, Word};
use fake::faker::name::en::FirstName;
use fake::rand::RngExt;
use manga_theka::entity::{
    AlternativeTitle, Book, BookKind, BookLink, BookLinkKind, BookName, BookStatus, ContentRating,
    Creator, CreatorRole, Description, Label, LabelKind, Language, LinkUrl, Name,
};
use time::OffsetDateTime;
use uuid::{Uuid, uuid};

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

pub struct CreatorFaker;

impl Dummy<CreatorFaker> for Creator {
    fn dummy_with_rng<R: RngExt + ?Sized>(_config: &CreatorFaker, rng: &mut R) -> Self {
        Creator {
            id: Uuid::now_v7(),
            first_name: NameFaker.fake_with_rng(rng),
            last_name: NameFaker.fake_with_rng(rng),
            role: CreatorRoleFaker.fake_with_rng(rng),
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

pub struct BookNameFaker;

impl Dummy<BookNameFaker> for BookName {
    fn dummy_with_rng<R: RngExt + ?Sized>(_config: &BookNameFaker, rng: &mut R) -> Self {
        let name = Sentence(2..5).fake_with_rng::<String, R>(rng);
        BookName::try_from(name).unwrap()
    }
}

pub struct DescriptionFaker;

impl Dummy<DescriptionFaker> for Description {
    fn dummy_with_rng<R: RngExt + ?Sized>(_config: &DescriptionFaker, rng: &mut R) -> Self {
        let description = Sentence(5..12).fake_with_rng::<String, R>(rng);
        Description::try_from(description).unwrap()
    }
}

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

pub struct ContentRatingFaker;

impl Dummy<ContentRatingFaker> for ContentRating {
    fn dummy_with_rng<R: RngExt + ?Sized>(_config: &ContentRatingFaker, rng: &mut R) -> Self {
        CONTENT_RATINGS[rng.random_range(0..CONTENT_RATINGS.len())].clone()
    }
}

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

pub struct LanguageFaker;

impl Dummy<LanguageFaker> for Language {
    fn dummy_with_rng<R: RngExt + ?Sized>(_config: &LanguageFaker, rng: &mut R) -> Self {
        LANGUAGES[rng.random_range(0..LANGUAGES.len())].clone()
    }
}

pub static LABELS: LazyLock<[Label; 6]> = LazyLock::new(|| {
    [
        Label {
            id: uuid!("019febc2-01af-7d7a-9468-ad2779d97503"),
            name: "Action".to_owned(),
            kind: LabelKind::Genre,
        },
        Label {
            id: uuid!("019febc2-01b3-75aa-9d15-98e0f3f847d6"),
            name: "Fantasy".to_owned(),
            kind: LabelKind::Genre,
        },
        Label {
            id: uuid!("019febc2-01b9-763d-822a-4c21c88b4f6d"),
            name: "Romance".to_owned(),
            kind: LabelKind::Genre,
        },
        Label {
            id: uuid!("019febba-36fa-701d-97ed-e65d87fe8ddd"),
            name: "Mafia".to_owned(),
            kind: LabelKind::Tag,
        },
        Label {
            id: uuid!("019febba-36fa-701d-97ed-eb6f2148a897"),
            name: "Zombies".to_owned(),
            kind: LabelKind::Tag,
        },
        Label {
            id: uuid!("019febbf-0532-70e9-85dc-7929be8cafc9"),
            name: "School Life".to_owned(),
            kind: LabelKind::Tag,
        },
    ]
});

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
}

impl Default for BookFaker {
    fn default() -> Self {
        BookFaker {
            labels: 0..=5,
            links: 0..=5,
            titles: 0..=5,
            creators: 0..=5,
        }
    }
}

impl Dummy<BookFaker> for Book {
    fn dummy_with_rng<R: RngExt + ?Sized>(config: &BookFaker, rng: &mut R) -> Self {
        let mut labels: Vec<Label> = (0..rng.random_range(config.labels.clone()))
            .map(|_| LabelFaker.fake_with_rng(rng))
            .collect();
        labels.sort_by_key(|l| l.id);
        labels.dedup_by_key(|l| l.id);

        let links: Vec<BookLink> = (0..rng.random_range(config.links.clone()))
            .map(|_| BookLinkFaker.fake_with_rng(rng))
            .collect();
        let titles: Vec<AlternativeTitle> = (0..rng.random_range(config.titles.clone()))
            .map(|_| AlternativeTitleFaker.fake_with_rng(rng))
            .collect();
        let creators: Vec<Creator> = (0..rng.random_range(config.creators.clone()))
            .map(|_| CreatorFaker.fake_with_rng(rng))
            .collect();

        Book {
            id: Uuid::now_v7(),
            name: BookNameFaker.fake_with_rng(rng),
            description: DescriptionFaker.fake_with_rng(rng),
            publication_year: rng.random_range(1950..=2026),
            content_rating: ContentRatingFaker.fake_with_rng(rng),
            status: BookStatusFaker.fake_with_rng(rng),
            kind: BookKindFaker.fake_with_rng(rng),
            publication_language: LanguageFaker.fake_with_rng(rng),
            labels,
            links,
            titles,
            creators,
            updated_at: None,
            created_at: OffsetDateTime::UNIX_EPOCH,
        }
    }
}
