use std::time::Duration;

use aws_sdk_s3::{
    presigning::PresigningConfig,
    primitives::ByteStream,
    types::{Delete, ObjectIdentifier},
};
use bytes::Bytes;
use tracing::instrument;
use uuid::Uuid;

use crate::{error::ObjectStorageError, object_storage::ObjectStorage};

const DELETE_BATCH_MAX: usize = 1000;

impl ObjectStorage {
    #[instrument(name = "object_storage.object.upload", skip_all, fields(bucket = %self.bucket, key = %key))]
    pub async fn upload(
        &self,
        key: &str,
        body: Bytes,
        content_type: &str,
        content_disposition: &str,
    ) -> Result<(), ObjectStorageError> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type(content_type)
            .content_disposition(content_disposition)
            .body(ByteStream::from(body))
            .send()
            .await
            .map_err(|e| ObjectStorageError::Upload(e.into()))
            .inspect_err(ObjectStorageError::log_internal)?;

        Ok(())
    }

    #[instrument(name = "object_storage.object.presign", skip_all, fields(bucket = %self.bucket, key = %key))]
    pub async fn presign(&self, key: &str, ttl: Duration) -> Result<String, ObjectStorageError> {
        let presigning = PresigningConfig::expires_in(ttl)
            .map_err(|e| ObjectStorageError::Presigning(e.into()))
            .inspect_err(ObjectStorageError::log_internal)?;

        let req = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .presigned(presigning)
            .await
            .map_err(|e| ObjectStorageError::Presign(e.into()))
            .inspect_err(ObjectStorageError::log_internal)?;

        Ok(req.uri().to_owned())
    }

    #[instrument(name = "object_storage.object.delete", skip_all, fields(bucket = %self.bucket, key = %key))]
    pub async fn delete(&self, key: Uuid) -> Result<(), ObjectStorageError> {
        self.delete_many(&[key]).await
    }

    #[instrument(name = "object_storage.object.delete_many", skip_all, fields(bucket = %self.bucket, keys = keys.len()))]
    pub async fn delete_many(&self, keys: &[Uuid]) -> Result<(), ObjectStorageError> {
        if keys.is_empty() {
            return Ok(());
        }

        for chunk in keys.chunks(DELETE_BATCH_MAX) {
            let mut objects = Vec::with_capacity(chunk.len());

            for key in chunk {
                let obj = ObjectIdentifier::builder()
                    .key(key.to_string())
                    .build()
                    .map_err(|e| ObjectStorageError::Delete(e.into()))
                    .inspect_err(ObjectStorageError::log_internal)?;

                objects.push(obj);
            }

            let delete = Delete::builder()
                .set_objects(Some(objects))
                .build()
                .map_err(|e| ObjectStorageError::Delete(e.into()))
                .inspect_err(ObjectStorageError::log_internal)?;

            let res = self
                .client
                .delete_objects()
                .bucket(&self.bucket)
                .delete(delete)
                .send()
                .await
                .map_err(|e| ObjectStorageError::Delete(e.into()))
                .inspect_err(ObjectStorageError::log_internal)?;

            if !res.errors().is_empty() {
                tracing::error!(errors = ?res.errors(), "internal object storage error");
            }
        }

        Ok(())
    }
}
