use aws_sdk_s3::operation::head_object::HeadObjectOutput;
use aws_sdk_s3::primitives::ByteStream;
use fake::Fake;
use manga_theka::entity::{
    AlternativeTitle, BookCoverQuery, BookKind, BookLink, BookName, BookQuery, BookStatus,
    BookVisibility, Chapter, ChapterLocalization, ChapterName, ChapterNumber, ChapterVolume,
    ContentRating, CoverUrl, Creator, CreatorQuery, CreatorRole, Email, Feedback, ImageExtension,
    Label, Language, LinkUrl, Name, PublicationDemographic, Role, Session, Text, Timestamp,
    UserQuery,
};
use uuid::Uuid;

use super::app::{DEFAULT_ROLES, TestApp};
use super::fakers::{
    ACTION, BookFaker, CONTENT_RATINGS, ChapterFaker, FANTASY, ISEKAI, LANGUAGES, LONG_STRIP,
    MAFIA, ROMANCE, SCHOOL_LIFE, UserFaker, ZOMBIES,
};
use super::pure::{day, labels};
use super::samples::{BookSample, BookVisibilityState, ChapterSample};

pub const KNOWN_PASSWORD: &str = "correct horse battery staple";
pub const KNOWN_PASSWORD_PHC: &str = "$argon2id$v=19$m=19456,t=2,p=1$P/qEkLVC2cUEZbYOairU+A$vdXpqQIqcVifQO1OXrCfvkTanUQLmtw2L7jEc6UDbiA";

type FacetRow = (
    &'static [Uuid],
    BookKind,
    BookStatus,
    PublicationDemographic,
    usize,
    usize,
);

impl TestApp {
    pub async fn db_insert_user(&self, user: &UserQuery) {
        let roles: Vec<String> = user
            .roles
            .as_slice()
            .iter()
            .map(|r| r.as_ref().to_owned())
            .collect();

        sqlx::query!(
            r#"
insert into users(id, email, username, password_hash, roles, verified_at, created_at)
values($1, $2, $3, $4, $5, $6, $7);
        "#,
            user.id,
            user.email.as_ref(),
            user.username.as_ref(),
            KNOWN_PASSWORD_PHC,
            &roles,
            user.verified_at.map(Timestamp::into_inner),
            user.created_at.into_inner(),
        )
        .execute(&self.pool)
        .await
        .expect("failed to insert factory user");
    }

    pub async fn db_seed_reader(&self) -> UserQuery {
        let user: UserQuery = UserFaker {
            roles: vec![Role::Reader],
            verified: true,
        }
        .fake();
        self.db_insert_user(&user).await;

        user
    }

    pub async fn memory_insert_session(
        &self,
        sub: Uuid,
        sid: Uuid,
        created_at: time::OffsetDateTime,
    ) {
        let session = Session {
            sid,
            jti: self
                .hasher
                .compute_keyed_hex_hash(Uuid::now_v7())
                .expect("keyed jti hash"),
            ua: None,
            ip: None,
            created_at: created_at.into(),
            updated_at: created_at.into(),
        };

        self.memory
            .put_session(sub, session)
            .await
            .expect("failed to insert factory session");
    }

    async fn storage_head_object(&self, bucket: &str, key: Uuid) -> Option<HeadObjectOutput> {
        self.s3
            .head_object()
            .bucket(bucket)
            .key(key.to_string())
            .send()
            .await
            .ok()
    }

    pub async fn storage_object_exists(&self, bucket: &str, key: Uuid) -> bool {
        self.storage_head_object(bucket, key).await.is_some()
    }

    pub async fn storage_object_content_type(&self, bucket: &str, key: Uuid) -> String {
        self.storage_head_object(bucket, key)
            .await
            .expect("stored object must exist")
            .content_type()
            .expect("stored object must carry a content type")
            .to_owned()
    }

    pub async fn storage_objects_count(&self, bucket: &str) -> usize {
        self.s3
            .list_objects_v2()
            .bucket(bucket)
            .send()
            .await
            .expect("failed to list the bucket")
            .contents()
            .len()
    }

