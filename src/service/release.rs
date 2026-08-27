use uuid::Uuid;

use crate::{
    entity::{
        ChapterPageParams, ChapterPageQuery, ChapterPages, ChapterRelease, ChapterReleaseQuery,
        ImageExtension, PageOrder,
    },
    error::ServiceError,
    service::{Service, concurrency::run_concurrently},
};

struct UploadedPage {
    id: Uuid,
    extension: ImageExtension,
}

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

    pub async fn store_chapter_pages(&self, item: ChapterPages) -> Result<Vec<Uuid>, ServiceError> {
        const UPLOAD_CONCURRENCY: usize = 5;

        let ChapterPages { release_id, images } = item;

        let existing = self.database.count_chapter_pages(release_id).await? as usize;
        ChapterRelease::ensure_row_capacity(existing, images.as_slice().len())?;

        let storage = self.storage.clone();
        let uploaded = run_concurrently(images.into_inner(), UPLOAD_CONCURRENCY, move |image| {
            let storage = storage.clone();
            async move {
                storage.upload_chapter_page(release_id, &image).await?;
                Ok::<UploadedPage, ServiceError>(UploadedPage {
                    id: image.id,
                    extension: image.extension,
                })
            }
        })
        .await?;

        let database = self.database.clone();
        run_concurrently(uploaded, UPLOAD_CONCURRENCY, move |page| {
            let database = database.clone();
            async move {
                database
                    .store_chapter_page(release_id, page.id, page.extension)
                    .await?;
                Ok::<Uuid, ServiceError>(page.id)
            }
        })
        .await
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
        params: ChapterPageParams,
    ) -> Result<Vec<ChapterPageQuery>, ServiceError> {
        self.database
            .ensure_chapter_release_exists(release_id)
            .await?;

        self.database
            .get_chapter_pages(release_id, params)
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
