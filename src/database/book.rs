use std::collections::HashMap;

use sqlx::{PgConnection, Postgres, QueryBuilder};
use tracing::{Level, instrument};
use uuid::Uuid;

use crate::{
    database::{Database, creator::CreatorRow, label::LabelRow},
    entity::{
        AlternativeTitle, Book, BookCover, BookCoverQuery, BookCreators, BookKind, BookLabels,
        BookLink, BookLinkKind, BookLinks, BookName, BookQuery, BookStatus, BookTitles,
        ContentRating, CoverUrl, Creator, DEFAULT_LIMIT, Description, Filter, ImageExtension,
        Label, Language, Limit, LinkUrl,
    },
    error::DatabaseError,
};

#[derive(sqlx::FromRow)]
struct BookRow {
    id: Uuid,
    name: String,
    description: String,
    publication_year: i16,
    content_rating_id: Uuid,
    content_rating_name: String,
    content_rating_code: String,
    status: String,
    kind: String,
    publication_language_id: Uuid,
    publication_language_code: String,
    publication_language_name: String,
    updated_at: Option<time::OffsetDateTime>,
    created_at: time::OffsetDateTime,
}

struct BookLabelRow {
    book_id: Uuid,
    id: Uuid,
    name: String,
    kind: String,
}

impl TryFrom<BookLabelRow> for Label {
    type Error = DatabaseError;

    fn try_from(row: BookLabelRow) -> Result<Self, Self::Error> {
        LabelRow {
            id: row.id,
            name: row.name,
            kind: row.kind,
        }
        .try_into()
    }
}

struct BookLinkRow {
    book_id: Uuid,
    kind: String,
    url: String,
}

impl TryFrom<BookLinkRow> for BookLink {
    type Error = DatabaseError;

    fn try_from(row: BookLinkRow) -> Result<Self, Self::Error> {
        let kind: BookLinkKind = row
            .kind
            .parse()
            .map_err(|e| DatabaseError::invariant_corrupted("kind", e))?;
        let url =
            LinkUrl::try_from(row.url).map_err(|e| DatabaseError::invariant_corrupted("url", e))?;

        Ok(BookLink { kind, url })
    }
}

struct BookTitleRow {
    book_id: Uuid,
    language_id: Uuid,
    name: String,
}

impl TryFrom<BookTitleRow> for AlternativeTitle {
    type Error = DatabaseError;

    fn try_from(row: BookTitleRow) -> Result<Self, Self::Error> {
        let name = BookName::try_from(row.name)
            .map_err(|e| DatabaseError::invariant_corrupted("name", e))?;

        Ok(AlternativeTitle {
            language_id: row.language_id,
            name,
        })
    }
}

struct BookCoverRow {
    id: Uuid,
    book_id: Uuid,
    extension: String,
    is_main: bool,
}

impl TryFrom<BookCoverRow> for BookCoverQuery {
    type Error = DatabaseError;

    fn try_from(row: BookCoverRow) -> Result<Self, Self::Error> {
        let url = CoverUrl { cover_id: row.id }.into();
        let extension: ImageExtension = row
            .extension
            .parse()
            .map_err(|e| DatabaseError::invariant_corrupted("extension", e))?;

        Ok(BookCoverQuery {
            url,
            id: row.id,
            book_id: row.book_id,
            extension,
            is_main: row.is_main,
        })
    }
}

struct BookCreatorRow {
    book_id: Uuid,
    id: Uuid,
    first_name: String,
    last_name: String,
    role: String,
    created_at: time::OffsetDateTime,
}

impl TryFrom<BookCreatorRow> for Creator {
    type Error = DatabaseError;

    fn try_from(row: BookCreatorRow) -> Result<Self, Self::Error> {
        CreatorRow {
            id: row.id,
            first_name: row.first_name,
            last_name: row.last_name,
            role: row.role,
            created_at: row.created_at,
        }
        .try_into()
    }
}

struct BookWithRelations {
    row: BookRow,
    labels: Vec<Label>,
    links: Vec<BookLink>,
    titles: Vec<AlternativeTitle>,
    creators: Vec<Creator>,
}

impl TryFrom<BookWithRelations> for BookQuery {
    type Error = DatabaseError;

