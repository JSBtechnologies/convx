use serde::{Deserialize, Serialize};

use super::{
    format::Format,
    options::{AudioOptions, ImageOptions, VideoOptions},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub name: &'static str,
    pub description: &'static str,
    pub output_format: Format,
    pub quality: Option<u8>,
    /// Target maximum file size in bytes.
    /// The engine applies best-effort iterative tuning to stay under this limit.
    pub max_file_size: Option<u64>,
    pub video: Option<VideoOptions>,
    pub audio: Option<AudioOptions>,
    pub image: Option<ImageOptions>,
}