    async fn storage_put_image(
        &self,
        bucket: &str,
        id: Uuid,
        extension: ImageExtension,
        body: ByteStream,
    ) {
        self.s3
            .put_object()
            .bucket(bucket)
            .key(id.to_string())
            .content_type(extension.content_type())
            .content_disposition(format!("inline; filename=\"{id}.{}\"", extension.as_ref()))
            .body(body)
            .send()
            .await
            .expect("failed to upload factory image object");
    }

    pub async fn db_insert_book_with_visibility(&self, visibility: BookVisibility) -> Uuid {
        let book: BookQuery = BookFaker {
            visibility,
            ..Default::default()
        }
        .fake();

        self.fixture_insert_book(&book).await;

        book.id
    }

    pub async fn db_fetch_book_visibility_state(&self, id: Uuid) -> BookVisibilityState {
        let row = sqlx::query!(
            r#"
select b.visibility, b.note, b.updated_at
from books b
where b.id = $1;
        "#,
            id
        )
        .fetch_one(&self.pool)
        .await
        .expect("failed to read book visibility state");

        BookVisibilityState {
            visibility: row.visibility,
            note: row.note,
            updated_at: row.updated_at,
        }
    }

    pub async fn fixture_insert_book(&self, b: &BookQuery) {
        let mut created_by: UserQuery = UserFaker::default().fake();
        created_by.id = b.created_by;
        self.db_insert_user(&created_by).await;

        self.db_insert_book(b).await;
    }

    pub async fn db_insert_book(&self, b: &BookQuery) {
        sqlx::query!(
            r#"
insert into books(id, name, description, publication_year, content_rating, status, kind,
                  publication_language, publication_demographic, visibility, note, submitted_at,
                  updated_at, created_at, created_by)
values($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15);
        "#,
            b.id,
            b.name.as_ref(),
            b.description.as_ref(),
            b.publication_year,
            b.content_rating.id,
            b.status.as_ref() as _,
            b.kind.as_ref() as _,
            b.publication_language.id,
            b.publication_demographic.as_ref() as _,
            b.visibility.as_ref() as _,
            b.note.as_ref().map(AsRef::as_ref) as Option<&str>,
            b.submitted_at,
            b.updated_at,
            b.created_at,
            b.created_by
        )
        .execute(&self.pool)
        .await
        .expect("failed to insert factory book");

        let mut label_ids: Vec<Uuid> = Vec::with_capacity(b.labels.len());
        for l in b.labels.as_slice() {
            label_ids.push(l.id);
        }
        sqlx::query!(
            r#"
insert into book_labels(book_id, label_id)
select $1, label_id
from unnest($2::uuid[]) as label_id;
        "#,
            b.id,
            &label_ids
        )
        .execute(&self.pool)
        .await
        .expect("failed to insert factory book labels");

        let mut link_kinds: Vec<String> = Vec::with_capacity(b.links.len());
        let mut link_urls: Vec<String> = Vec::with_capacity(b.links.len());
        for l in b.links.as_slice() {
            link_kinds.push(l.kind.as_ref().to_owned());
            link_urls.push(l.url.as_ref().to_owned());
        }
        sqlx::query!(
            r#"
insert into book_links(book_id, kind, url)
select $1, link.kind, link.url
from unnest($2::text[], $3::text[]) as link(kind, url);
        "#,
            b.id,
            &link_kinds,
            &link_urls
        )
        .execute(&self.pool)
        .await
        .expect("failed to insert factory book links");

        let mut title_language_ids: Vec<Uuid> = Vec::with_capacity(b.titles.len());
        let mut title_names: Vec<String> = Vec::with_capacity(b.titles.len());
        for t in b.titles.as_slice() {
            title_language_ids.push(t.language_id);
            title_names.push(t.name.as_ref().to_owned());
        }
        sqlx::query!(
            r#"
insert into book_titles(book_id, language_id, name)
select $1, title.language_id, title.name
from unnest($2::uuid[], $3::text[]) as title(language_id, name);
        "#,
            b.id,
            &title_language_ids,
            &title_names
        )
        .execute(&self.pool)
        .await
        .expect("failed to insert factory book titles");

        let mut creator_ids: Vec<Uuid> = Vec::with_capacity(b.creators.len());
        let mut creator_first_names: Vec<String> = Vec::with_capacity(b.creators.len());
        let mut creator_last_names: Vec<String> = Vec::with_capacity(b.creators.len());
        let mut creator_created_ats: Vec<time::OffsetDateTime> =
            Vec::with_capacity(b.creators.len());
        let mut credit_creator_ids: Vec<Uuid> = Vec::new();
        let mut credit_roles: Vec<String> = Vec::new();
        for c in b.creators.as_slice() {
            creator_ids.push(c.id);
            creator_first_names.push(c.first_name.as_ref().to_owned());
            creator_last_names.push(c.last_name.as_ref().to_owned());
            creator_created_ats.push(c.created_at);
            for role in &c.roles {
                credit_creator_ids.push(c.id);
                credit_roles.push(role.as_ref().to_owned());
            }
        }

        if !creator_ids.is_empty() {
            sqlx::query!(
                r#"
insert into creators(id, first_name, last_name, created_at)
select c.id, c.first_name, c.last_name, c.created_at
from unnest($1::uuid[], $2::text[], $3::text[], $4::timestamptz[]) as c(id, first_name, last_name, created_at);
            "#,
                &creator_ids,
                &creator_first_names,
                &creator_last_names,
                &creator_created_ats
            )
            .execute(&self.pool)
            .await
            .expect("failed to insert factory book creators");

            sqlx::query!(
                r#"
insert into book_creators(book_id, creator_id, role)
select $1, credit.creator_id, credit.role
from unnest($2::uuid[], $3::text[]) as credit(creator_id, role);
            "#,
                b.id,
                &credit_creator_ids,
                &credit_roles
            )
            .execute(&self.pool)
            .await
            .expect("failed to insert factory book creator credits");
        }
    }

