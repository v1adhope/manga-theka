mod book;
mod object;
mod release;

use std::time::Duration;

use aws_config::BehaviorVersion;
use aws_sdk_s3::{
    Client,
    config::{Credentials, Region, timeout::TimeoutConfig},
};
use secrecy::ExposeSecret;
use tokio::{sync::Semaphore, time::sleep};

use crate::config;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const OPERATION_ATTEMPT_TIMEOUT: Duration = Duration::from_secs(5);
const OPERATION_TIMEOUT: Duration = Duration::from_secs(15);
// RustFS accepts at most 8 bucket creations in flight, so hold that ceiling here and
// let the rest queue.
const BUCKET_CREATION_LIMIT: usize = 8;
const BUCKET_CREATION_ATTEMPTS: u32 = 5;
const BUCKET_CREATION_BACKOFF: Duration = Duration::from_millis(250);

static BUCKET_CREATION: Semaphore = Semaphore::const_new(BUCKET_CREATION_LIMIT);

pub async fn client(cfg: &config::ObjectStorage) -> Client {
    let credentials = Credentials::new(
        &cfg.access_key,
        cfg.secret_key.expose_secret(),
        None,
        None,
        "manga-theka",
    );

    let region = Region::new(cfg.region.clone());

    let timeouts = TimeoutConfig::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .operation_attempt_timeout(OPERATION_ATTEMPT_TIMEOUT)
        .operation_timeout(OPERATION_TIMEOUT)
        .build();

    let shared_cfg = aws_config::defaults(BehaviorVersion::latest())
        .region(region)
        .credentials_provider(credentials)
        .endpoint_url(cfg.endpoint.as_str())
        .timeout_config(timeouts)
        .load()
        .await;

    let s3_cfg = aws_sdk_s3::config::Builder::from(&shared_cfg)
        .force_path_style(true)
        .build();

    Client::from_conf(s3_cfg)
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

    pub async fn ensure_bucket(&self) {
        let exists = self
            .client
            .head_bucket()
            .bucket(&self.bucket)
            .send()
            .await
            .is_ok();

        if exists {
            return;
        }

        let _permit = BUCKET_CREATION
            .acquire()
            .await
            .expect("bucket creation semaphore has been closed");

        for attempt in 1..=BUCKET_CREATION_ATTEMPTS {
            let err = match self
                .client
                .create_bucket()
                .bucket(&self.bucket)
                .send()
                .await
            {
                Ok(_) => return,
                Err(e) => e,
            };

            if let Some(service) = err.as_service_error()
                && (service.is_bucket_already_exists() || service.is_bucket_already_owned_by_you())
            {
                return;
            }

            if attempt == BUCKET_CREATION_ATTEMPTS {
                panic!("bucket has not been ensured: {err:?}");
            }

            sleep(BUCKET_CREATION_BACKOFF * attempt).await;
        }
    }
}
