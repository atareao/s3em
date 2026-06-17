use aws_sdk_s3::{
    Client,
    config::{BehaviorVersion, Credentials, Region},
    operation::get_object::GetObjectOutput,
    primitives::ByteStream,
};
use aws_config::timeout::TimeoutConfig;
use crate::config::Config;

pub struct S3Storage {
    client: Client,
    bucket: String,
}

impl S3Storage {
    pub async fn new(config: &Config) -> Self {
        let credentials = Credentials::new(
            &config.s3_access_key,
            &config.s3_secret_key,
            None,
            None,
            "custom",
        );

        let s3_config = aws_sdk_s3::config::Builder::new()
            .behavior_version(BehaviorVersion::latest())
            .endpoint_url(&config.s3_endpoint)
            .region(Region::new(config.s3_region.clone()))
            .credentials_provider(credentials)
            .force_path_style(config.s3_path_style)
            .timeout_config(
                TimeoutConfig::builder()
                    .connect_timeout(std::time::Duration::from_secs(10))
                    .operation_timeout(std::time::Duration::from_secs(60))
                    .build(),
            )
            .build();

        let client = Client::from_conf(s3_config);
        Self {
            client,
            bucket: config.s3_bucket.clone(),
        }
    }

    pub async fn ensure_bucket_exists(&self) -> Result<(), String> {
        match self.client.head_bucket().bucket(&self.bucket).send().await {
            Ok(_) => Ok(()),
            Err(_) => {
                self.client
                    .create_bucket()
                    .bucket(&self.bucket)
                    .send()
                    .await
                    .map_err(|e| format!("Failed to create bucket: {e}"))?;
                tracing::info!("Bucket '{}' created", self.bucket);
                Ok(())
            }
        }
    }

    pub async fn upload(
        &self,
        key: &str,
        data: ByteStream,
        content_type: &str,
    ) -> Result<String, String> {
        let result = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(data)
            .content_type(content_type)
            .send()
            .await
            .map_err(|e| format!("S3 upload error: {e}"))?;
        Ok(result
            .e_tag()
            .map(|e| e.trim_matches('"').to_string())
            .unwrap_or_default())
    }

    pub async fn download(&self, key: &str) -> Result<GetObjectOutput, String> {
        self.client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| format!("S3 download error: {e}"))
    }

    pub async fn delete(&self, key: &str) -> Result<(), String> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| format!("S3 delete error: {e}"))?;
        Ok(())
    }
}