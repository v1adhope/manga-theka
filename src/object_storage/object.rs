use std::time::Duration;

use aws_sdk_s3::{
    presigning::PresigningConfig,
    primitives::ByteStream,
    types::{Delete, ObjectIdentifier},
};
use bytes::Bytes;
use tracing::instrument;
use uuid::Uuid;

use crate::{
    error::{LogInternal, ObjectStorageError},
    object_storage::ObjectStorage,
};

const DELETE_BATCH_MAX: usize = 1000;

impl ObjectStorage {
    #[instrument(name = "object_storage.object.upload", skip_all, fields(bucket = %bucket, key = %key))]
    pub(super) async fn upload(
        &self,
        bucket: &str,
        key: &str,
        body: Bytes,
        content_type: &str,
        content_disposition: &str,
    ) -> Result<(), ObjectStorageError> {
        self.client
            .put_object()
            .bucket(bucket)
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

    #[instrument(name = "object_storage.object.presign", skip_all, fields(bucket = %bucket, key = %key))]
    pub(super) async fn presign(
        &self,
        bucket: &str,
        key: &str,
        ttl: Duration,
    ) -> Result<String, ObjectStorageError> {
        self.presign_inner(bucket, key, ttl)
            .await
            .map_err(ObjectStorageError::Presign)
            .inspect_err(ObjectStorageError::log_internal)
    }

    async fn presign_inner(
        &self,
        bucket: &str,
        key: &str,
        ttl: Duration,
    ) -> anyhow::Result<String> {
        let presigning = PresigningConfig::expires_in(ttl)?;

        let req = self
            .client
            .get_object()
            .bucket(bucket)
            .key(key)
            .presigned(presigning)
            .await?;

        Ok(req.uri().to_owned())
    }

    #[instrument(name = "object_storage.object.delete_many", skip_all, fields(bucket = %bucket, keys = keys.len()))]
    pub(super) async fn delete_many(
        &self,
        bucket: &str,
        keys: &[Uuid],
    ) -> Result<(), ObjectStorageError> {
        self.delete_many_inner(bucket, keys)
            .await
            .map_err(ObjectStorageError::Delete)
            .inspect_err(ObjectStorageError::log_internal)
    }

    async fn delete_many_inner(&self, bucket: &str, keys: &[Uuid]) -> anyhow::Result<()> {
        if keys.is_empty() {
            return Ok(());
        }

        for chunk in keys.chunks(DELETE_BATCH_MAX) {
            let mut objects = Vec::with_capacity(chunk.len());

            for key in chunk {
                let obj = ObjectIdentifier::builder().key(key.to_string()).build()?;

                objects.push(obj);
            }

            let delete = Delete::builder().set_objects(Some(objects)).build()?;

            let res = self
                .client
                .delete_objects()
                .bucket(bucket)
                .delete(delete)
                .send()
                .await?;

            if !res.errors().is_empty() {
                anyhow::bail!(
                    "failed to delete {} of {} objects: {:?}",
                    res.errors().len(),
                    chunk.len(),
                    res.errors()
                );
            }
        }

        Ok(())
    }
}
