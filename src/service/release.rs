use uuid::Uuid;

use crate::{
    entity::{
        ChapterPage, ChapterPageQuery, ChapterRelease, ChapterReleaseQuery, PageOrder,
        StagedPageQuery,
    },
    error::ServiceError,
    service::Service,
};

impl Service {
    pub async fn store_chapter_release(&self, item: &ChapterRelease) -> Result<(), ServiceError> {
        self.database
            .store_chapter_release(item)
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
        self.database.ensure_chapter_exists(chapter_id).await?;

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

    pub async fn ensure_chapter_release_exists(&self, id: Uuid) -> Result<(), ServiceError> {
        self.database
            .ensure_chapter_release_exists(id)
            .await
            .map_err(Into::into)
    }

    pub async fn store_chapter_page(&self, item: &ChapterPage) -> Result<(), ServiceError> {
        self.storage.upload_chapter_page(item).await?;

        self.database
            .store_chapter_page(item)
            .await
            .map_err(Into::into)
    }

    pub async fn commit_chapter_release(
        &self,
        id: Uuid,
        order: &PageOrder,
    ) -> Result<(), ServiceError> {
        let removed = self.database.commit_chapter_release(id, order).await?;

        let _ = self.storage.delete_chapter_pages(&removed).await;

        Ok(())
    }

    pub async fn get_chapter_pages(
        &self,
        release_id: Uuid,
    ) -> Result<Vec<ChapterPageQuery>, ServiceError> {
        self.database
            .ensure_chapter_release_exists(release_id)
            .await?;

        self.database
            .get_chapter_pages(release_id)
            .await
            .map_err(Into::into)
    }

    pub async fn get_staged_chapter_pages(
        &self,
        release_id: Uuid,
    ) -> Result<Vec<StagedPageQuery>, ServiceError> {
        self.database
            .ensure_chapter_release_exists(release_id)
            .await?;

        self.database
            .get_staged_chapter_pages(release_id)
            .await
            .map_err(Into::into)
    }

    pub async fn presign_chapter_page(
        &self,
        release_id: Uuid,
        id: Uuid,
    ) -> Result<String, ServiceError> {
        self.database
            .ensure_chapter_page_exists(release_id, id)
            .await?;

        self.storage
            .presign_chapter_page(id)
            .await
            .map_err(Into::into)
    }

    pub async fn delete_chapter_release(&self, id: Uuid) -> Result<(), ServiceError> {
        let page_ids = self.database.get_chapter_release_page_ids(id).await?;

        self.database.delete_chapter_release(id).await?;

        self.storage
            .delete_chapter_pages(&page_ids)
            .await
            .map_err(Into::into)
    }
}
