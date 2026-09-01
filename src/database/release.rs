use tracing::instrument;
use uuid::Uuid;

use crate::{
    database::{Database, Invariant},
    entity::{
        ChapterPage, ChapterPageParams, ChapterPageQuery, ChapterPages, ChapterRelease,
        ChapterReleaseQuery, ImageExtension, Language, Ordinal, PageOrder, PageStatus, PageUrl,
        UserClaims,
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
        let version = Ordinal::try_from(row.version).or_corrupted("version")?;

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
    sort_order: i32,
    extension: String,
}

impl TryFrom<ChapterPageRow> for ChapterPageQuery {
    type Error = DatabaseError;

    fn try_from(row: ChapterPageRow) -> Result<Self, Self::Error> {
        let page_number = Ordinal::try_from(row.sort_order).or_corrupted("sort_order")?;
        let url = PageUrl {
            release_id: row.release_id,
            page_number,
        }
        .into();
        let extension: ImageExtension = row.extension.parse().or_corrupted("extension")?;

        Ok(ChapterPageQuery::Committed {
            id: row.id,
            page_number,
            extension,
            url,
        })
    }
}

struct StagedChapterPageRow {
    id: Uuid,
    extension: String,
}

impl TryFrom<StagedChapterPageRow> for ChapterPageQuery {
    type Error = DatabaseError;

    fn try_from(row: StagedChapterPageRow) -> Result<Self, Self::Error> {
        let extension: ImageExtension = row.extension.parse().or_corrupted("extension")?;

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
        sqlx::query_file!(
            "queries/store_chapter_release.sql",
            item.id,
            item.chapter_id,
            item.language_id,
        )
        .execute(&self.pool)
        .await
        .map_err(DatabaseError::from)?;

        Ok(())
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

        match row {
            Some(row) => row.try_into(),
            None => Err(DatabaseError::not_found::<ChapterRelease>()),
        }
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

    #[instrument(name = "db.chapter_release.commit", skip_all, fields(release.id = %id, pages = order.len()))]
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

        if ordered.rows_affected() as usize != order.len() {
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

    #[instrument(name = "db.chapter_page.store_many", skip_all, fields(release.id = %item.release_id, pages = item.images.len()))]
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
                    StagedChapterPageRow,
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

    #[instrument(name = "db.chapter_page.id", skip_all, fields(release.id = %release_id, page.number = number.as_i32()))]
    pub async fn get_chapter_page_id(
        &self,
        release_id: Uuid,
        number: Ordinal,
        claims: Option<&UserClaims>,
    ) -> Result<Uuid, DatabaseError> {
        let id = sqlx::query_file_scalar!(
            "queries/get_chapter_page_id.sql",
            release_id,
            number.as_i32(),
            claims.is_some_and(UserClaims::can_moderate)
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(DatabaseError::from)
        .inspect_err(DatabaseError::log_internal)?;

        let Some(id) = id else {
            return Err(DatabaseError::not_found::<ChapterPage>());
        };

        Ok(id)
    }
}
