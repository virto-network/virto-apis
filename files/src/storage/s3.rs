use std::error::Error;

use super::storage::{LocalFile, Storage};
use async_std::fs::read;
use async_trait::async_trait;
use s3::{bucket::Bucket, creds::Credentials, Region};

#[derive(Clone)]
pub struct S3Storage {
    bucket: Bucket,
}

impl S3Storage {
    pub fn new(bucket_name: &str, region: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let region = region.parse::<Region>()?;
        let credentials = Credentials::from_env()?;
        Ok(S3Storage {
            bucket: Bucket::new(bucket_name, region, credentials)?,
        })
    }
}

#[async_trait]
impl Storage for S3Storage {
    async fn get_file(&self, key: &str) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        let (object, _status_code) = self.bucket.get_object(key).await?;
        Ok(object)
    }

    async fn put_file(
        &self,
        local_file: &LocalFile,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        let content = read(&local_file.path).await?;
        let _status_code = self
            .bucket
            .put_object_with_content_type(
                &local_file.key,
                &content,
                &local_file
                    .content_type
                    .clone()
                    .unwrap_or("application/octet-stream".to_string())
                    .to_string(),
            )
            .await?;
        Ok(self.get_uri_from_key(&local_file.key))
    }

    async fn delete_file(&self, key: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        let (_, _status_code) = self.bucket.delete_object(key).await?;
        Ok(())
    }

    fn get_uri_from_key(&self, key: &str) -> String {
        format!("https://{}.s3.amazonaws.com/{}", self.bucket.name(), key)
    }
}