    pub async fn db_fetch_book_sample(&self, id: Uuid) -> BookSample {
        let row = sqlx::query!(
            r#"
select (select b.name from books b where b.id = $1) as "name?",
       (select b.updated_at from books b where b.id = $1) as "updated_at?",
       (select count(*) from book_labels bl where bl.book_id = $1) as "labels!",
       (select count(*) from book_links blk where blk.book_id = $1) as "links!",
       (select count(*) from book_titles bt where bt.book_id = $1) as "titles!";
        "#,
            id
        )
        .fetch_one(&self.pool)
        .await
        .expect("failed to read book sample");

        BookSample {
            name: row.name,
            updated_at: row.updated_at,
            labels: row.labels,
            links: row.links,
            titles: row.titles,
        }
    }

    pub async fn db_fetch_book(&self, id: Uuid) -> BookQuery {
        let row = sqlx::query!(
            r#"
select b.name, b.description, b.publication_year, b.status, b.kind,
       b.publication_demographic, b.visibility, b.note, b.submitted_at, b.updated_at,
       b.created_at, b.created_by,
       cr.id as content_rating_id, cr.name as content_rating_name, cr.code as content_rating_code,
       l.id as language_id, l.code as language_code, l.name as language_name
from books b
join content_ratings cr on cr.id = b.content_rating
join languages l on l.id = b.publication_language
where b.id = $1;
        "#,
            id
        )
        .fetch_one(&self.pool)
        .await
        .expect("failed to read book");

        let labels: Vec<Label> = sqlx::query!(
            r#"
select l.id, l.name, l.kind
from labels l
join book_labels bl on bl.label_id = l.id
where bl.book_id = $1
order by l.name;
        "#,
            id
        )
        .fetch_all(&self.pool)
        .await
        .expect("failed to read book labels")
        .into_iter()
        .map(|r| Label {
            id: r.id,
            name: r.name,
            kind: r.kind.parse().expect("stored label kind must be valid"),
        })
        .collect();

        let links: Vec<BookLink> = sqlx::query!(
            r#"
select kind, url
from book_links
where book_id = $1
order by kind, url;
        "#,
            id
        )
        .fetch_all(&self.pool)
        .await
        .expect("failed to read book links")
        .into_iter()
        .map(|r| BookLink {
            kind: r.kind.parse().expect("stored link kind must be valid"),
            url: LinkUrl::try_from(r.url).expect("stored link url must be valid"),
        })
        .collect();

        let titles: Vec<AlternativeTitle> = sqlx::query!(
            r#"
select language_id, name
from book_titles
where book_id = $1
order by language_id, name;
        "#,
            id
        )
        .fetch_all(&self.pool)
        .await
        .expect("failed to read book titles")
        .into_iter()
        .map(|r| AlternativeTitle {
            language_id: r.language_id,
            name: BookName::try_from(r.name).expect("stored title name must be valid"),
        })
        .collect();

        let creators: Vec<CreatorQuery> = sqlx::query!(
            r#"
select c.id, c.first_name, c.last_name,
       array_agg(bc.role order by bc.role) as "roles!", c.created_at
from book_creators bc
join creators c on c.id = bc.creator_id
where bc.book_id = $1
group by c.id
order by c.id;
        "#,
            id
        )
        .fetch_all(&self.pool)
        .await
        .expect("failed to read book creators")
        .into_iter()
        .map(|r| CreatorQuery {
            id: r.id,
            first_name: Name::try_from(r.first_name).expect("stored first name must be valid"),
            last_name: Name::try_from(r.last_name).expect("stored last name must be valid"),
            roles: r
                .roles
                .into_iter()
                .map(|role| role.parse().expect("stored creator role must be valid"))
                .collect(),
            created_at: r.created_at,
        })
        .collect();

        BookQuery {
            id,
            name: BookName::try_from(row.name).expect("stored name must be valid"),
            description: Text::try_from(row.description).expect("stored description must be valid"),
            publication_year: row.publication_year,
            content_rating: ContentRating {
                id: row.content_rating_id,
                name: row.content_rating_name,
                code: row.content_rating_code,
            },
            status: row.status.parse().expect("stored status must be valid"),
            kind: row.kind.parse().expect("stored kind must be valid"),
            publication_language: Language {
                id: row.language_id,
                code: row.language_code,
                name: row.language_name,
            },
            publication_demographic: row
                .publication_demographic
                .parse()
                .expect("stored publication demographic must be valid"),
            labels: labels.try_into().expect("too many labels in test fixture"),
            links: links.try_into().expect("too many links in test fixture"),
            titles: titles.try_into().expect("too many titles in test fixture"),
            creators: creators
                .try_into()
                .expect("too many creators in test fixture"),
            visibility: row
                .visibility
                .parse()
                .expect("stored visibility must be valid"),
            note: row
                .note
                .map(Text::try_from)
                .transpose()
                .expect("stored note must be valid"),
            submitted_at: row.submitted_at,
            updated_at: row.updated_at,
            created_at: row.created_at,
            created_by: row.created_by,
        }
    }

