use tokio::task::JoinSet;
use uuid::Uuid;

use crate::{
    entity::{
        ChapterPageParams, ChapterPageQuery, ChapterPages, ChapterRelease, ChapterReleaseQuery,
        Ordinal, PageOrder, UPLOAD_CHUNK_SIZE, UserClaims,
    },
    error::{ObjectStorageError, ServiceError},
    service::Service,
};

impl Service {
    pub async fn store_chapter_release(&self, item: ChapterRelease) -> Result<(), ServiceError> {
        self.ensure_content_writable_by_chapter(item.chapter_id)
            .await?;

        self.database
            .store_chapter_release(&item)
            .await
            .map_err(Into::into)
    }

    pub async fn get_chapter_release(&self, id: Uuid) -> Result<ChapterReleaseQuery, ServiceError> {
        self.database
            .get_chapter_release(id)
            .await
            .map_err(Into::into)
    }

    pub async fn get_chapter_releases(
        &self,
        chapter_id: Uuid,
    ) -> Result<Vec<ChapterReleaseQuery>, ServiceError> {
        self.ensure_book_exists_by_chapter(chapter_id).await?;

        self.database
            .get_chapter_releases(chapter_id)
            .await
            .map_err(Into::into)
    }

    pub async fn count_chapter_pages(&self, release_id: Uuid) -> Result<i64, ServiceError> {
        self.database
            .count_chapter_pages(release_id)
            .await
            .map_err(Into::into)
    }

    pub async fn ensure_chapter_release_writable(
        &self,
        id: Uuid,
        claims: &UserClaims,
    ) -> Result<(), ServiceError> {
        self.ensure_release_mutable(id, claims).await
    }

    pub async fn store_chapter_pages(
        &self,
        item: ChapterPages,
        claims: &UserClaims,
    ) -> Result<(), ServiceError> {
        self.ensure_release_mutable(item.release_id, claims).await?;

        let existing = self.database.count_chapter_pages(item.release_id).await? as usize;
        ChapterRelease::ensure_row_capacity(existing, item.images.len())?;

        for chunk in item.images.as_slice().chunks(UPLOAD_CHUNK_SIZE) {
            let mut uploads = JoinSet::new();

            for image in chunk {
                let storage = self.storage.clone();
                let image = image.clone();
                let release_id = item.release_id;

                uploads.spawn(async move { storage.upload_chapter_page(release_id, &image).await });
            }

            while let Some(res) = uploads.join_next().await {
                res.map_err(|e| ObjectStorageError::Upload(e.into()))??;
            }
        }

        self.database
            .store_chapter_pages(&item)
            .await
            .map_err(Into::into)
    }

    pub async fn commit_chapter_release(
        &self,
        id: Uuid,
        order: &PageOrder,
        claims: &UserClaims,
    ) -> Result<(), ServiceError> {
        self.ensure_release_mutable(id, claims).await?;

        let removed = self.database.commit_chapter_release(id, order).await?;

        let _ = self.storage.delete_chapter_pages(&removed).await;

        Ok(())
    }

    pub async fn get_chapter_pages(
        &self,
        release_id: Uuid,
        params: ChapterPageParams,
        claims: Option<&UserClaims>,
    ) -> Result<Vec<ChapterPageQuery>, ServiceError> {
        self.ensure_release_readable(release_id, claims).await?;

        self.database
            .get_chapter_pages(release_id, params)
            .await
            .map_err(Into::into)
    }

    pub async fn presign_chapter_page(
        &self,
        release_id: Uuid,
        id: Uuid,
        claims: Option<&UserClaims>,
    ) -> Result<String, ServiceError> {
        self.ensure_page_readable(release_id, id, claims).await?;

        self.storage
            .presign_chapter_page(id)
            .await
            .map_err(Into::into)
    }

    pub async fn presign_chapter_page_by_number(
        &self,
        release_id: Uuid,
        number: Ordinal,
        claims: Option<&UserClaims>,
    ) -> Result<String, ServiceError> {
        let id = self
            .database
            .get_chapter_page_id(release_id, number, claims)
            .await?;

        self.storage
            .presign_chapter_page(id)
            .await
            .map_err(Into::into)
    }

    pub async fn delete_chapter_release(
        &self,
        id: Uuid,
        claims: &UserClaims,
    ) -> Result<(), ServiceError> {
        self.ensure_release_mutable(id, claims).await?;

        let page_ids = self.database.get_chapter_release_page_ids(id).await?;

        self.database.delete_chapter_release(id).await?;

        let _ = self.storage.delete_chapter_pages(&page_ids).await;

        Ok(())
    }
}
