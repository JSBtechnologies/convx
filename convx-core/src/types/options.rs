use super::format::Format;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConversionOptions {
    pub output_format: Format,
    pub quality: Option<u8>,
    pub max_file_size: Option<u64>,
    pub document: Option<DocumentOptions>,
    pub image: Option<ImageOptions>,
    pub video: Option<VideoOptions>,
    pub audio: Option<AudioOptions>,
    pub overwrite: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DocumentOptions {
    pub page_start: Option<u32>,
    pub page_end: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImageOptions {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub strip_metadata: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VideoOptions {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fps: Option<f32>,
    pub crf: Option<u8>,
    pub no_audio: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AudioOptions {
    pub bitrate: Option<String>,
    pub sample_rate: Option<u32>,
}