    pub async fn db_insert_random_book(&self) -> Uuid {
        let book: BookQuery = BookFaker::default().fake();
        self.fixture_insert_book(&book).await;

        book.id
    }

    pub async fn db_fetch_covers(&self, book_id: Uuid) -> Vec<BookCoverQuery> {
        sqlx::query!(
            r#"
select id, extension, is_main
from book_covers
where book_id = $1
order by id;
        "#,
            book_id
        )
        .fetch_all(&self.pool)
        .await
        .expect("failed to read book covers")
        .into_iter()
        .map(|r| BookCoverQuery {
            id: r.id,
            book_id,
            extension: r
                .extension
                .parse()
                .expect("stored cover extension must be valid"),
            is_main: r.is_main,
            url: CoverUrl { cover_id: r.id }.into(),
        })
        .collect()
    }

    pub async fn fixture_insert_cover(&self, book_id: Uuid, image: &'static [u8]) -> Uuid {
        let id = Uuid::now_v7();
        let extension =
            ImageExtension::try_from(image).expect("factory cover must be a supported image");

        sqlx::query!(
            r#"
insert into book_covers(id, book_id, extension, is_main)
values($1, $2, $3, false);
        "#,
            id,
            book_id,
            extension.as_ref()
        )
        .execute(&self.pool)
        .await
        .expect("failed to insert factory book cover");

        self.storage_put_image(
            &self.covers_bucket,
            id,
            extension,
            ByteStream::from_static(image),
        )
        .await;

        id
    }

