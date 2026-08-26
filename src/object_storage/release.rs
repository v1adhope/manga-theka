use tracing::instrument;
use uuid::Uuid;

use crate::{
    entity::ChapterPage,
    error::ObjectStorageError,
    object_storage::{DEFAULT_PRESIGN_TTL, ObjectStorage},
};

impl ObjectStorage {
    #[instrument(name = "object_storage.chapter_page.upload", skip_all, fields(release.id = %item.release_id, page.id = %item.image.id))]
    pub async fn upload_chapter_page(&self, item: &ChapterPage) -> Result<(), ObjectStorageError> {
        self.upload(
            &self.release_pages_bucket,
            &item.image.id.to_string(),
            item.image.content.to_bytes(),
            item.image.extension.content_type(),
            &item.image.content_disposition(),
        )
        .await
    }

    #[instrument(name = "object_storage.chapter_page.presign", skip_all, fields(page.id = %id))]
    pub async fn presign_chapter_page(&self, id: Uuid) -> Result<String, ObjectStorageError> {
        self.presign(
            &self.release_pages_bucket,
            &id.to_string(),
            DEFAULT_PRESIGN_TTL,
        )
        .await
    }

    #[instrument(name = "object_storage.chapter_page.delete", skip_all, fields(pages = ids.len()))]
    pub async fn delete_chapter_pages(&self, ids: &[Uuid]) -> Result<(), ObjectStorageError> {
        self.delete_many(&self.release_pages_bucket, ids).await
    }
}
