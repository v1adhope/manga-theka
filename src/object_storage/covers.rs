use std::time::Duration;

use aws_sdk_s3::{presigning::PresigningConfig, primitives::ByteStream};
use bytes::Bytes;
use tracing::instrument;

use crate::{error::ObjectStorageError, object_storage::ObjectStorage};

impl ObjectStorage {
    #[instrument(name = "object_storage.covers.upload_cover", skip_all, fields(bucket = %self.bucket, key = %key))]
    pub async fn upload_cover(
        &self,
        key: &str,
        body: Bytes,
        content_type: &str,
    ) -> Result<(), ObjectStorageError> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type(content_type)
            .body(ByteStream::from(body))
            .send()
            .await
            .map_err(|e| ObjectStorageError::Upload(Box::new(e)))
            .inspect_err(ObjectStorageError::log_internal)?;

        Ok(())
    }

    #[instrument(name = "object_storage.covers.presign_cover", skip_all, fields(bucket = %self.bucket, key = %key))]
    pub async fn presign_cover(
        &self,
        key: &str,
        ttl: Duration,
        content_disposition: &str,
    ) -> Result<String, ObjectStorageError> {
        let presigning = PresigningConfig::expires_in(ttl)
            .map_err(ObjectStorageError::Presigning)
            .inspect_err(ObjectStorageError::log_internal)?;

        let req = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .response_content_disposition(content_disposition)
            .presigned(presigning)
            .await
            .map_err(|e| ObjectStorageError::Presign(Box::new(e)))
            .inspect_err(ObjectStorageError::log_internal)?;

        Ok(req.uri().to_owned())
    }

    #[instrument(name = "object_storage.covers.delete_cover", skip_all, fields(bucket = %self.bucket, key = %key))]
    pub async fn delete_cover(&self, key: &str) -> Result<(), ObjectStorageError> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| ObjectStorageError::Delete(Box::new(e)))
            .inspect_err(ObjectStorageError::log_internal)?;

        Ok(())
    }
}
