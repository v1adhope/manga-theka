use tracing::instrument;

use crate::{entity::ChapterPage, error::ObjectStorageError, object_storage::ObjectStorage};

impl ObjectStorage {
    #[instrument(name = "object_storage.chapter_page.upload", skip_all, fields(release.id = %item.release_id, page.id = %item.id))]
    pub async fn upload_chapter_page(&self, item: &ChapterPage) -> Result<(), ObjectStorageError> {
        self.upload(
            &item.id.to_string(),
            item.content.clone(),
            item.extension.content_type(),
            &item.content_disposition(),
        )
        .await
    }
}
