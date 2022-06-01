use crate::{
    models::{
        FileUpload, ImageFlip, ImageOutputFormat, ImagePipeline, ImageRotation, ImageTransformation,
    },
    storage::storage::{LocalFile, Storage},
};
use async_std::{fs::File, io::WriteExt};
use futures::future::join_all;
use image::{DynamicImage, ImageFormat};
use mime2ext::mime2ext;
use multer::Field;
use nanoid::nanoid;
use std::{error::Error, path::PathBuf};

pub async fn process_image(
    mut file: LocalFile,
    storage: &(dyn Storage + Send + Sync),
    pipeline: &Option<ImagePipeline>,
) -> Result<FileUpload, Box<dyn Error + Send + Sync>> {
    let mut img = image::open(&file.path)?;
    let mut dimensions: Vec<DynamicImage> = vec![];
    let mut output_format = file
        .content_type
        .as_ref()
        .and_then(|content_type| ImageOutputFormat::from_mime(content_type.as_str()))
        .expect("Unsupported content type");

    if let Some(pipeline) = pipeline {
        if let Some(format) = &pipeline.output_format {
            output_format = *format;
        }

        for transformation in pipeline.transformations.iter().flatten() {
            img = match transformation {
                ImageTransformation::RotateClockwise(rotation) => match rotation {
                    ImageRotation::Degree90 => img.rotate90(),
                    ImageRotation::Degree180 => img.rotate180(),

                    ImageRotation::Degree270 => img.rotate270(),
                },
                ImageTransformation::Flip(flip) => match flip {
                    ImageFlip::Horizontal => img.fliph(),
                    ImageFlip::Vertical => img.flipv(),
                },
            }
        }

        for &(width, height) in pipeline.dimensions.iter().flatten() {
            dimensions.push(img.resize(width, height, image::imageops::FilterType::Triangle));
        }
    }

    let format: ImageFormat = output_format.into();
    img.save_with_format(&file.path, format)?;
    file.content_type = Some(String::from(output_format));

    let mut main_file_upload = FileUpload {
        key: file.key.clone(),
        url: storage.put_file(&file).await?,
        artifacts: None,
    };

    let artifacts = dimensions
        .into_iter()
        .map(|img| {
            let key = format!("{}/{}", file.key, nanoid!());
            let path = file.path.clone().with_file_name(nanoid!());
            let result = img.save_with_format(&path, format);
            result.map(|_| LocalFile::new(key, path, file.content_type.clone()))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let uploads = join_all(artifacts.iter().map(|file| storage.put_file(file)))
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

    main_file_upload.artifacts = Some(
        artifacts
            .into_iter()
            .zip(uploads)
            .map(|(file, upload_url)| FileUpload {
                key: file.key,
                url: upload_url,
                artifacts: None,
            })
            .collect(),
    );

    Ok(main_file_upload)
}

pub async fn process_file(
    file: LocalFile,
    storage: &(dyn Storage + Send + Sync),
) -> Result<FileUpload, Box<dyn Error + Send + Sync>> {
    storage.put_file(&file).await.map(|upload_url| FileUpload {
        key: file.key,
        url: upload_url,
        artifacts: None,
    })
}

pub async fn process_file_field(
    mut field: Field<'_>,
    temp_path: &PathBuf,
) -> Result<LocalFile, Box<dyn Error + Send + Sync>> {
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
        file.write_all(&chunk).await?;
    }
    file.flush().await?;
    Ok(LocalFile::new(
        file_id,
        path,
        field
            .content_type()
            .map(|mime| mime.essence_str().to_owned()),
    ))
}
