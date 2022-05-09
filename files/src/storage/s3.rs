use async_trait::async_trait;
use s3::{bucket::Bucket, creds::Credentials, Region};

use super::storage::{LocalFile, Storage};

struct S3 {
    bucket: Bucket,
}

impl S3 {
    pub fn new(bucket_name: &str, region: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let region = region.parse::<Region>()?;
        let credentials = Credentials::from_env()?;
        Ok(S3 {
            bucket: Bucket::new(bucket_name, region, credentials)?,
        })
    }
}

#[async_trait]
impl Storage for S3 {
    async fn get(&self, key: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let (object, _status_code) = self.bucket.get_object(key).await?;
        Ok(object)
    }

    async fn put(&self, local_file: &LocalFile) -> Result<(), Box<dyn std::error::Error>> {
        let mut path = async_std::fs::File::open(&local_file.path).await?;
        let _status_code = self
            .bucket
            .put_object_stream(&mut path, &local_file.key)
            .await?;
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<(), Box<dyn std::error::Error>> {
        let (_, _status_code) = self.bucket.delete_object(key).await?;
        Ok(())
    }
}
