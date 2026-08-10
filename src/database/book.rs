use sqlx::{PgConnection, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::{
    database::Database,
    entity::{
        AlternativeTitle, Book, BookDetails, BookKind, BookLink, BookLinkKind, BookName,
        BookStatus, BookWrite, Description, Label, LinkUrl, Pagination,
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
    author: Uuid,
    artist: Uuid,
    updated_at: Option<time::OffsetDateTime>,
    created_at: time::OffsetDateTime,
}

impl TryFrom<BookRow> for Book {
    type Error = DatabaseError;

    fn try_from(row: BookRow) -> Result<Self, Self::Error> {
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
            author: row.author,
            artist: row.artist,
            updated_at: row.updated_at,
            created_at: row.created_at,
        })
    }
}

impl Database {
    pub async fn store_book(&self, item: BookWrite) -> Result<(), DatabaseError> {
        self.store_book_inner(item).await.inspect_err(|e| {
            if DatabaseError::is_internal(e) {
                tracing::error!("failed to store new book in database: {e:?}");
            }
        })
    }

    async fn store_book_inner(&self, item: BookWrite) -> Result<(), DatabaseError> {
        let mut tx = self.pool.begin().await?;

        sqlx::query_file!(
            "queries/store_book.sql",
            item.book.id,
            item.book.name.as_ref(),
            item.book.description.as_ref(),
            item.book.publication_year,
            item.book.content_rating,
            item.book.status.as_ref(),
            item.book.kind.as_ref(),
            item.book.publication_language,
            item.book.author,
            item.book.artist,
            item.book.updated_at,
            item.book.created_at,
        )
        .execute(&mut *tx)
        .await?;

        store_book_relations(&mut tx, &item).await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn update_book(&self, item: BookWrite) -> Result<(), DatabaseError> {
        self.update_book_inner(item).await.inspect_err(|e| {
            if DatabaseError::is_internal(e) {
                tracing::error!("failed to update book in database: {e:?}");
            }
        })
    }

    async fn update_book_inner(&self, item: BookWrite) -> Result<(), DatabaseError> {
        let mut tx = self.pool.begin().await?;

        let row = sqlx::query_file!(
            "queries/update_book.sql",
            item.book.id,
            item.book.name.as_ref(),
            item.book.description.as_ref(),
            item.book.publication_year,
            item.book.content_rating,
            item.book.status.as_ref(),
            item.book.kind.as_ref(),
            item.book.publication_language,
            item.book.author,
            item.book.artist,
            item.book.updated_at,
        )
        .fetch_optional(&mut *tx)
        .await?;

        if row.is_none() {
            return Err(DatabaseError::BookNotFound);
        }

        // The arrays are replaced wholesale, never diffed (ADR-0005).
        delete_book_relations(&mut tx, item.book.id).await?;
        store_book_relations(&mut tx, &item).await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn get_book(&self, id: Uuid) -> Result<BookDetails, DatabaseError> {
        self.get_book_inner(id).await.inspect_err(|e| {
            if DatabaseError::is_internal(e) {
                tracing::error!("failed to get book from database: {e:?}");
            }
        })
    }

    async fn get_book_inner(&self, id: Uuid) -> Result<BookDetails, DatabaseError> {
        let row = sqlx::query_file_as!(BookRow, "queries/get_book.sql", id)
            .fetch_optional(&self.pool)
            .await?;

        let Some(row) = row else {
            return Err(DatabaseError::BookNotFound);
        };
        let book = Book::try_from(row)?;

        let label_rows = sqlx::query_file!("queries/get_book_labels.sql", id)
            .fetch_all(&self.pool)
            .await?;
        let mut labels = Vec::with_capacity(label_rows.len());
        for row in label_rows {
            let kind = row
                .kind
                .parse()
                .map_err(|e| DatabaseError::invariant_corrupted("kind", e))?;
            labels.push(Label {
                id: row.id,
                name: row.name,
                kind,
            });
        }

        let link_rows = sqlx::query_file!("queries/get_book_links.sql", id)
            .fetch_all(&self.pool)
            .await?;
        let mut links = Vec::with_capacity(link_rows.len());
        for row in link_rows {
            let kind: BookLinkKind = row
                .kind
                .parse()
                .map_err(|e| DatabaseError::invariant_corrupted("kind", e))?;
            let url = LinkUrl::try_from(row.url)
                .map_err(|e| DatabaseError::invariant_corrupted("url", e))?;
            links.push(BookLink {
                id: row.id,
                kind,
                url,
            });
        }

        let title_rows = sqlx::query_file!("queries/get_book_titles.sql", id)
            .fetch_all(&self.pool)
            .await?;
        let mut titles = Vec::with_capacity(title_rows.len());
        for row in title_rows {
            let name = BookName::try_from(row.name)
                .map_err(|e| DatabaseError::invariant_corrupted("name", e))?;
            titles.push(AlternativeTitle {
                id: row.id,
                language_id: row.language_id,
                name,
            });
        }

        Ok(BookDetails {
            book,
            labels,
            links,
            titles,
        })
    }

    pub async fn get_books(
        &self,
        pagination: &Pagination,
    ) -> Result<(Vec<Book>, Option<Uuid>), DatabaseError> {
        let limit = pagination.limit.as_u32();
        let mut builder: QueryBuilder<Postgres> = QueryBuilder::new(
            "select id, name, description, publication_year, content_rating, status, kind, \
             publication_language, author, artist, updated_at, created_at from books",
        );

        if let Some(id) = pagination.after {
            builder.push(" where id < ").push_bind(id);
        }

        builder
            .push(" order by id desc limit ")
            .push_bind((limit + 1) as i64);

        let rows = builder
            .build_query_as::<BookRow>()
            .fetch_all(&self.pool)
            .await
            .map_err(DatabaseError::from)
            .inspect_err(|e| {
                if DatabaseError::is_internal(e) {
                    tracing::error!("failed to get books from database: {e:?}");
                }
            })?;

        let mut books: Vec<Book> = Vec::with_capacity(rows.len());
        for row in rows {
            let book = row.try_into().inspect_err(|e| {
                if DatabaseError::is_internal(e) {
                    tracing::error!("failed to convert book row: {e:?}");
                }
            })?;
            books.push(book);
        }

        let mut next_cursor = None;
        if books.len() > limit as usize {
            books.pop();
            next_cursor = books.last().map(|b| b.id);
        }

        Ok((books, next_cursor))
    }

    pub async fn delete_book(&self, id: Uuid) -> Result<(), DatabaseError> {
        self.delete_book_inner(id).await.inspect_err(|e| {
            if DatabaseError::is_internal(e) {
                tracing::error!("failed to delete book from database: {e:?}");
            }
        })
    }

    async fn delete_book_inner(&self, id: Uuid) -> Result<(), DatabaseError> {
        let mut tx = self.pool.begin().await?;

        // The attached rows are cleared here rather than by ON DELETE CASCADE,
        // so the ordering stays visible in the application layer.
        delete_book_relations(&mut tx, id).await?;

        let row = sqlx::query_file!("queries/delete_book.sql", id)
            .fetch_optional(&mut *tx)
            .await?;

        if row.is_none() {
            return Err(DatabaseError::BookNotFound);
        }

        tx.commit().await?;
        Ok(())
    }
}

async fn store_book_relations(
    conn: &mut PgConnection,
    item: &BookWrite,
) -> Result<(), DatabaseError> {
    sqlx::query_file!(
        "queries/store_book_labels.sql",
        item.book.id,
        &item.label_ids
    )
    .execute(&mut *conn)
    .await?;

    let link_ids: Vec<Uuid> = item.links.iter().map(|l| l.id).collect();
    let link_kinds: Vec<String> = item
        .links
        .iter()
        .map(|l| l.kind.as_ref().to_owned())
        .collect();
    let link_urls: Vec<String> = item
        .links
        .iter()
        .map(|l| l.url.as_ref().to_owned())
        .collect();
    sqlx::query_file!(
        "queries/store_book_links.sql",
        item.book.id,
        &link_ids,
        &link_kinds,
        &link_urls
    )
    .execute(&mut *conn)
    .await?;

    let titles = item.titles.as_slice();
    let title_ids: Vec<Uuid> = titles.iter().map(|t| t.id).collect();
    let title_language_ids: Vec<Uuid> = titles.iter().map(|t| t.language_id).collect();
    let title_names: Vec<String> = titles.iter().map(|t| t.name.as_ref().to_owned()).collect();
    sqlx::query_file!(
        "queries/store_book_titles.sql",
        item.book.id,
        &title_ids,
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
