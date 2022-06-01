use std::{error::Error, path::PathBuf};

use async_trait::async_trait;

pub struct LocalFile {
    // Unique identifier for the file
    pub key: String,
    // Path to the file
    pub path: PathBuf,
    // Mime type of the file
    pub content_type: Option<String>,
}

impl LocalFile {
    pub fn new(key: String, path: PathBuf, content_type: Option<String>) -> Self {
        Self {
            key,
            path,
            content_type,
        }
    }

    pub fn is_image(&self) -> bool {
        self.content_type
            .as_ref()
            .map(|content_type| content_type.starts_with("image/"))
            .unwrap_or(false)
    }
}

#[async_trait]
pub trait Storage {
    async fn get_file(&self, key: &str) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>>;

    async fn put_file(
        &self,
        local_file: &LocalFile,
    ) -> Result<String, Box<dyn Error + Send + Sync>>;

    async fn delete_file(&self, key: &str) -> Result<(), Box<dyn Error + Send + Sync>>;

    fn get_uri_from_key(&self, key: &str) -> String;
}
