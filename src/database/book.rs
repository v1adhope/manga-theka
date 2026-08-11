use std::collections::HashMap;

use sqlx::{PgConnection, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::{
    database::{Database, creator::CreatorRow, label::LabelRow},
    entity::{
        AlternativeTitle, Book, BookKind, BookLink, BookLinkKind, BookName, BookStatus, Creator,
        DEFAULT_LIMIT, Description, Label, Limit, LinkUrl, Pagination,
    },
    error::DatabaseError,
};

#[derive(sqlx::FromRow)]
struct BookRow {
    id: Uuid,
    name: String,
    description: String,
    publication_year: i16,
    content_rating: Uuid,
    status: String,
    kind: String,
    publication_language: Uuid,
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

impl TryFrom<BookWithRelations> for Book {
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

        Ok(Book {
            id: row.id,
            name,
            description,
            publication_year: row.publication_year,
            content_rating: row.content_rating,
            status,
            kind,
            publication_language: row.publication_language,
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
    pub async fn store_book(&self, item: &Book, label_ids: &[Uuid]) -> Result<(), DatabaseError> {
        self.store_book_inner(item, label_ids)
            .await
            .inspect_err(|e| {
                if DatabaseError::is_internal(e) {
                    tracing::error!("failed to store new book in database: {e:?}");
                }
            })
    }

    async fn store_book_inner(&self, item: &Book, label_ids: &[Uuid]) -> Result<(), DatabaseError> {
        let mut tx = self.pool.begin().await?;

        sqlx::query_file!(
            "queries/store_book.sql",
            item.id,
            item.name.as_ref(),
            item.description.as_ref(),
            item.publication_year,
            item.content_rating,
            item.status.as_ref(),
            item.kind.as_ref(),
            item.publication_language,
            item.updated_at,
            item.created_at,
        )
        .execute(&mut *tx)
        .await?;

        Self::store_book_relations(&mut tx, item, label_ids).await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn update_book(&self, item: &Book, label_ids: &[Uuid]) -> Result<(), DatabaseError> {
        self.update_book_inner(item, label_ids)
            .await
            .inspect_err(|e| {
                if DatabaseError::is_internal(e) {
                    tracing::error!("failed to update book in database: {e:?}");
                }
            })
    }

    async fn update_book_inner(
        &self,
        item: &Book,
        label_ids: &[Uuid],
    ) -> Result<(), DatabaseError> {
        let mut tx = self.pool.begin().await?;

        let row = sqlx::query_file!(
            "queries/update_book.sql",
            item.id,
            item.name.as_ref(),
            item.description.as_ref(),
            item.publication_year,
            item.content_rating,
            item.status.as_ref(),
            item.kind.as_ref(),
            item.publication_language,
            item.updated_at,
        )
        .fetch_optional(&mut *tx)
        .await?;

        if row.is_none() {
            return Err(DatabaseError::BookNotFound);
        }

        Self::delete_book_relations(&mut tx, item.id).await?;
        Self::store_book_relations(&mut tx, item, label_ids).await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn get_book(&self, id: Uuid) -> Result<Book, DatabaseError> {
        self.get_book_inner(id).await.inspect_err(|e| {
            if DatabaseError::is_internal(e) {
                tracing::error!("failed to get book from database: {e:?}");
            }
        })
    }

    async fn get_book_inner(&self, id: Uuid) -> Result<Book, DatabaseError> {
        let row = sqlx::query_file_as!(BookRow, "queries/get_book.sql", id)
            .fetch_optional(&self.pool)
            .await?;

        let Some(row) = row else {
            return Err(DatabaseError::BookNotFound);
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
        .inspect_err(|e| {
            if DatabaseError::is_internal(e) {
                tracing::error!("failed to convert book row with relations: {e:?}");
            }
        })
    }

    pub async fn get_books(
        &self,
        pagination: &Pagination,
    ) -> Result<(Vec<Book>, Option<Uuid>), DatabaseError> {
        self.get_books_inner(pagination).await.inspect_err(|e| {
            if DatabaseError::is_internal(e) {
                tracing::error!("failed to get books from database: {e:?}");
            }
        })
    }

    async fn get_books_inner(
        &self,
        pagination: &Pagination,
    ) -> Result<(Vec<Book>, Option<Uuid>), DatabaseError> {
        let limit = pagination.limit.map_or(DEFAULT_LIMIT, Limit::as_u32);
        let mut builder: QueryBuilder<Postgres> = QueryBuilder::new(
            r"select id, name, description, publication_year, content_rating, status, kind,
                     publication_language, updated_at, created_at
              from books",
        );

        if let Some(id) = pagination.after {
            builder.push(" where id < ").push_bind(id);
        }

        builder
            .push(" order by id desc limit ")
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

        let mut books: Vec<Book> = Vec::with_capacity(rows.len());
        for row in rows {
            let id = row.id;
            let book: Book = BookWithRelations {
                row,
                labels: labels.remove(&id).unwrap_or_default(),
                links: links.remove(&id).unwrap_or_default(),
                titles: titles.remove(&id).unwrap_or_default(),
                creators: creators.remove(&id).unwrap_or_default(),
            }
            .try_into()
            .inspect_err(|e| {
                if DatabaseError::is_internal(e) {
                    tracing::error!("failed to convert book row with relations: {e:?}");
                }
            })?;
            books.push(book);
        }

        Ok((books, next_cursor))
    }

    pub async fn delete_book(&self, id: Uuid) -> Result<(), DatabaseError> {
        let row = sqlx::query_file!("queries/delete_book.sql", id)
            .fetch_optional(&self.pool)
            .await
            .map_err(DatabaseError::from)
            .inspect_err(|e| {
                if DatabaseError::is_internal(e) {
                    tracing::error!("failed to delete book from database: {e:?}");
                }
            })?;

        if row.is_none() {
            return Err(DatabaseError::BookNotFound);
        }

        Ok(())
    }

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
            let label: Label = row.try_into().inspect_err(|e| {
                if DatabaseError::is_internal(e) {
                    tracing::error!("failed to convert book label row: {e:?}");
                }
            })?;
            labels.entry(book_id).or_default().push(label);
        }

        Ok(labels)
    }

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
            let link: BookLink = row.try_into().inspect_err(|e| {
                if DatabaseError::is_internal(e) {
                    tracing::error!("failed to convert book link row: {e:?}");
                }
            })?;
            links.entry(book_id).or_default().push(link);
        }

        Ok(links)
    }

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
            let title: AlternativeTitle = row.try_into().inspect_err(|e| {
                if DatabaseError::is_internal(e) {
                    tracing::error!("failed to convert book title row: {e:?}");
                }
            })?;
            titles.entry(book_id).or_default().push(title);
        }

        Ok(titles)
    }

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
            let creator: Creator = row.try_into().inspect_err(|e| {
                if DatabaseError::is_internal(e) {
                    tracing::error!("failed to convert book creator row: {e:?}");
                }
            })?;
            creators.entry(book_id).or_default().push(creator);
        }

        Ok(creators)
    }

    async fn store_book_relations(
        conn: &mut PgConnection,
        item: &Book,
        label_ids: &[Uuid],
    ) -> Result<(), DatabaseError> {
        sqlx::query_file!("queries/store_book_labels.sql", item.id, label_ids)
            .execute(&mut *conn)
            .await?;

        let mut link_kinds: Vec<String> = Vec::with_capacity(item.links.len());
        let mut link_urls: Vec<String> = Vec::with_capacity(item.links.len());
        for link in &item.links {
            link_kinds.push(link.kind.as_ref().to_owned());
            link_urls.push(link.url.as_ref().to_owned());
        }
        sqlx::query_file!(
            "queries/store_book_links.sql",
            item.id,
            &link_kinds,
            &link_urls
        )
        .execute(&mut *conn)
        .await?;

        let mut title_language_ids: Vec<Uuid> = Vec::with_capacity(item.titles.len());
        let mut title_names: Vec<String> = Vec::with_capacity(item.titles.len());
        for title in &item.titles {
            title_language_ids.push(title.language_id);
            title_names.push(title.name.as_ref().to_owned());
        }
        sqlx::query_file!(
            "queries/store_book_titles.sql",
            item.id,
            &title_language_ids,
            &title_names
        )
        .execute(&mut *conn)
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
}
