use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum ImageRotation {
    Degree90,
    Degree180,
    Degree270,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ImageFlip {
    Horizontal,
    Vertical,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ImageTransformation {
    RotateClockwise(ImageRotation),
    Flip(ImageFlip),
    // TODO: add more transformations (crop, blur, contrast, etc.)
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
pub enum ImageOutputFormat {
    Jpeg,
    Png,
    // TODO: WebP - Image crate doesn't support this yet
}

impl From<ImageOutputFormat> for String {
    fn from(format: ImageOutputFormat) -> String {
        match format {
            ImageOutputFormat::Jpeg => "image/jpeg",
            ImageOutputFormat::Png => "image/png",
        }
        .to_string()
    }
}

impl From<ImageOutputFormat> for image::ImageFormat {
    fn from(format: ImageOutputFormat) -> image::ImageFormat {
        match format {
            ImageOutputFormat::Jpeg => image::ImageFormat::Jpeg,
            ImageOutputFormat::Png => image::ImageFormat::Png,
        }
    }
}

impl ImageOutputFormat {
    pub fn from_mime(mime: &str) -> Option<Self> {
        match mime {
            "image/jpeg" => Some(ImageOutputFormat::Jpeg),
            "image/png" => Some(ImageOutputFormat::Png),
            _ => None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ImagePipeline {
    pub output_format: Option<ImageOutputFormat>,
    pub dimensions: Option<Vec<(u32, u32)>>,
    // pub quality: Option<u8>,
    pub transformations: Option<Vec<ImageTransformation>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum FileTransformPipeline {
    Image(ImagePipeline),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FileUpload {
    pub key: String,
    pub url: String,
    pub artifacts: Option<Vec<FileUpload>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FilesUploadResponse {
    pub message: String,
    pub files: Vec<FileUpload>,
}
