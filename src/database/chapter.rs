use std::collections::HashMap;

use sqlx::{PgConnection, Postgres, QueryBuilder};
use tracing::{Level, instrument};
use uuid::Uuid;

use crate::{
    database::Database,
    entity::{
        Chapter, ChapterLocalization, ChapterLocalizations, ChapterName, ChapterNumber,
        ChapterVolume, DEFAULT_LIMIT, Filter, Limit, SortOrder,
    },
    error::DatabaseError,
};

#[derive(sqlx::FromRow)]
struct ChapterRow {
    id: Uuid,
    book_id: Uuid,
    number: f32,
    name: Option<String>,
    volume: Option<i16>,
    updated_at: Option<time::OffsetDateTime>,
    created_at: time::OffsetDateTime,
}

struct ChapterLocalizationRow {
    chapter_id: Uuid,
    language_id: Uuid,
    name: String,
}

impl TryFrom<ChapterLocalizationRow> for ChapterLocalization {
    type Error = DatabaseError;

    fn try_from(row: ChapterLocalizationRow) -> Result<Self, Self::Error> {
        let name = ChapterName::try_from(row.name)
            .map_err(|e| DatabaseError::invariant_corrupted("name", e))?;

        Ok(ChapterLocalization {
            language_id: row.language_id,
            name,
        })
    }
}

struct ChapterWithRelations {
    row: ChapterRow,
    localizations: Vec<ChapterLocalization>,
}

impl TryFrom<ChapterWithRelations> for Chapter {
    type Error = DatabaseError;

    fn try_from(item: ChapterWithRelations) -> Result<Self, Self::Error> {
        let ChapterWithRelations { row, localizations } = item;

        let number = ChapterNumber::try_from(row.number)
            .map_err(|e| DatabaseError::invariant_corrupted("number", e))?;
        let name = row
            .name
            .map(ChapterName::try_from)
            .transpose()
            .map_err(|e| DatabaseError::invariant_corrupted("name", e))?;

        let volume = row
            .volume
            .map(ChapterVolume::try_from)
            .transpose()
            .map_err(|e| DatabaseError::invariant_corrupted("volume", e))?;

        let localizations = ChapterLocalizations::try_from(localizations)
            .map_err(|e| DatabaseError::invariant_corrupted("localizations", e))?;

        Ok(Chapter {
            id: row.id,
            book_id: row.book_id,
            number,
            name,
            volume,
            localizations,
            updated_at: row.updated_at,
            created_at: row.created_at,
        })
    }
}

impl Database {
    #[instrument(name = "db.chapter.store", skip_all, fields(book.id = %item.book_id, chapter.id = %item.id))]
    pub async fn store_chapter(&self, item: &Chapter) -> Result<(), DatabaseError> {
        self.store_chapter_inner(item)
            .await
            .inspect_err(DatabaseError::log_internal)
    }

    async fn store_chapter_inner(&self, item: &Chapter) -> Result<(), DatabaseError> {
        let mut tx = self.pool.begin().await?;

        sqlx::query_file!(
            "queries/store_chapter.sql",
            item.id,
            item.book_id,
            item.number.as_f32(),
            item.name.as_ref().map(AsRef::as_ref),
            item.volume.map(ChapterVolume::as_i16),
            item.updated_at,
            item.created_at,
        )
        .execute(&mut *tx)
        .await?;

        Self::store_chapter_localizations(&mut tx, item).await?;

        tx.commit().await?;
        Ok(())
    }

    #[instrument(name = "db.chapter.update", skip_all, fields(chapter.id = %item.id))]
    pub async fn update_chapter(&self, item: &Chapter) -> Result<(), DatabaseError> {
        self.update_chapter_inner(item)
            .await
            .inspect_err(DatabaseError::log_internal)
    }

    async fn update_chapter_inner(&self, item: &Chapter) -> Result<(), DatabaseError> {
        let mut tx = self.pool.begin().await?;

        let row = sqlx::query_file!(
            "queries/update_chapter.sql",
            item.id,
            item.number.as_f32(),
            item.name.as_ref().map(AsRef::as_ref),
            item.volume.map(ChapterVolume::as_i16),
            item.updated_at,
        )
        .fetch_optional(&mut *tx)
        .await?;

        if row.is_none() {
            return Err(DatabaseError::not_found::<Chapter>());
        }

        sqlx::query_file!("queries/delete_chapter_localizations.sql", item.id)
            .execute(&mut *tx)
            .await?;
        Self::store_chapter_localizations(&mut tx, item).await?;

        tx.commit().await?;
        Ok(())
    }

    #[instrument(name = "db.chapter.get", skip_all, fields(chapter.id = %id))]
    pub async fn get_chapter(&self, id: Uuid) -> Result<Chapter, DatabaseError> {
        self.get_chapter_inner(id)
            .await
            .inspect_err(DatabaseError::log_internal)
    }