    pub async fn db_insert_chapter(&self, c: &Chapter) {
        sqlx::query!(
            r#"
insert into chapters(id, book_id, number, name, volume, updated_at, created_at)
values($1, $2, $3, $4, $5, $6, $7);
        "#,
            c.id,
            c.book_id,
            c.number.as_f32(),
            c.name.as_ref().map(AsRef::as_ref),
            c.volume.map(ChapterVolume::as_i16),
            c.updated_at,
            c.created_at
        )
        .execute(&self.pool)
        .await
        .expect("failed to insert factory chapter");

        let mut language_ids: Vec<Uuid> = Vec::with_capacity(c.localizations.len());
        let mut names: Vec<String> = Vec::with_capacity(c.localizations.len());
        for l in c.localizations.as_slice() {
            language_ids.push(l.language_id);
            names.push(l.name.as_ref().to_owned());
        }
        sqlx::query!(
            r#"
insert into chapter_localizations(chapter_id, language_id, name)
select $1, localization.language_id, localization.name
from unnest($2::uuid[], $3::text[]) as localization(language_id, name);
        "#,
            c.id,
            &language_ids,
            &names
        )
        .execute(&self.pool)
        .await
        .expect("failed to insert factory chapter localizations");
    }

    pub async fn db_insert_numbered_chapters(&self, book_id: Uuid, numbers: &[f32]) {
        for number in numbers {
            let mut chapter: Chapter = ChapterFaker {
                book_id,
                localizations: 0..=0,
            }
            .fake();
            chapter.number =
                ChapterNumber::try_from(*number).expect("factory number must be valid");

            self.db_insert_chapter(&chapter).await;
        }
    }

    pub async fn db_insert_random_chapter(&self, book_id: Uuid) -> Uuid {
        let chapter: Chapter = ChapterFaker::new(book_id).fake();
        self.db_insert_chapter(&chapter).await;

        chapter.id
    }

    pub async fn db_fetch_chapter(&self, id: Uuid) -> Chapter {
        let row = sqlx::query!(
            r#"
select book_id, number, name, volume, updated_at, created_at
from chapters
where id = $1;
        "#,
            id
        )
        .fetch_one(&self.pool)
        .await
        .expect("failed to read chapter");

        let localizations: Vec<ChapterLocalization> = sqlx::query!(
            r#"
select language_id, name
from chapter_localizations
where chapter_id = $1
order by language_id;
        "#,
            id
        )
        .fetch_all(&self.pool)
        .await
        .expect("failed to read chapter localizations")
        .into_iter()
        .map(|r| ChapterLocalization {
            language_id: r.language_id,
            name: ChapterName::try_from(r.name).expect("stored localization name must be valid"),
        })
        .collect();

        Chapter {
            id,
            book_id: row.book_id,
            number: ChapterNumber::try_from(row.number).expect("stored number must be valid"),
            name: row
                .name
                .map(|n| ChapterName::try_from(n).expect("stored name must be valid")),
            volume: row
                .volume
                .map(|v| ChapterVolume::try_from(v).expect("stored volume must be valid")),
            localizations: localizations
                .try_into()
                .expect("too many localizations in test fixture"),
            updated_at: row.updated_at,
            created_at: row.created_at,
        }
    }

    pub async fn db_fetch_chapter_sample(&self, id: Uuid) -> ChapterSample {
        let row = sqlx::query!(
            r#"
select (select c.number from chapters c where c.id = $1) as "number?",
       (select c.name from chapters c where c.id = $1) as "name?",
       (select c.updated_at from chapters c where c.id = $1) as "updated_at?",
       (select count(*) from chapter_localizations cl where cl.chapter_id = $1) as "localizations!";
        "#,
            id
        )
        .fetch_one(&self.pool)
        .await
        .expect("failed to read chapter sample");

        ChapterSample {
            number: row.number,
            name: row.name,
            updated_at: row.updated_at,
            localizations: row.localizations,
        }
    }

    pub async fn db_insert_creator(&self, c: &Creator) {
        sqlx::query!(
            r#"
insert into creators(id, first_name, last_name, created_at)
values($1, $2, $3, $4);
        "#,
            c.id,
            c.first_name.as_ref(),
            c.last_name.as_ref(),
            c.created_at
        )
        .execute(&self.pool)
        .await
        .expect("failed to insert factory creator");
    }

