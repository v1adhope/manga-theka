use std::time::Duration;

use aws_sdk_s3::{
    Client,
    config::{BehaviorVersion, Credentials, Region, retry::RetryConfig},
    operation::create_bucket::CreateBucketError,
    presigning::PresigningConfig,
    primitives::ByteStream,
};
use bytes::Bytes;
use secrecy::ExposeSecret;
use tracing::instrument;

use crate::{config, error::ObjectStorageError};

const REGION: &str = "us-east-1";
const CREDENTIALS_PROVIDER: &str = "manga-theka";
const MAX_ATTEMPTS: u32 = 5;

pub fn client(cfg: &config::ObjectStorage) -> Client {
    let credentials = Credentials::new(
        &cfg.access_key,
        cfg.secret_key.expose_secret(),
        None,
        None,
        CREDENTIALS_PROVIDER,
    );

    let conf = aws_sdk_s3::Config::builder()
        .behavior_version(BehaviorVersion::latest())
        .region(Region::new(REGION))
        .endpoint_url(&cfg.endpoint)
        .credentials_provider(credentials)
        .force_path_style(true)
        .retry_config(RetryConfig::standard().with_max_attempts(MAX_ATTEMPTS))
        .build();

    Client::from_conf(conf)
}

#[derive(Debug, Clone)]
pub struct ObjectStorage {
    client: Client,
    bucket: String,
}

impl ObjectStorage {
    pub fn new(client: Client, bucket: String) -> Self {
        Self { client, bucket }
    }

    #[instrument(name = "storage.bucket.ensure", skip_all, fields(bucket = %self.bucket))]
    pub async fn ensure_bucket(&self) -> Result<(), ObjectStorageError> {
        let Err(err) = self
            .client
            .create_bucket()
            .bucket(&self.bucket)
            .send()
            .await
        else {
            return Ok(());
        };

        if matches!(
            err.as_service_error(),
            Some(
                CreateBucketError::BucketAlreadyOwnedByYou(_)
                    | CreateBucketError::BucketAlreadyExists(_)
            )
        ) {
            return Ok(());
        }

        let err = ObjectStorageError::CreateBucket(Box::new(err));
        err.log_internal();

        Err(err)
    }

    #[instrument(name = "storage.object.upload", skip_all, fields(bucket = %self.bucket, key = %key))]
    pub async fn upload(
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

    #[instrument(name = "storage.object.presign", skip_all, fields(bucket = %self.bucket, key = %key))]
    pub async fn presign(
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

    #[instrument(name = "storage.object.delete", skip_all, fields(bucket = %self.bucket, key = %key))]
    pub async fn delete(&self, key: &str) -> Result<(), ObjectStorageError> {
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
