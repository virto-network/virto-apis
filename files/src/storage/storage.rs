use std::path::PathBuf;

use async_trait::async_trait;

pub struct LocalFile {
    pub key: String,
    pub path: PathBuf,
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
}

#[async_trait]
pub trait Storage {
    async fn get(&self, key: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>>;
    async fn put(&self, local_file: &LocalFile) -> Result<(), Box<dyn std::error::Error>>;
    async fn delete(&self, key: &str) -> Result<(), Box<dyn std::error::Error>>;
}