    pub async fn db_credit_creator(&self, book_id: Uuid, creator_id: Uuid, role: CreatorRole) {
        sqlx::query!(
            r#"
insert into book_creators(book_id, creator_id, role)
values($1, $2, $3);
        "#,
            book_id,
            creator_id,
            role.as_ref()
        )
        .execute(&self.pool)
        .await
        .expect("failed to insert factory book creator credit");
    }

    pub async fn db_insert_feedback(&self, f: &Feedback) {
        sqlx::query!(
            r#"
insert into feedback(id, kind, status, email, note, book_id, updated_at, created_at)
values($1, $2, $3, $4, $5, $6, $7, $8);
        "#,
            f.id,
            f.kind.as_ref(),
            f.status.as_ref(),
            f.email.as_ref(),
            f.note.as_ref(),
            f.book_id,
            f.updated_at,
            f.created_at
        )
        .execute(&self.pool)
        .await
        .expect("failed to insert factory feedback");
    }

    pub async fn db_fetch_feedback(&self, id: Uuid) -> Feedback {
        let row = sqlx::query!(
            r#"
select f.id, f.kind, f.status, f.email, f.note, f.book_id, f.updated_at, f.created_at
from feedback f
where f.id = $1;
        "#,
            id
        )
        .fetch_one(&self.pool)
        .await
        .expect("failed to read feedback");

        Feedback {
            id: row.id,
            kind: row.kind.parse().unwrap(),
            status: row.status.parse().unwrap(),
            email: Email::try_from(row.email).unwrap(),
            note: Text::try_from(row.note).unwrap(),
            book_id: row.book_id,
            updated_at: row.updated_at,
            created_at: row.created_at,
        }
    }

