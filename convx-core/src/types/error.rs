use super::format::Format;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConvxError {
    #[error("File not found: {path}")]
    FileNotFound { path: PathBuf },

    #[error("Cannot read file: {path}: {reason}")]
    FileReadError { path: PathBuf, reason: String },

    #[error("Cannot write file: {path}: {reason}")]
    FileWriteError { path: PathBuf, reason: String },

    #[error("Output file already exists: {path}")]
    OutputAlreadyExists { path: PathBuf },

    #[error("Unknown format: {format}")]
    UnknownFormat { format: String },

    #[error("Unknown preset: {preset}")]
    UnknownPreset { preset: String },

    #[error("Cannot detect format for: {path}")]
    FormatDetectionFailed { path: PathBuf },

    #[error("Unsupported conversion: {from:?} → {to:?}")]
    UnsupportedConversion { from: Format, to: Format },

    #[error("Document conversion is not supported yet. This is planned for a future release.")]
    DocumentConversionNotYetSupported,

    #[error("Conversion failed: {reason}")]
    ConversionFailed { reason: String },

    #[error("Conversion was cancelled")]
    Cancelled,

    #[error("FFmpeg not found. Please install FFmpeg.")]
    FfmpegNotFound,

    #[error("libvips not found. Please install libvips.")]
    VipsNotFound,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
