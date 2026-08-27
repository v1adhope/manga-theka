use tracing::instrument;
use uuid::Uuid;

use crate::{
    database::Database,
    entity::{
        Chapter, ChapterPage, ChapterPageParams, ChapterPageQuery, ChapterPages, ChapterRelease,
        ChapterReleaseQuery, ImageExtension, Language, PageOrder, PageStatus,
    },
    error::DatabaseError,
};

struct ChapterReleaseRow {
    id: Uuid,
    chapter_id: Uuid,
    version: i32,
    language_id: Uuid,
    language_code: String,
    language_name: String,
    page_count: i64,
}

impl TryFrom<ChapterReleaseRow> for ChapterReleaseQuery {
    type Error = DatabaseError;

    fn try_from(row: ChapterReleaseRow) -> Result<Self, Self::Error> {
        let version = u16::try_from(row.version)
            .map_err(|e| DatabaseError::invariant_corrupted("version", e))?;

        Ok(ChapterReleaseQuery {
            id: row.id,
            chapter_id: row.chapter_id,
            language: Language {
                id: row.language_id,
                code: row.language_code,
                name: row.language_name,
            },
            page_count: row.page_count,
            version,
        })
    }
}

struct ChapterPageRow {
    id: Uuid,
    release_id: Uuid,
    sort_order: i16,
    extension: String,
}

impl TryFrom<ChapterPageRow> for ChapterPageQuery {
    type Error = DatabaseError;

    fn try_from(row: ChapterPageRow) -> Result<Self, Self::Error> {
        let url = (row.release_id, row.sort_order).into();
        let extension: ImageExtension = row
            .extension
            .parse()
            .map_err(|e| DatabaseError::invariant_corrupted("extension", e))?;

        Ok(ChapterPageQuery::Committed {
            id: row.id,
            page_number: row.sort_order,
            extension,
            url,
        })
    }
}

struct StagedPageRow {
    id: Uuid,
    extension: String,
}

impl TryFrom<StagedPageRow> for ChapterPageQuery {
    type Error = DatabaseError;

    fn try_from(row: StagedPageRow) -> Result<Self, Self::Error> {
        let extension: ImageExtension = row
            .extension
            .parse()
            .map_err(|e| DatabaseError::invariant_corrupted("extension", e))?;

        Ok(ChapterPageQuery::Staged {
            id: row.id,
            extension,
        })
    }
}

impl Database {
    #[instrument(name = "db.chapter_release.store", skip_all, fields(chapter.id = %item.chapter_id, release.id = %item.id))]
    pub async fn store_chapter_release(&self, item: &ChapterRelease) -> Result<(), DatabaseError> {
        self.store_chapter_release_inner(item)
            .await
            .inspect_err(DatabaseError::log_internal)
    }

    async fn store_chapter_release_inner(
        &self,
        item: &ChapterRelease,
    ) -> Result<(), DatabaseError> {
        let row = sqlx::query_file!(
            "queries/store_chapter_release.sql",
            item.id,
            item.chapter_id,
            item.language_id,
        )
        .fetch_optional(&self.pool)
        .await?;

        if row.is_some() {
            return Ok(());
        }

        let probe = sqlx::query_file!("queries/chapter_exists.sql", item.chapter_id)
            .fetch_one(&self.pool)
            .await?;

        if probe.exists {
            return Err(DatabaseError::ChapterReleaseLanguageIsPublicationLanguage);
        }

        Err(DatabaseError::not_found::<Chapter>())
    }

    #[instrument(name = "db.chapter_release.get", skip_all, fields(release.id = %id))]
    pub async fn get_chapter_release(
        &self,
        id: Uuid,
    ) -> Result<ChapterReleaseQuery, DatabaseError> {
        self.get_chapter_release_inner(id)
            .await
            .inspect_err(DatabaseError::log_internal)
    }

    async fn get_chapter_release_inner(
        &self,
        id: Uuid,
    ) -> Result<ChapterReleaseQuery, DatabaseError> {
        let row = sqlx::query_file_as!(ChapterReleaseRow, "queries/get_chapter_release.sql", id)
            .fetch_optional(&self.pool)
            .await?;

        let Some(row) = row else {
            return Err(DatabaseError::not_found::<ChapterRelease>());
        };

        row.try_into()
    }

    #[instrument(name = "db.chapter_release.list", skip_all, fields(chapter.id = %chapter_id))]
    pub async fn get_chapter_releases(
        &self,
        chapter_id: Uuid,
    ) -> Result<Vec<ChapterReleaseQuery>, DatabaseError> {
        self.get_chapter_releases_inner(chapter_id)
            .await
            .inspect_err(DatabaseError::log_internal)
    }

    async fn get_chapter_releases_inner(
        &self,
        chapter_id: Uuid,
    ) -> Result<Vec<ChapterReleaseQuery>, DatabaseError> {
        let rows = sqlx::query_file_as!(
            ChapterReleaseRow,
            "queries/get_chapter_releases.sql",
            chapter_id
        )
        .fetch_all(&self.pool)
        .await?;

        let mut releases: Vec<ChapterReleaseQuery> = Vec::with_capacity(rows.len());
        for row in rows {
            let release: ChapterReleaseQuery = row.try_into()?;
            releases.push(release);
        }

        Ok(releases)
    }

    #[instrument(name = "db.chapter_release.exists", skip_all, fields(release.id = %id))]
    pub async fn ensure_chapter_release_exists(&self, id: Uuid) -> Result<(), DatabaseError> {
        let row = sqlx::query_file!("queries/chapter_release_exists.sql", id)
            .fetch_one(&self.pool)
            .await
            .map_err(DatabaseError::from)
            .inspect_err(DatabaseError::log_internal)?;

        if !row.exists {
            return Err(DatabaseError::not_found::<ChapterRelease>());
        }

        Ok(())
    }