    fn try_from(item: BookWithRelations) -> Result<Self, Self::Error> {
        let BookWithRelations {
            row,
            labels,
            links,
            titles,
            creators,
        } = item;

        let name = BookName::try_from(row.name)
            .map_err(|e| DatabaseError::invariant_corrupted("name", e))?;
        let description = Description::try_from(row.description)
            .map_err(|e| DatabaseError::invariant_corrupted("description", e))?;
        let status: BookStatus = row
            .status
            .parse()
            .map_err(|e| DatabaseError::invariant_corrupted("status", e))?;
        let kind: BookKind = row
            .kind
            .parse()
            .map_err(|e| DatabaseError::invariant_corrupted("kind", e))?;
        let labels = BookLabels::try_from(labels)
            .map_err(|e| DatabaseError::invariant_corrupted("labels", e))?;
        let links = BookLinks::try_from(links)
            .map_err(|e| DatabaseError::invariant_corrupted("links", e))?;
        let titles = BookTitles::try_from(titles)
            .map_err(|e| DatabaseError::invariant_corrupted("titles", e))?;
        let creators = BookCreators::try_from(creators)
            .map_err(|e| DatabaseError::invariant_corrupted("creators", e))?;

        Ok(BookQuery {
            id: row.id,
            name,
            description,
            publication_year: row.publication_year,
            content_rating: ContentRating {
                id: row.content_rating_id,
                name: row.content_rating_name,
                code: row.content_rating_code,
            },
            status,
            kind,
            publication_language: Language {
                id: row.publication_language_id,
                code: row.publication_language_code,
                name: row.publication_language_name,
            },
            labels,
            links,
            titles,
            creators,
            updated_at: row.updated_at,
            created_at: row.created_at,
        })
    }
}

impl Database {
    #[instrument(name = "db.book.store", skip_all, fields(book.id = %item.id))]
    pub async fn store_book(&self, item: &Book) -> Result<(), DatabaseError> {
        self.store_book_inner(item)
            .await
            .inspect_err(DatabaseError::log_internal)
    }

    async fn store_book_inner(&self, item: &Book) -> Result<(), DatabaseError> {
        let mut tx = self.pool.begin().await?;

        sqlx::query_file!(
            "queries/store_book.sql",
            item.id,
            item.name.as_ref(),
            item.description.as_ref(),
            item.publication_year,
            item.content_rating_id,
            item.status.as_ref(),
            item.kind.as_ref(),
            item.publication_language_id,
            item.updated_at,
            item.created_at,
        )
        .execute(&mut *tx)
        .await?;

        Self::store_book_relations(&mut tx, item).await?;

        tx.commit().await?;
        Ok(())
    }

    #[instrument(name = "db.book.update", skip_all, fields(book.id = %item.id))]
    pub async fn update_book(&self, item: &Book) -> Result<(), DatabaseError> {
        self.update_book_inner(item)
            .await
            .inspect_err(DatabaseError::log_internal)
    }

    async fn update_book_inner(&self, item: &Book) -> Result<(), DatabaseError> {
        let mut tx = self.pool.begin().await?;

        let row = sqlx::query_file!(
            "queries/update_book.sql",
            item.id,
            item.name.as_ref(),
            item.description.as_ref(),
            item.publication_year,
            item.content_rating_id,
            item.status.as_ref(),
            item.kind.as_ref(),
            item.publication_language_id,
            item.updated_at,
        )
        .fetch_optional(&mut *tx)
        .await?;

        if row.is_none() {
            return Err(DatabaseError::not_found::<Book>());
        }

        Self::delete_book_relations(&mut tx, item.id).await?;
        Self::store_book_relations(&mut tx, item).await?;

        tx.commit().await?;
        Ok(())
    }

    #[instrument(name = "db.book.get", skip_all, fields(book.id = %id))]
    pub async fn get_book(&self, id: Uuid) -> Result<BookQuery, DatabaseError> {
        self.get_book_inner(id)
            .await
            .inspect_err(DatabaseError::log_internal)
    }

    async fn get_book_inner(&self, id: Uuid) -> Result<BookQuery, DatabaseError> {
        let row = sqlx::query_file_as!(BookRow, "queries/get_book.sql", id)
            .fetch_optional(&self.pool)
            .await?;

        let Some(row) = row else {
            return Err(DatabaseError::not_found::<Book>());
        };

        let ids = [id];
        let mut labels = self.get_books_labels(&ids).await?;
        let mut links = self.get_books_links(&ids).await?;
        let mut titles = self.get_books_titles(&ids).await?;
        let mut creators = self.get_books_creators(&ids).await?;

        BookWithRelations {
            row,
            labels: labels.remove(&id).unwrap_or_default(),
            links: links.remove(&id).unwrap_or_default(),
            titles: titles.remove(&id).unwrap_or_default(),
            creators: creators.remove(&id).unwrap_or_default(),
        }
        .try_into()
    }