    pub async fn db_count_feedback(&self) -> i64 {
        sqlx::query_scalar!(r#"select count(*) as "count!" from feedback"#)
            .fetch_one(&self.pool)
            .await
            .expect("failed to count feedback")
    }

    async fn db_seed_books<S>(
        &self,
        specs: impl IntoIterator<Item = S>,
        build: impl Fn(usize, S) -> BookFaker,
    ) -> Vec<BookQuery> {
        let mut books = Vec::new();
        for (i, spec) in specs.into_iter().enumerate() {
            let book: BookQuery = build(i, spec).fake();
            self.fixture_insert_book(&book).await;
            books.push(book);
        }

        books
    }

    pub async fn db_seed_facet_corpus(&self) -> Vec<BookQuery> {
        const ROWS: [FacetRow; 12] = [
            (
                &[ACTION, ROMANCE],
                BookKind::Manga,
                BookStatus::Ongoing,
                PublicationDemographic::Shounen,
                0,
                0,
            ),
            (
                &[ACTION],
                BookKind::Manga,
                BookStatus::Completed,
                PublicationDemographic::Shounen,
                0,
                0,
            ),
            (
                &[FANTASY, ISEKAI],
                BookKind::Manhwa,
                BookStatus::Ongoing,
                PublicationDemographic::Seinen,
                1,
                1,
            ),
            (
                &[ACTION, ROMANCE, FANTASY],
                BookKind::Manhwa,
                BookStatus::Completed,
                PublicationDemographic::Josei,
                1,
                1,
            ),
            (
                &[FANTASY, LONG_STRIP],
                BookKind::Manhua,
                BookStatus::Hiatus,
                PublicationDemographic::Shoujo,
                2,
                2,
            ),
            (
                &[ROMANCE],
                BookKind::Manhua,
                BookStatus::Cancelled,
                PublicationDemographic::Kids,
                2,
                2,
            ),
            (
                &[],
                BookKind::Manga,
                BookStatus::Ongoing,
                PublicationDemographic::Seinen,
                3,
                3,
            ),
            (
                &[MAFIA, ZOMBIES],
                BookKind::Manhwa,
                BookStatus::Hiatus,
                PublicationDemographic::Shoujo,
                3,
                3,
            ),
            (
                &[SCHOOL_LIFE],
                BookKind::Manhua,
                BookStatus::Completed,
                PublicationDemographic::Josei,
                0,
                4,
            ),
            (
                &[ACTION, FANTASY],
                BookKind::Manga,
                BookStatus::Cancelled,
                PublicationDemographic::Kids,
                1,
                4,
            ),
            (
                &[ISEKAI],
                BookKind::Manhwa,
                BookStatus::Ongoing,
                PublicationDemographic::Shounen,
                2,
                0,
            ),
            (
                &[ZOMBIES],
                BookKind::Manhua,
                BookStatus::Completed,
                PublicationDemographic::Seinen,
                3,
                1,
            ),
        ];

        self.db_seed_books(
            ROWS,
            |i, (label_ids, kind, status, demographic, rating, language)| BookFaker {
                exact_labels: Some(labels(label_ids)),
                kind: Some(kind),
                status: Some(status),
                publication_demographic: Some(demographic),
                content_rating: Some(CONTENT_RATINGS[rating].clone()),
                publication_language: Some(LANGUAGES[language].clone()),
                publication_year: Some(2000 + i as i16),
                created_at: Some(day(i as i64)),
                ..BookFaker::scalar()
            },
        )
        .await
    }

    pub async fn db_seed_range_corpus(&self) -> Vec<BookQuery> {
        const YEARS: [i16; 5] = [2010, 2012, 2015, 2018, 2020];

        self.db_seed_books(YEARS, |_, year| BookFaker {
            publication_year: Some(year),
            created_at: Some(day(i64::from(year) - 2000)),
            ..BookFaker::scalar()
        })
        .await
    }

    pub async fn db_seed_sort_corpus(&self) -> Vec<BookQuery> {
        const ROWS: [(&str, i16, i64); 4] = [
            ("Alpha", 2020, 4),
            ("Bravo", 2010, 1),
            ("Charlie", 2015, 3),
            ("Delta", 2005, 2),
        ];

        self.db_seed_books(ROWS, |_, (name, year, created)| BookFaker {
            name: Some(name.to_owned()),
            publication_year: Some(year),
            created_at: Some(day(created)),
            ..BookFaker::scalar()
        })
        .await
    }

    pub async fn db_seed_tie_corpus(&self, n: usize) -> Vec<BookQuery> {
        let mut books = self
            .db_seed_books(0..n, |_, _| BookFaker {
                name: Some("Tied".to_owned()),
                publication_year: Some(2020),
                created_at: Some(day(0)),
                ..BookFaker::scalar()
            })
            .await;

        books.sort_by_key(|b| b.id);

        books
    }

    pub async fn db_seed_translated_corpus(&self) -> Vec<BookQuery> {
        let japanese = LANGUAGES[0].clone();
        let english = LANGUAGES[3].clone();
        let russian = LANGUAGES[4].clone();

        let rows: [(Language, BookVisibility, Vec<Language>); 6] = [
            (
                japanese.clone(),
                BookVisibility::Listed,
                vec![english.clone()],
            ),
            (
                japanese.clone(),
                BookVisibility::Listed,
                vec![english.clone(), english.clone()],
            ),
            (japanese.clone(), BookVisibility::Listed, vec![russian]),
            (english.clone(), BookVisibility::Listed, vec![]),
            (japanese.clone(), BookVisibility::Listed, vec![]),
            (japanese, BookVisibility::Hidden, vec![english]),
        ];

        let mut books = Vec::with_capacity(rows.len());
        for (language, visibility, releases) in rows {
            let book: BookQuery = BookFaker {
                publication_language: Some(language),
                visibility,
                ..BookFaker::scalar()
            }
            .fake();

            self.fixture_insert_book(&book).await;

            for release_language in releases {
                let chapter_id = self.db_insert_random_chapter(book.id).await;
                self.db_insert_release(chapter_id, release_language.id)
                    .await;
            }

            books.push(book);
        }

        books
    }

    pub async fn db_non_publication_language(&self, book_id: Uuid) -> Uuid {
        let publication = sqlx::query_scalar!(
            r#"
select publication_language
from books
where id = $1;
        "#,
            book_id
        )
        .fetch_one(&self.pool)
        .await
        .expect("failed to read book publication language");

        LANGUAGES
            .iter()
            .map(|l| l.id)
            .find(|id| *id != publication)
            .expect("the language catalog must hold a translation language")
    }

    pub async fn db_insert_release(&self, chapter_id: Uuid, language_id: Uuid) -> Uuid {
        self.db_insert_release_as(chapter_id, language_id, self.caller_id)
            .await
    }

    pub async fn db_insert_release_as(
        &self,
        chapter_id: Uuid,
        language_id: Uuid,
        created_by: Uuid,
    ) -> Uuid {
        let id = Uuid::now_v7();

        sqlx::query!(
            r#"
insert into chapter_releases(id, chapter_id, book_id, language_id, version, created_by)
select $1, $2, c.book_id, $3, 1, $4
from chapters c
where c.id = $2;
        "#,
            id,
            chapter_id,
            language_id,
            created_by
        )
        .execute(&self.pool)
        .await
        .expect("failed to insert factory chapter release");

        id
    }

    pub async fn db_insert_random_release(&self, book_id: Uuid, chapter_id: Uuid) -> Uuid {
        let language_id = self.db_non_publication_language(book_id).await;

        self.db_insert_release(chapter_id, language_id).await
    }

    pub async fn db_seed_caller(&self) {
        let mut user: UserQuery = UserFaker {
            roles: DEFAULT_ROLES.to_vec(),
            verified: true,
        }
        .fake();
        user.id = self.caller_id;

        self.db_insert_user(&user).await;
    }

    pub async fn fixture_insert_page(
        &self,
        release_id: Uuid,
        sort_order: Option<i32>,
        image: &[u8],
    ) -> Uuid {
        let id = Uuid::now_v7();
        let extension =
            ImageExtension::try_from(image).expect("factory page must be a supported image");

        sqlx::query!(
            r#"
insert into chapter_pages(id, release_id, book_id, sort_order, extension)
select $1, $2, cr.book_id, $3, $4
from chapter_releases cr
where cr.id = $2;
        "#,
            id,
            release_id,
            sort_order,
            extension.as_ref()
        )
        .execute(&self.pool)
        .await
        .expect("failed to insert factory chapter page");

        self.storage_put_image(
            &self.release_pages_bucket,
            id,
            extension,
            ByteStream::from(image.to_vec()),
        )
        .await;

        id
    }

    pub async fn fixture_insert_staged_pages(
        &self,
        release_id: Uuid,
        parts: &[&[u8]],
    ) -> Vec<Uuid> {
        let mut ids = Vec::with_capacity(parts.len());
        for part in parts {
            ids.push(self.fixture_insert_page(release_id, None, part).await);
        }

        ids
    }

    pub async fn db_fetch_release_version(&self, release_id: Uuid) -> i32 {
        sqlx::query_scalar!(
            r#"
select version
from chapter_releases
where id = $1;
        "#,
            release_id
        )
        .fetch_one(&self.pool)
        .await
        .expect("failed to read chapter release version")
    }

    pub async fn db_fetch_release_id(&self, release_id: Uuid) -> Option<Uuid> {
        sqlx::query_scalar!(
            r#"
select id
from chapter_releases
where id = $1;
        "#,
            release_id
        )
        .fetch_optional(&self.pool)
        .await
        .expect("failed to read chapter release id")
    }

    pub async fn db_fetch_page_order(&self, release_id: Uuid) -> Vec<(Uuid, Option<i32>)> {
        sqlx::query!(
            r#"
select id, sort_order
from chapter_pages
where release_id = $1
order by sort_order nulls last, id;
        "#,
            release_id
        )
        .fetch_all(&self.pool)
        .await
        .expect("failed to read chapter pages")
        .into_iter()
        .map(|r| (r.id, r.sort_order))
        .collect()
    }

    pub async fn db_fetch_committed_page_ids(&self, release_id: Uuid) -> Vec<Uuid> {
        sqlx::query_scalar!(
            r#"
select id
from chapter_pages
where release_id = $1 and sort_order is not null
order by sort_order;
        "#,
            release_id
        )
        .fetch_all(&self.pool)
        .await
        .expect("failed to read committed chapter page order")
    }
}
