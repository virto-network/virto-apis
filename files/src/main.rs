mod models;
mod service;
mod storage;
mod utils;

use crate::service::{process_file, process_file_field, process_image};
use futures::future::join_all;
use models::{FileTransformPipeline, FilesUploadResponse};
use multer::{Constraints, Multipart, SizeLimit};
use std::io::Error as IoError;
use std::sync::Arc;
use storage::{
    s3::S3Storage,
    storage::{LocalFile, Storage},
};
use tempfile::TempDir;
use tide::{
    http::headers::HeaderValue,
    security::{CorsMiddleware, Origin},
    Body, Request, Response, StatusCode,
};
use utils::buffered_bytes_stream::BufferedBytesStream;

#[derive(Clone)]
struct ServerState {
    tempdir: Arc<TempDir>,
    storage: Arc<dyn Storage + Send + Sync>,
}

impl ServerState {
    fn try_new() -> Result<Self, IoError> {
        let bucket_name = std::env::var("S3_BUCKET_NAME")
            .expect("Environment variable S3_BUCKET_NAME must be set");
        let s3_region =
            std::env::var("S3_REGION").expect("Environment variable S3_REGION must be set");
        let _aws_access_key_id = std::env::var("AWS_ACCESS_KEY_ID")
            .expect("Environment variable AWS_ACCESS_KEY_ID must be set");
        let _aws_access_key_id = std::env::var("AWS_SECRET_ACCESS_KEY")
            .expect("Environment variable AWS_SECRET_ACCESS_KEY must be set");
        let storage =
            S3Storage::new(&bucket_name, &s3_region).expect("Failed to instantiate S3 bucket");

        Ok(Self {
            tempdir: Arc::new(tempfile::tempdir()?),
            storage: Arc::new(storage),
        })
    }
}

async fn upload_files(req: Request<ServerState>) -> tide::Result {
    let storage = Arc::clone(&req.state().storage);
    // Get dir where files will be temporaly stored
    let temp_path = req.state().tempdir.path().to_path_buf();
    tide::log::debug!("temp dir {} ", temp_path.display());

    // Read multipart boundary
    let multipart_boundary = req
        .content_type()
        .filter(|content_type| content_type.essence() == "multipart/form-data")
        .and_then(|content_type| {
            content_type
                .param("boundary")
                .map(|boundary| boundary.to_string())
        });

    let multipart_boundary = match multipart_boundary {
        Some(boundary) => boundary,
        _ => {
            // TODO: Respond with descriptive error
            return Ok(Response::from(StatusCode::BadRequest));
        }
    };

    let body_stream = BufferedBytesStream::new(req);
    // Totall size limit of 100 MB
    let constraints =
        Constraints::new().size_limit(SizeLimit::new().whole_stream(100 * 1024 * 1024));
    let mut multipart = Multipart::with_constraints(body_stream, multipart_boundary, constraints);
    let mut files: Vec<LocalFile> = vec![];
    let mut transform: Option<FileTransformPipeline> = None;

    // Loop over the parts
    while let Some(field) = multipart.next_field().await? {
        let field_index = field.index().clone();
        let field_name = field.name().map(|name| name.to_string());
        let field_filename = field.file_name().map(|name| name.to_string());
        tide::log::debug!("Reading field", {
            index: field_index,
            name: field_name,
            filename: field_filename
        });

        match field_name.as_deref() {
            Some("transform") => {
                transform = field
                    .text()
                    .await
                    .map_err(|_| ())
                    .and_then(|text| {
                        serde_json::from_str::<FileTransformPipeline>(text.as_str()).map_err(|_| ())
                    })
                    .ok();
                continue;
            }
            Some("file") => {
                // proceed with file processing
                ()
            }
            // Ignore unknown fields
            _ => continue,
        };

        if let Ok(local_file) = process_file_field(field, &temp_path).await {
            files.push(local_file);
        } else {
            tide::log::warn!("Failed to process file field", {
                index: field_index,
                name: field_name,
                filename: field_filename
            });
        }
    }

    let pipeline = match transform {
        Some(FileTransformPipeline::Image(pipeline)) => Some(pipeline),
        None => None,
    };

    let uploads = join_all(files.into_iter().map(|file| async {
        if file.is_image() {
            return process_image(file, storage.as_ref(), &pipeline).await;
        }
        process_file(file, storage.as_ref()).await
    }))
    .await;

    let response = {
        println!("{:?}", uploads);
        let failed_uploads = uploads.iter().filter(|upload| upload.is_err()).count();
        let message = if failed_uploads > 0 {
            format!("{} files failed to upload", failed_uploads)
        } else {
            "All files uploaded".to_string()
        };
        Response::builder(StatusCode::Ok)
            .body(Body::from_json(&FilesUploadResponse {
                message,
                files: uploads
                    .into_iter()
                    .filter(|upload| !upload.is_err())
                    .map(|upload| upload.unwrap())
                    .collect(),
            })?)
            .build()
    };

    Ok(response)
}

const DEFAULT_PORT: &str = "5556";

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // TODO: disable debug logs
    tide::log::with_level(tide::log::LevelFilter::Debug);
    let mut app = tide::with_state(ServerState::try_new()?);
    app.with(tide::log::LogMiddleware::new());

    app.with(
        CorsMiddleware::new()
            .allow_methods("*".parse::<HeaderValue>().unwrap())
            .allow_origin(Origin::from("*"))
            .allow_credentials(false),
    );

    app.at("upload").post(upload_files);
    let port = std::env::var("PORT").unwrap_or(DEFAULT_PORT.into());
    let addr = format!("0.0.0.0:{}", port);
    app.listen(addr).await?;
    Ok(())
}