    #[instrument(name = "db.chapter_release.commit", skip_all, fields(release.id = %id, pages = order.as_slice().len()))]
    pub async fn commit_chapter_release(
        &self,
        id: Uuid,
        order: &PageOrder,
    ) -> Result<Vec<Uuid>, DatabaseError> {
        self.commit_chapter_release_inner(id, order)
            .await
            .inspect_err(DatabaseError::log_internal)
    }

    async fn commit_chapter_release_inner(
        &self,
        id: Uuid,
        order: &PageOrder,
    ) -> Result<Vec<Uuid>, DatabaseError> {
        let mut tx = self.pool.begin().await?;

        let bumped = sqlx::query_file!("queries/bump_chapter_release_version.sql", id)
            .fetch_optional(&mut *tx)
            .await?;

        if bumped.is_none() {
            return Err(DatabaseError::not_found::<ChapterRelease>());
        }

        let ordered = sqlx::query_file!("queries/order_chapter_pages.sql", id, order.as_slice())
            .execute(&mut *tx)
            .await?;

        if ordered.rows_affected() as usize != order.as_slice().len() {
            return Err(DatabaseError::PageOrderIsForeign);
        }

        let removed = sqlx::query_file_scalar!(
            "queries/delete_undeclared_chapter_pages.sql",
            id,
            order.as_slice()
        )
        .fetch_all(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(removed)
    }

    #[instrument(name = "db.chapter_release.page_ids", skip_all, fields(release.id = %id))]
    pub async fn get_chapter_release_page_ids(&self, id: Uuid) -> Result<Vec<Uuid>, DatabaseError> {
        sqlx::query_file_scalar!("queries/get_chapter_release_page_ids.sql", id)
            .fetch_all(&self.pool)
            .await
            .map_err(DatabaseError::from)
            .inspect_err(DatabaseError::log_internal)
    }

    #[instrument(name = "db.chapter_release.delete", skip_all, fields(release.id = %id))]
    pub async fn delete_chapter_release(&self, id: Uuid) -> Result<(), DatabaseError> {
        let row = sqlx::query_file!("queries/delete_chapter_release.sql", id)
            .fetch_optional(&self.pool)
            .await
            .map_err(DatabaseError::from)
            .inspect_err(DatabaseError::log_internal)?;

        if row.is_none() {
            return Err(DatabaseError::not_found::<ChapterRelease>());
        }

        Ok(())
    }

    #[instrument(name = "db.chapter_page.store_many", skip_all, fields(release.id = %item.release_id, pages = item.images.as_slice().len()))]
    pub async fn store_chapter_pages(&self, item: &ChapterPages) -> Result<(), DatabaseError> {
        let images = item.images.as_slice();
        let mut ids = Vec::with_capacity(images.len());
        let mut extensions = Vec::with_capacity(images.len());

        for image in images {
            ids.push(image.id);
            extensions.push(image.extension.as_ref().to_owned());
        }

        sqlx::query_file!(
            "queries/store_chapter_pages.sql",
            item.release_id,
            &ids,
            &extensions,
        )
        .execute(&self.pool)
        .await
        .map_err(DatabaseError::from)
        .inspect_err(DatabaseError::log_internal)?;

        Ok(())
    }

    #[instrument(name = "db.chapter_page.count", skip_all, fields(release.id = %release_id))]
    pub async fn count_chapter_pages(&self, release_id: Uuid) -> Result<i64, DatabaseError> {
        sqlx::query_file_scalar!("queries/count_chapter_pages.sql", release_id)
            .fetch_one(&self.pool)
            .await
            .map_err(DatabaseError::from)
            .inspect_err(DatabaseError::log_internal)
    }

    #[instrument(name = "db.chapter_page.list", skip_all, fields(release.id = %release_id))]
    pub async fn get_chapter_pages(
        &self,
        release_id: Uuid,
        params: ChapterPageParams,
    ) -> Result<Vec<ChapterPageQuery>, DatabaseError> {
        self.get_chapter_pages_inner(release_id, params)
            .await
            .inspect_err(DatabaseError::log_internal)
    }

    async fn get_chapter_pages_inner(
        &self,
        release_id: Uuid,
        params: ChapterPageParams,
    ) -> Result<Vec<ChapterPageQuery>, DatabaseError> {
        match params.status {
            Some(PageStatus::Staged) => {
                let rows = sqlx::query_file_as!(
                    StagedPageRow,
                    "queries/get_staged_chapter_pages.sql",
                    release_id
                )
                .fetch_all(&self.pool)
                .await?;

                rows.into_iter().map(TryInto::try_into).collect()
            }
            None => {
                let rows = sqlx::query_file_as!(
                    ChapterPageRow,
                    "queries/get_chapter_pages.sql",
                    release_id
                )
                .fetch_all(&self.pool)
                .await?;

                rows.into_iter().map(TryInto::try_into).collect()
            }
        }
    }

    #[instrument(name = "db.chapter_page.exists", skip_all, fields(release.id = %release_id, page.id = %id))]
    pub async fn ensure_chapter_page_exists(
        &self,
        release_id: Uuid,
        id: Uuid,
    ) -> Result<(), DatabaseError> {
        let row = sqlx::query_file!("queries/chapter_page_exists.sql", id, release_id)
            .fetch_one(&self.pool)
            .await
            .map_err(DatabaseError::from)
            .inspect_err(DatabaseError::log_internal)?;

        if !row.exists {
            return Err(DatabaseError::not_found::<ChapterPage>());
        }

        Ok(())
    }
}
