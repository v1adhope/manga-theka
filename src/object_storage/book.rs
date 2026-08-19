use tracing::instrument;

use crate::{entity::BookCover, error::ObjectStorageError, object_storage::ObjectStorage};

impl ObjectStorage {
    #[instrument(name = "object_storage.book_cover.upload", skip_all, fields(book.id = %item.book_id, cover.id = %item.id))]
    pub async fn upload_book_cover(&self, item: &BookCover) -> Result<(), ObjectStorageError> {
        self.upload(
            &item.id.to_string(),
            item.content.clone(),
            item.extension.content_type(),
            &item.content_disposition(),
        )
        .await
    }
}
