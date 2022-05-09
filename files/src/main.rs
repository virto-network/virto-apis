mod storage;
mod utils;

use async_std::{fs::File, io::WriteExt};
use mime2ext::mime2ext;
use multer::{Constraints, Multipart, SizeLimit};
use nanoid::nanoid;
use std::path::Path;
use std::sync::Arc;
use std::{io::Error as IoError, path::PathBuf};
use tempfile::TempDir;
use tide::{
    http::headers::HeaderValue,
    prelude::*,
    security::{CorsMiddleware, Origin},
    Request, Response, StatusCode,
};
use utils::buffered_bytes_stream::BufferedBytesStream;

#[derive(Clone)]
struct TempDirState {
    tempdir: Arc<TempDir>,
}

impl TempDirState {
    fn try_new() -> Result<Self, IoError> {
        Ok(Self {
            tempdir: Arc::new(tempfile::tempdir()?),
        })
    }

    fn path(&self) -> &Path {
        self.tempdir.path()
    }
}

async fn upload_files(req: Request<TempDirState>) -> tide::Result {
    // Get temp dir where files will be stored
    let temp_path = req.state().path().to_path_buf();

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

    let body_stream = BufferedBytesStream { inner: req };
    // Only allow 'files' field (can be setted multiple times) with a total size limit of 15MB
    let constraints = Constraints::new()
        .allowed_fields(vec!["files"])
        .size_limit(SizeLimit::new().whole_stream(15 * 1024 * 1024));
    let mut multipart = Multipart::with_constraints(body_stream, multipart_boundary, constraints);
    let mut files: Vec<PathBuf> = vec![];

    // Loop over the parts
    while let Some(mut field) = multipart.next_field().await? {
        tide::log::debug!("Reading file", {
          index: field.index(),
          filename: field.file_name()
        });

        let file_id = nanoid!();
        let path = {
            // Get file path in temp dir
            let mut path = temp_path.join(&file_id);
            // Get extension from mime type
            let ext = field
                .content_type()
                .and_then(|mime| mime2ext(mime.essence_str()))
                .unwrap_or_default();
            path.set_extension(ext);
            path
        };

        // Write file to disk in chunks
        let mut file = File::create(&path).await?;
        // TODO: Handle contraints exceptions (max size reached)
        while let Some(chunk) = field.chunk().await? {
            file.write(&chunk).await?;
        }
        file.flush().await?;
        // TODO: trigger job to upload it to storage (S3, Matrix) and persist job info in DB
        files.push(path);
    }

    let response = {
        let mut response = Response::from(StatusCode::Ok);
        response.set_body(json!({
          "message": "Files uploaded successfully",
          "files": files
        }));
        response
    };
    Ok(response)
}

const DEFAULT_PORT: &str = "5556";

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // TODO: disable debug logs
    tide::log::with_level(tide::log::LevelFilter::Debug);
    let mut app = tide::with_state(TempDirState::try_new()?);
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