    #[instrument(
        name = "db.book.list",
        skip_all,
        fields(filter = ?filter)
    )]
    pub async fn get_books(
        &self,
        filter: &Filter,
    ) -> Result<(Vec<BookQuery>, Option<Uuid>), DatabaseError> {
        self.get_books_inner(filter)
            .await
            .inspect_err(DatabaseError::log_internal)
    }

    async fn get_books_inner(
        &self,
        filter: &Filter,
    ) -> Result<(Vec<BookQuery>, Option<Uuid>), DatabaseError> {
        let limit = filter.limit.map_or(DEFAULT_LIMIT, Limit::as_u32);
        let mut builder: QueryBuilder<Postgres> = QueryBuilder::new(
            r"select b.id, b.name, b.description, b.publication_year,
                     cr.id as content_rating_id, cr.name as content_rating_name,
                     cr.code as content_rating_code, b.status, b.kind,
                     l.id as publication_language_id, l.code as publication_language_code,
                     l.name as publication_language_name, b.updated_at, b.created_at
              from books b
              join content_ratings cr on cr.id = b.content_rating
              join languages l on l.id = b.publication_language",
        );

        if let Some(id) = filter.after {
            builder.push(" where b.id < ").push_bind(id);
        }

        builder
            .push(" order by b.id desc limit ")
            .push_bind((limit + 1) as i64);

        let mut rows = builder
            .build_query_as::<BookRow>()
            .fetch_all(&self.pool)
            .await?;

        if rows.is_empty() {
            return Ok((Vec::new(), None));
        }

        let mut next_cursor = None;
        if rows.len() > limit as usize {
            rows.pop();
            next_cursor = rows.last().map(|r| r.id);
        }

        let ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
        let mut labels = self.get_books_labels(&ids).await?;
        let mut links = self.get_books_links(&ids).await?;
        let mut titles = self.get_books_titles(&ids).await?;
        let mut creators = self.get_books_creators(&ids).await?;

        let mut books: Vec<BookQuery> = Vec::with_capacity(rows.len());
        for row in rows {
            let id = row.id;
            let book: BookQuery = BookWithRelations {
                row,
                labels: labels.remove(&id).unwrap_or_default(),
                links: links.remove(&id).unwrap_or_default(),
                titles: titles.remove(&id).unwrap_or_default(),
                creators: creators.remove(&id).unwrap_or_default(),
            }
            .try_into()?;
            books.push(book);
        }

        Ok((books, next_cursor))
    }

    #[instrument(name = "db.book.delete", skip_all, fields(book.id = %id))]
    pub async fn delete_book(&self, id: Uuid) -> Result<(), DatabaseError> {
        let row = sqlx::query_file!("queries/delete_book.sql", id)
            .fetch_optional(&self.pool)
            .await
            .map_err(DatabaseError::from)
            .inspect_err(DatabaseError::log_internal)?;

        if row.is_none() {
            return Err(DatabaseError::not_found::<Book>());
        }

        Ok(())
    }

    #[instrument(name = "db.book.exists", skip_all, fields(book.id = %id))]
    pub async fn ensure_book_exists(&self, id: Uuid) -> Result<(), DatabaseError> {
        let row = sqlx::query_file!("queries/book_exists.sql", id)
            .fetch_one(&self.pool)
            .await
            .map_err(DatabaseError::from)
            .inspect_err(DatabaseError::log_internal)?;

        if !row.exists {
            return Err(DatabaseError::not_found::<Book>());
        }

        Ok(())
    }

    #[instrument(name = "db.book.labels", skip_all, level = Level::DEBUG, fields(books = book_ids.len()))]
    async fn get_books_labels(
        &self,
        book_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, Vec<Label>>, DatabaseError> {
        let rows = sqlx::query_file_as!(BookLabelRow, "queries/get_books_labels.sql", book_ids)
            .fetch_all(&self.pool)
            .await?;

        let mut labels: HashMap<Uuid, Vec<Label>> = HashMap::with_capacity(book_ids.len());
        for row in rows {
            let book_id = row.book_id;
            let label: Label = row.try_into()?;
            labels.entry(book_id).or_default().push(label);
        }

        Ok(labels)
    }

    #[instrument(name = "db.book.links", skip_all, level = Level::DEBUG, fields(books = book_ids.len()))]
    async fn get_books_links(
        &self,
        book_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, Vec<BookLink>>, DatabaseError> {
        let rows = sqlx::query_file_as!(BookLinkRow, "queries/get_books_links.sql", book_ids)
            .fetch_all(&self.pool)
            .await?;

        let mut links: HashMap<Uuid, Vec<BookLink>> = HashMap::with_capacity(book_ids.len());
        for row in rows {
            let book_id = row.book_id;
            let link: BookLink = row.try_into()?;
            links.entry(book_id).or_default().push(link);
        }

        Ok(links)
    }

    #[instrument(name = "db.book.titles", skip_all, level = Level::DEBUG, fields(books = book_ids.len()))]
    async fn get_books_titles(
        &self,
        book_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, Vec<AlternativeTitle>>, DatabaseError> {
        let rows = sqlx::query_file_as!(BookTitleRow, "queries/get_books_titles.sql", book_ids)
            .fetch_all(&self.pool)
            .await?;

        let mut titles: HashMap<Uuid, Vec<AlternativeTitle>> =
            HashMap::with_capacity(book_ids.len());
        for row in rows {
            let book_id = row.book_id;
            let title: AlternativeTitle = row.try_into()?;
            titles.entry(book_id).or_default().push(title);
        }

        Ok(titles)
    }

    #[instrument(name = "db.book.creators", skip_all, level = Level::DEBUG, fields(books = book_ids.len()))]
    async fn get_books_creators(
        &self,
        book_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, Vec<Creator>>, DatabaseError> {
        let rows = sqlx::query_file_as!(BookCreatorRow, "queries/get_books_creators.sql", book_ids)
            .fetch_all(&self.pool)
            .await?;

        let mut creators: HashMap<Uuid, Vec<Creator>> = HashMap::with_capacity(book_ids.len());
        for row in rows {
            let book_id = row.book_id;
            let creator: Creator = row.try_into()?;
            creators.entry(book_id).or_default().push(creator);
        }

        Ok(creators)
    }

    async fn store_book_relations(
        conn: &mut PgConnection,
        item: &Book,
    ) -> Result<(), DatabaseError> {
        Self::store_book_labels(&mut *conn, item).await?;
        Self::store_book_links(&mut *conn, item).await?;
        Self::store_book_titles(&mut *conn, item).await?;

        Ok(())
    }

    async fn store_book_labels(conn: &mut PgConnection, item: &Book) -> Result<(), DatabaseError> {
        if item.label_ids.is_empty() {
            return Ok(());
        }

        sqlx::query_file!(
            "queries/store_book_labels.sql",
            item.id,
            item.label_ids.as_slice()
        )
        .execute(conn)
        .await?;

        Ok(())
    }

    async fn store_book_links(conn: &mut PgConnection, item: &Book) -> Result<(), DatabaseError> {
        if item.links.is_empty() {
            return Ok(());
        }

        let mut kinds: Vec<String> = Vec::with_capacity(item.links.len());
        let mut urls: Vec<String> = Vec::with_capacity(item.links.len());
        for link in item.links.as_slice() {
            kinds.push(link.kind.as_ref().to_owned());
            urls.push(link.url.as_ref().to_owned());
        }

        sqlx::query_file!("queries/store_book_links.sql", item.id, &kinds, &urls)
            .execute(conn)
            .await?;

        Ok(())
    }

    async fn store_book_titles(conn: &mut PgConnection, item: &Book) -> Result<(), DatabaseError> {
        if item.titles.is_empty() {
            return Ok(());
        }

        let mut language_ids: Vec<Uuid> = Vec::with_capacity(item.titles.len());
        let mut names: Vec<String> = Vec::with_capacity(item.titles.len());
        for title in item.titles.as_slice() {
            language_ids.push(title.language_id);
            names.push(title.name.as_ref().to_owned());
        }

        sqlx::query_file!(
            "queries/store_book_titles.sql",
            item.id,
            &language_ids,
            &names
        )
        .execute(conn)
        .await?;

        Ok(())
    }

    async fn delete_book_relations(conn: &mut PgConnection, id: Uuid) -> Result<(), DatabaseError> {
        sqlx::query_file!("queries/delete_book_labels.sql", id)
            .execute(&mut *conn)
            .await?;
        sqlx::query_file!("queries/delete_book_links.sql", id)
            .execute(&mut *conn)
            .await?;
        sqlx::query_file!("queries/delete_book_titles.sql", id)
            .execute(&mut *conn)
            .await?;

        Ok(())
    }

    #[instrument(name = "db.book_cover.store", skip_all, fields(book.id = %item.book_id, cover.id = %item.image.id))]
    pub async fn store_book_cover(&self, item: &BookCover) -> Result<(), DatabaseError> {
        sqlx::query_file!(
            "queries/store_book_cover.sql",
            item.image.id,
            item.book_id,
            item.image.extension.as_ref(),
        )
        .execute(&self.pool)
        .await
        .map_err(DatabaseError::from)
        .inspect_err(DatabaseError::log_internal)?;

        Ok(())
    }

    #[instrument(name = "db.book_cover.list", skip_all, fields(book.id = %book_id))]
    pub async fn get_book_covers(
        &self,
        book_id: Uuid,
    ) -> Result<Vec<BookCoverQuery>, DatabaseError> {
        self.get_book_covers_inner(book_id)
            .await
            .inspect_err(DatabaseError::log_internal)
    }

    async fn get_book_covers_inner(
        &self,
        book_id: Uuid,
    ) -> Result<Vec<BookCoverQuery>, DatabaseError> {
        let rows = sqlx::query_file_as!(BookCoverRow, "queries/get_book_covers.sql", book_id)
            .fetch_all(&self.pool)
            .await?;

        let mut covers: Vec<BookCoverQuery> = Vec::with_capacity(rows.len());
        for row in rows {
            let cover: BookCoverQuery = row.try_into()?;
            covers.push(cover);
        }

        Ok(covers)
    }

    #[instrument(name = "db.book_cover.exists", skip_all, fields(cover.id = %id))]
    pub async fn ensure_book_cover_exists(&self, id: Uuid) -> Result<(), DatabaseError> {
        let row = sqlx::query_file!("queries/book_cover_exists.sql", id)
            .fetch_one(&self.pool)
            .await
            .map_err(DatabaseError::from)
            .inspect_err(DatabaseError::log_internal)?;

        if !row.exists {
            return Err(DatabaseError::not_found::<BookCover>());
        }

        Ok(())
    }

    #[instrument(name = "db.book_cover.ids", skip_all, fields(book.id = %book_id))]
    pub async fn get_book_cover_ids(&self, book_id: Uuid) -> Result<Vec<Uuid>, DatabaseError> {
        sqlx::query_file_scalar!("queries/get_book_cover_ids.sql", book_id)
            .fetch_all(&self.pool)
            .await
            .map_err(DatabaseError::from)
            .inspect_err(DatabaseError::log_internal)
    }

    #[instrument(name = "db.book_cover.delete", skip_all, fields(cover.id = %id))]
    pub async fn delete_book_cover(&self, id: Uuid) -> Result<(), DatabaseError> {
        let row = sqlx::query_file!("queries/delete_book_cover.sql", id)
            .fetch_optional(&self.pool)
            .await
            .map_err(DatabaseError::from)
            .inspect_err(DatabaseError::log_internal)?;

        if row.is_none() {
            return Err(DatabaseError::not_found::<BookCover>());
        }

        Ok(())
    }

    #[instrument(name = "db.book_cover.promote", skip_all, fields(book.id = %book_id, cover.id = %id))]
    pub async fn promote_book_cover(&self, book_id: Uuid, id: Uuid) -> Result<(), DatabaseError> {
        self.promote_book_cover_inner(book_id, id)
            .await
            .inspect_err(DatabaseError::log_internal)
    }

    async fn promote_book_cover_inner(&self, book_id: Uuid, id: Uuid) -> Result<(), DatabaseError> {
        let mut tx = self.pool.begin().await?;

        sqlx::query_file!("queries/demote_book_cover.sql", book_id)
            .execute(&mut *tx)
            .await?;

        let row = sqlx::query_file!("queries/promote_book_cover.sql", book_id, id)
            .fetch_optional(&mut *tx)
            .await?;

        if row.is_none() {
            return Err(DatabaseError::not_found::<BookCover>());
        }

        tx.commit().await?;
        Ok(())
    }
}
