use tracing::instrument;
use uuid::Uuid;

use crate::{
    entity::{BookCover, COVER_PRESIGN_TTL},
    error::ObjectStorageError,
    object_storage::ObjectStorage,
};

impl ObjectStorage {
    #[instrument(name = "object_storage.book_cover.upload", skip_all, fields(book.id = %item.book_id, cover.id = %item.id))]
    pub async fn upload_book_cover(&self, item: &BookCover) -> Result<(), ObjectStorageError> {
        self.upload(
            &self.covers_bucket,
            &item.id.to_string(),
            item.content.clone(),
            item.extension.content_type(),
            &item.content_disposition(),
        )
        .await
    }

    #[instrument(name = "object_storage.book_cover.presign", skip_all, fields(cover.id = %id))]
    pub async fn presign_book_cover(&self, id: Uuid) -> Result<String, ObjectStorageError> {
        self.presign(&self.covers_bucket, &id.to_string(), COVER_PRESIGN_TTL)
            .await
    }

    #[instrument(name = "object_storage.book_cover.delete", skip_all, fields(covers = ids.len()))]
    pub async fn delete_book_covers(&self, ids: &[Uuid]) -> Result<(), ObjectStorageError> {
        self.delete_many(&self.covers_bucket, ids).await
    }
}