    async fn get_chapter_inner(&self, id: Uuid) -> Result<Chapter, DatabaseError> {
        let row = sqlx::query_file_as!(ChapterRow, "queries/get_chapter.sql", id)
            .fetch_optional(&self.pool)
            .await?;

        let Some(row) = row else {
            return Err(DatabaseError::not_found::<Chapter>());
        };

        let mut localizations = self.get_chapters_localizations(&[id]).await?;

        ChapterWithRelations {
            row,
            localizations: localizations.remove(&id).unwrap_or_default(),
        }
        .try_into()
    }

    #[instrument(
        name = "db.chapter.list",
        skip_all,
        fields(book.id = %book_id, filter = ?filter)
    )]
    pub async fn get_chapters(
        &self,
        book_id: Uuid,
        filter: &Filter,
    ) -> Result<(Vec<Chapter>, Option<Uuid>), DatabaseError> {
        self.get_chapters_inner(book_id, filter)
            .await
            .inspect_err(DatabaseError::log_internal)
    }

    async fn get_chapters_inner(
        &self,
        book_id: Uuid,
        filter: &Filter,
    ) -> Result<(Vec<Chapter>, Option<Uuid>), DatabaseError> {
        let limit = filter.limit.map_or(DEFAULT_LIMIT, Limit::as_u32);
        let (cursor_comparison, direction) = match filter.sort_order.unwrap_or(SortOrder::Desc) {
            SortOrder::Asc => (">", "asc"),
            SortOrder::Desc => ("<", "desc"),
        };

        let mut builder: QueryBuilder<Postgres> = QueryBuilder::new(
            r"select c.id, c.book_id, c.number, c.name, c.volume, c.updated_at, c.created_at
              from chapters c
              where c.book_id = ",
        );
        builder.push_bind(book_id);

        if let Some(cursor) = filter.after {
            builder
                .push(" and c.number ")
                .push(cursor_comparison)
                .push(" (select cur.number from chapters cur where cur.id = ")
                .push_bind(cursor)
                .push(" and cur.book_id = ")
                .push_bind(book_id)
                .push(")");
        }

        builder
            .push(" order by c.number ")
            .push(direction)
            .push(" limit ")
            .push_bind(i64::from(limit) + 1);

        let mut rows = builder
            .build_query_as::<ChapterRow>()
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
        let mut localizations = self.get_chapters_localizations(&ids).await?;

        let mut chapters: Vec<Chapter> = Vec::with_capacity(rows.len());
        for row in rows {
            let id = row.id;
            let chapter: Chapter = ChapterWithRelations {
                row,
                localizations: localizations.remove(&id).unwrap_or_default(),
            }
            .try_into()?;
            chapters.push(chapter);
        }

        Ok((chapters, next_cursor))
    }

    #[instrument(name = "db.chapter.exists", skip_all, fields(chapter.id = %id))]
    pub async fn ensure_chapter_exists(&self, id: Uuid) -> Result<(), DatabaseError> {
        let row = sqlx::query_file!("queries/chapter_exists.sql", id)
            .fetch_one(&self.pool)
            .await
            .map_err(DatabaseError::from)
            .inspect_err(DatabaseError::log_internal)?;

        if !row.exists {
            return Err(DatabaseError::not_found::<Chapter>());
        }

        Ok(())
    }

    #[instrument(name = "db.chapter.delete", skip_all, fields(chapter.id = %id))]
    pub async fn delete_chapter(&self, id: Uuid) -> Result<(), DatabaseError> {
        let row = sqlx::query_file!("queries/delete_chapter.sql", id)
            .fetch_optional(&self.pool)
            .await
            .map_err(DatabaseError::from)
            .inspect_err(DatabaseError::log_internal)?;

        if row.is_none() {
            return Err(DatabaseError::not_found::<Chapter>());
        }

        Ok(())
    }

    #[instrument(name = "db.chapter.localizations", skip_all, level = Level::DEBUG, fields(chapters = chapter_ids.len()))]
    async fn get_chapters_localizations(
        &self,
        chapter_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, Vec<ChapterLocalization>>, DatabaseError> {
        let rows = sqlx::query_file_as!(
            ChapterLocalizationRow,
            "queries/get_chapters_localizations.sql",
            chapter_ids
        )
        .fetch_all(&self.pool)
        .await?;

        let mut localizations: HashMap<Uuid, Vec<ChapterLocalization>> =
            HashMap::with_capacity(chapter_ids.len());
        for row in rows {
            let chapter_id = row.chapter_id;
            let localization: ChapterLocalization = row.try_into()?;
            localizations
                .entry(chapter_id)
                .or_default()
                .push(localization);
        }

        Ok(localizations)
    }

    async fn store_chapter_localizations(
        conn: &mut PgConnection,
        item: &Chapter,
    ) -> Result<(), DatabaseError> {
        if item.localizations.is_empty() {
            return Ok(());
        }

        let mut language_ids: Vec<Uuid> = Vec::with_capacity(item.localizations.len());
        let mut names: Vec<String> = Vec::with_capacity(item.localizations.len());
        for localization in item.localizations.as_slice() {
            language_ids.push(localization.language_id);
            names.push(localization.name.as_ref().to_owned());
        }

        sqlx::query_file!(
            "queries/store_chapter_localizations.sql",
            item.id,
            &language_ids,
            &names
        )
        .execute(conn)
        .await?;

        Ok(())
    }
}
