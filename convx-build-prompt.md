Build a local-first file conversion tool called "convx" using the specifications below.

**Tech Stack:**
- Rust core engine (CLI + library)
- Quasar/Vue 3 frontend (desktop via Tauri, web via SPA)
- FFmpeg for video/audio (shell out to CLI)
- libvips for images (shell out to CLI)

**Build Order:**
1. Rust core engine + CLI (Phases 1-5)
2. Run tests to verify it works
3. Quasar frontend + Tauri integration

**Start with Phase 1. After each phase, run the checkpoint test before proceeding.**

---

# SPECIFICATION 1: CORE ENGINE API

## Overview

convx is a local-first file conversion engine. All processing happens on the user's machine. The core is written in Rust and provides a unified API for CLI, desktop, web (WASM), and mobile.

## Project Structure

```
convx-core/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── main.rs              # CLI entry point
│   ├── engine.rs            # Main ConvxEngine
│   ├── types/
│   │   ├── mod.rs
│   │   ├── format.rs        # Format enum
│   │   ├── options.rs       # ConversionOptions
│   │   ├── result.rs        # ConversionResult
│   │   └── error.rs         # ConvxError
│   ├── converters/
│   │   ├── mod.rs           # Converter trait + registry
│   │   ├── image.rs         # libvips wrapper
│   │   ├── video.rs         # FFmpeg wrapper
│   │   └── audio.rs         # FFmpeg wrapper
│   └── utils/
│       ├── mod.rs
│       └── deps.rs          # Dependency checking
└── tests/
    └── integration/
```

## Cargo.toml

```toml
[package]
name = "convx-core"
version = "0.1.0"
edition = "2021"

[lib]
name = "convx"
path = "src/lib.rs"

[[bin]]
name = "convx"
path = "src/main.rs"

[dependencies]
tokio = { version = "1", features = ["full"] }
thiserror = "1"
anyhow = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
clap = { version = "4", features = ["derive"] }
indicatif = "0.17"
console = "0.15"
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
bytesize = { version = "1", features = ["serde"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
walkdir = "2"
tempfile = "3"

[dev-dependencies]
assert_cmd = "2"
predicates = "3"
assert_fs = "1"
```

## Core Types

### Format (src/types/format.rs)

```rust
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    // Images
    Png, Jpg, Jpeg, WebP, Gif, Bmp, Tiff, Ico, Svg, Heic, Heif, Avif,
    // Video
    Mp4, Mov, Webm, Avi, Mkv, Wmv, Flv, M4v, Mpeg, Ts,
    // Audio
    Mp3, Wav, Flac, M4a, Aac, Ogg, Wma, Aiff, Opus, Ac3,
    // Documents
    Pdf, Docx, Doc, Pptx, Xlsx, Txt, Md, Html,
    // Data (including ML formats)
    Csv, Json, Yaml, Xml, Parquet, Jsonl, Tsv, Arrow, Sqlite, Npy, Npz, Hdf5,
    // Ebooks
    Epub, Mobi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatCategory {
    Image,
    Video,
    Audio,
    Document,
    Data,
    Ebook,
}

impl Format {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            // Images
            "png" => Some(Self::Png),
            "jpg" | "jpeg" => Some(Self::Jpg),
            "webp" => Some(Self::WebP),
            "gif" => Some(Self::Gif),
            "bmp" => Some(Self::Bmp),
            "tiff" | "tif" => Some(Self::Tiff),
            "ico" => Some(Self::Ico),
            "svg" => Some(Self::Svg),
            "heic" => Some(Self::Heic),
            "heif" => Some(Self::Heif),
            "avif" => Some(Self::Avif),
            // Video
            "mp4" => Some(Self::Mp4),
            "mov" => Some(Self::Mov),
            "webm" => Some(Self::Webm),
            "avi" => Some(Self::Avi),
            "mkv" => Some(Self::Mkv),
            "wmv" => Some(Self::Wmv),
            "flv" => Some(Self::Flv),
            "m4v" => Some(Self::M4v),
            "mpeg" => Some(Self::Mpeg),
            "ts" => Some(Self::Ts),
            // Audio
            "mp3" => Some(Self::Mp3),
            "wav" => Some(Self::Wav),
            "flac" => Some(Self::Flac),
            "m4a" => Some(Self::M4a),
            "aac" => Some(Self::Aac),
            "ogg" => Some(Self::Ogg),
            "wma" => Some(Self::Wma),
            "aiff" | "aif" => Some(Self::Aiff),
            "opus" => Some(Self::Opus),
            "ac3" => Some(Self::Ac3),
            // Documents
            "pdf" => Some(Self::Pdf),
            "docx" => Some(Self::Docx),
            "doc" => Some(Self::Doc),
            "pptx" => Some(Self::Pptx),
            "xlsx" => Some(Self::Xlsx),
            "txt" => Some(Self::Txt),
            "md" | "markdown" => Some(Self::Md),
            "html" | "htm" => Some(Self::Html),
            // Data (including ML formats)
            "csv" => Some(Self::Csv),
            "json" => Some(Self::Json),
            "yaml" | "yml" => Some(Self::Yaml),
            "xml" => Some(Self::Xml),
            "parquet" => Some(Self::Parquet),
            "jsonl" | "ndjson" => Some(Self::Jsonl),
            "tsv" => Some(Self::Tsv),
            "arrow" | "feather" => Some(Self::Arrow),
            "sqlite" | "db" => Some(Self::Sqlite),
            "npy" => Some(Self::Npy),
            "npz" => Some(Self::Npz),
            "h5" | "hdf5" => Some(Self::Hdf5),
            // Ebooks
            "epub" => Some(Self::Epub),
            "mobi" => Some(Self::Mobi),
            _ => None,
        }
    }

    // extension() and category() follow the same pattern —
    // see convx-core/src/types/format.rs for the full implementation.
    // All 54 formats map to their canonical extension and one of 6 categories.

    pub fn detect(path: &Path) -> Option<Self> {
        path.extension()
            .and_then(|e| e.to_str())
            .and_then(Self::from_extension)
    }
}
```

### Options (src/types/options.rs)

```rust
use serde::{Deserialize, Serialize};
use super::Format;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConversionOptions {
    pub output_format: Format,
    pub quality: Option<u8>,
    pub image: Option<ImageOptions>,
    pub video: Option<VideoOptions>,
    pub audio: Option<AudioOptions>,
    pub overwrite: bool,
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
```

### Result (src/types/result.rs)

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use super::Format;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionResult {
    pub id: Uuid,
    pub status: ConversionStatus,
    pub input_path: PathBuf,
    pub output_path: Option<PathBuf>,
    pub input_format: Format,
    pub output_format: Format,
    pub input_size: u64,
    pub output_size: Option<u64>,
    pub space_saved: Option<i64>,
    pub duration_ms: u64,
    pub error: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConversionStatus {
    Completed,
    Failed,
}
```

### Error (src/types/error.rs)

```rust
use std::path::PathBuf;
use thiserror::Error;
use super::Format;

#[derive(Debug, Error)]
pub enum ConvxError {
    #[error("File not found: {path}")]
    FileNotFound { path: PathBuf },
    
    #[error("Cannot read file: {path}: {reason}")]
    FileReadError { path: PathBuf, reason: String },
    
    #[error("Cannot write file: {path}: {reason}")]
    FileWriteError { path: PathBuf, reason: String },
    
    #[error("Unknown format: {format}")]
    UnknownFormat { format: String },
    
    #[error("Cannot detect format for: {path}")]
    FormatDetectionFailed { path: PathBuf },
    
    #[error("Unsupported conversion: {from:?} → {to:?}")]
    UnsupportedConversion { from: Format, to: Format },
    
    #[error("Conversion failed: {reason}")]
    ConversionFailed { reason: String },
    
    #[error("FFmpeg not found. Please install FFmpeg.")]
    FfmpegNotFound,
    
    #[error("libvips not found. Please install libvips.")]
    VipsNotFound,
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

## Converters

### Image Converter (src/converters/image.rs)

Shell out to `vips` CLI:

```rust
use std::path::Path;
use std::process::Command;
use crate::{ConversionOptions, ConversionResult, ConversionStatus, ConvxError, Format};
use uuid::Uuid;
use chrono::Utc;

pub struct ImageConverter;

impl ImageConverter {
    pub fn convert(
        &self,
        input: &Path,
        output: &Path,
        options: &ConversionOptions,
    ) -> Result<ConversionResult, ConvxError> {
        let start = std::time::Instant::now();
        let input_size = std::fs::metadata(input)
            .map_err(|e| ConvxError::FileReadError {
                path: input.to_path_buf(),
                reason: e.to_string(),
            })?
            .len();

        // Build vips command
        let mut cmd = Command::new("vips");
        cmd.arg("copy");
        cmd.arg(input);
        
        // Add quality suffix for lossy formats
        let output_str = if let Some(q) = options.quality {
            match options.output_format {
                Format::Jpg | Format::Jpeg => format!("{}[Q={}]", output.display(), q),
                Format::WebP => format!("{}[Q={}]", output.display(), q),
                _ => output.display().to_string(),
            }
        } else {
            output.display().to_string()
        };
        cmd.arg(&output_str);

        let status = cmd.output().map_err(|_| ConvxError::VipsNotFound)?;

        if !status.status.success() {
            return Err(ConvxError::ConversionFailed {
                reason: String::from_utf8_lossy(&status.stderr).to_string(),
            });
        }

        let output_size = std::fs::metadata(output)
            .map_err(|e| ConvxError::FileWriteError {
                path: output.to_path_buf(),
                reason: e.to_string(),
            })?
            .len();

        Ok(ConversionResult {
            id: Uuid::new_v4(),
            status: ConversionStatus::Completed,
            input_path: input.to_path_buf(),
            output_path: Some(output.to_path_buf()),
            input_format: Format::detect(input).unwrap_or(Format::Png),
            output_format: options.output_format,
            input_size,
            output_size: Some(output_size),
            space_saved: Some(input_size as i64 - output_size as i64),
            duration_ms: start.elapsed().as_millis() as u64,
            error: None,
            timestamp: Utc::now(),
        })
    }

    pub fn can_convert(&self, from: Format, to: Format) -> bool {
        matches!(from.category(), crate::FormatCategory::Image)
            && matches!(to.category(), crate::FormatCategory::Image)
    }
}
```

### Video Converter (src/converters/video.rs)

Shell out to `ffmpeg` CLI:

```rust
use std::path::Path;
use std::process::Command;
use crate::{ConversionOptions, ConversionResult, ConversionStatus, ConvxError, Format, FormatCategory};
use uuid::Uuid;
use chrono::Utc;

pub struct VideoConverter;

impl VideoConverter {
    pub fn convert(
        &self,
        input: &Path,
        output: &Path,
        options: &ConversionOptions,
    ) -> Result<ConversionResult, ConvxError> {
        let start = std::time::Instant::now();
        let input_size = std::fs::metadata(input)
            .map_err(|e| ConvxError::FileReadError {
                path: input.to_path_buf(),
                reason: e.to_string(),
            })?
            .len();

        let mut args: Vec<String> = vec![
            "-i".to_string(),
            input.to_string_lossy().to_string(),
            "-y".to_string(), // Overwrite
        ];

        // Handle GIF output specially
        if options.output_format == Format::Gif {
            let fps = options.video.as_ref().and_then(|v| v.fps).unwrap_or(10.0);
            let width = options.video.as_ref().and_then(|v| v.width).unwrap_or(480);
            
            args.extend([
                "-vf".to_string(),
                format!("fps={},scale={}:-1:flags=lanczos", fps, width),
            ]);
        } else {
            // Video codec
            args.extend(["-c:v".to_string(), "libx264".to_string()]);
            
            // CRF quality
            let crf = options.video.as_ref().and_then(|v| v.crf).unwrap_or(23);
            args.extend(["-crf".to_string(), crf.to_string()]);
            
            // Resolution
            if let Some(ref video) = options.video {
                if let (Some(w), Some(h)) = (video.width, video.height) {
                    args.extend(["-vf".to_string(), format!("scale={}:{}", w, h)]);
                }
            }
            
            // Audio
            if options.video.as_ref().map(|v| v.no_audio).unwrap_or(false) {
                args.push("-an".to_string());
            } else {
                args.extend(["-c:a".to_string(), "aac".to_string()]);
            }
        }

        args.push(output.to_string_lossy().to_string());

        let status = Command::new("ffmpeg")
            .args(&args)
            .output()
            .map_err(|_| ConvxError::FfmpegNotFound)?;

        if !status.status.success() {
            return Err(ConvxError::ConversionFailed {
                reason: String::from_utf8_lossy(&status.stderr).to_string(),
            });
        }

        let output_size = std::fs::metadata(output)
            .map_err(|e| ConvxError::FileWriteError {
                path: output.to_path_buf(),
                reason: e.to_string(),
            })?
            .len();

        Ok(ConversionResult {
            id: Uuid::new_v4(),
            status: ConversionStatus::Completed,
            input_path: input.to_path_buf(),
            output_path: Some(output.to_path_buf()),
            input_format: Format::detect(input).unwrap_or(Format::Mp4),
            output_format: options.output_format,
            input_size,
            output_size: Some(output_size),
            space_saved: Some(input_size as i64 - output_size as i64),
            duration_ms: start.elapsed().as_millis() as u64,
            error: None,
            timestamp: Utc::now(),
        })
    }

    pub fn can_convert(&self, from: Format, to: Format) -> bool {
        let from_ok = matches!(from.category(), FormatCategory::Video);
        let to_ok = matches!(to.category(), FormatCategory::Video) || to == Format::Gif;
        from_ok && to_ok
    }
}
```

### Audio Converter (src/converters/audio.rs)

```rust
use std::path::Path;
use std::process::Command;
use crate::{ConversionOptions, ConversionResult, ConversionStatus, ConvxError, Format, FormatCategory};
use uuid::Uuid;
use chrono::Utc;

pub struct AudioConverter;

impl AudioConverter {
    pub fn convert(
        &self,
        input: &Path,
        output: &Path,
        options: &ConversionOptions,
    ) -> Result<ConversionResult, ConvxError> {
        let start = std::time::Instant::now();
        let input_size = std::fs::metadata(input)
            .map_err(|e| ConvxError::FileReadError {
                path: input.to_path_buf(),
                reason: e.to_string(),
            })?
            .len();

        let mut args: Vec<String> = vec![
            "-i".to_string(),
            input.to_string_lossy().to_string(),
            "-y".to_string(),
        ];

        // Codec based on output format
        let codec = match options.output_format {
            Format::Mp3 => "libmp3lame",
            Format::Aac | Format::M4a => "aac",
            Format::Opus | Format::Ogg => "libopus",
            Format::Flac => "flac",
            Format::Wav => "pcm_s16le",
            _ => "copy",
        };
        args.extend(["-c:a".to_string(), codec.to_string()]);

        // Bitrate
        if let Some(ref audio) = options.audio {
            if let Some(ref bitrate) = audio.bitrate {
                args.extend(["-b:a".to_string(), bitrate.clone()]);
            }
        }

        args.push(output.to_string_lossy().to_string());

        let status = Command::new("ffmpeg")
            .args(&args)
            .output()
            .map_err(|_| ConvxError::FfmpegNotFound)?;

        if !status.status.success() {
            return Err(ConvxError::ConversionFailed {
                reason: String::from_utf8_lossy(&status.stderr).to_string(),
            });
        }

        let output_size = std::fs::metadata(output)
            .map_err(|e| ConvxError::FileWriteError {
                path: output.to_path_buf(),
                reason: e.to_string(),
            })?
            .len();

        Ok(ConversionResult {
            id: Uuid::new_v4(),
            status: ConversionStatus::Completed,
            input_path: input.to_path_buf(),
            output_path: Some(output.to_path_buf()),
            input_format: Format::detect(input).unwrap_or(Format::Mp3),
            output_format: options.output_format,
            input_size,
            output_size: Some(output_size),
            space_saved: Some(input_size as i64 - output_size as i64),
            duration_ms: start.elapsed().as_millis() as u64,
            error: None,
            timestamp: Utc::now(),
        })
    }

    pub fn can_convert(&self, from: Format, to: Format) -> bool {
        matches!(from.category(), FormatCategory::Audio)
            && matches!(to.category(), FormatCategory::Audio)
    }
}
```

## Main Engine (src/engine.rs)

```rust
use std::path::Path;
use crate::{ConversionOptions, ConversionResult, ConvxError, Format, FormatCategory};
use crate::converters::{ImageConverter, VideoConverter, AudioConverter};

pub struct ConvxEngine {
    image: ImageConverter,
    video: VideoConverter,
    audio: AudioConverter,
}

impl ConvxEngine {
    pub fn new() -> Result<Self, ConvxError> {
        Ok(Self {
            image: ImageConverter,
            video: VideoConverter,
            audio: AudioConverter,
        })
    }

    pub fn convert(
        &self,
        input: &Path,
        output: &Path,
        options: ConversionOptions,
    ) -> Result<ConversionResult, ConvxError> {
        // Check input exists
        if !input.exists() {
            return Err(ConvxError::FileNotFound {
                path: input.to_path_buf(),
            });
        }

        // Detect input format
        let input_format = Format::detect(input).ok_or_else(|| ConvxError::FormatDetectionFailed {
            path: input.to_path_buf(),
        })?;

        let output_format = options.output_format;

        // Route to appropriate converter
        match (input_format.category(), output_format.category()) {
            (FormatCategory::Image, FormatCategory::Image) => {
                self.image.convert(input, output, &options)
            }
            (FormatCategory::Video, FormatCategory::Video) => {
                self.video.convert(input, output, &options)
            }
            (FormatCategory::Video, FormatCategory::Image) if output_format == Format::Gif => {
                self.video.convert(input, output, &options)
            }
            (FormatCategory::Audio, FormatCategory::Audio) => {
                self.audio.convert(input, output, &options)
            }
            _ => Err(ConvxError::UnsupportedConversion {
                from: input_format,
                to: output_format,
            }),
        }
    }

    pub fn can_convert(&self, from: Format, to: Format) -> bool {
        self.image.can_convert(from, to)
            || self.video.can_convert(from, to)
            || self.audio.can_convert(from, to)
    }
}
```

## CLI (src/main.rs)

```rust
use clap::{Parser, Subcommand};
use convx::{ConvxEngine, ConversionOptions, Format};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "convx")]
#[command(about = "Local-first file conversion. Your files never leave your machine.")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert a file
    Convert {
        /// Input file
        input: PathBuf,

        /// Output file (optional, defaults to input with new extension)
        output: Option<PathBuf>,

        /// Output format
        #[arg(short, long)]
        to: Option<String>,

        /// Quality (0-100)
        #[arg(short, long)]
        quality: Option<u8>,

        /// FPS for GIF output
        #[arg(long)]
        fps: Option<f32>,

        /// Width
        #[arg(short, long)]
        width: Option<u32>,
    },

    /// List supported formats
    Formats,

    /// Show version
    Version,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let engine = ConvxEngine::new()?;

    match cli.command {
        Commands::Convert {
            input,
            output,
            to,
            quality,
            fps,
            width,
        } => {
            // Determine output format
            let output_format = to
                .as_deref()
                .and_then(Format::from_extension)
                .or_else(|| output.as_ref().and_then(|p| Format::detect(p)))
                .expect("Could not determine output format. Use --to flag.");

            // Determine output path
            let output = output.unwrap_or_else(|| {
                let mut out = input.clone();
                out.set_extension(output_format.extension());
                out
            });

            let options = ConversionOptions {
                output_format,
                quality,
                video: Some(convx::VideoOptions {
                    fps,
                    width,
                    ..Default::default()
                }),
                ..Default::default()
            };

            let result = engine.convert(&input, &output, options)?;

            println!("✓ Converted: {} → {}", input.display(), output.display());
            println!(
                "  Size: {} → {} ({:+.1}%)",
                format_size(result.input_size),
                format_size(result.output_size.unwrap_or(0)),
                if result.input_size > 0 {
                    ((result.output_size.unwrap_or(0) as f64 / result.input_size as f64) - 1.0) * 100.0
                } else {
                    0.0
                }
            );
            println!("  Time: {}ms", result.duration_ms);
        }

        Commands::Formats => {
            println!("Supported formats:\n");
            println!("  Images:    png, jpg, webp, gif, bmp, tiff, ico, svg, heic, heif, avif");
            println!("  Video:     mp4, mov, webm, avi, mkv, wmv, flv, m4v, mpeg, ts");
            println!("  Audio:     mp3, wav, flac, m4a, aac, ogg, wma, aiff, opus, ac3");
            println!("  Documents: pdf, docx, doc, pptx, xlsx, txt, md, html");
            println!("  Data:      csv, json, yaml, xml, parquet, jsonl, tsv, arrow, sqlite, npy, npz, h5");
            println!("  Ebooks:    epub, mobi");
        }

        Commands::Version => {
            println!("convx {}", env!("CARGO_PKG_VERSION"));
        }
    }

    Ok(())
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
```

## lib.rs

```rust
pub mod types;
pub mod converters;
pub mod engine;

pub use types::format::{Format, FormatCategory};
pub use types::options::{ConversionOptions, ImageOptions, VideoOptions, AudioOptions};
pub use types::result::{ConversionResult, ConversionStatus};
pub use types::error::ConvxError;
pub use engine::ConvxEngine;
```

---

# SPECIFICATION 2: TEST SUITE

## Test Files

Download test fixtures:

```bash
mkdir -p tests/fixtures
cd tests/fixtures
curl -L -o sample.png "https://www.w3.org/Graphics/PNG/nurbcup2si.png"
curl -L -o sample.jpg "https://www.w3.org/People/Raggett/Images/davephoto.jpg"
curl -L -o sample.mp4 "https://filesamples.com/samples/video/mp4/sample_640x360.mp4"
curl -L -o sample.mp3 "https://filesamples.com/samples/audio/mp3/sample3.mp3"
curl -L -o sample.wav "https://filesamples.com/samples/audio/wav/sample3.wav"
```

Or generate with FFmpeg:

```bash
# Test image
ffmpeg -f lavfi -i testsrc=duration=1:size=640x480:rate=1 -frames:v 1 tests/fixtures/sample.png

# Test video (3 seconds)
ffmpeg -f lavfi -i testsrc=duration=3:size=320x240:rate=30 -f lavfi -i sine=frequency=440:duration=3 -c:v libx264 -c:a aac tests/fixtures/sample.mp4

# Test audio (3 seconds)
ffmpeg -f lavfi -i sine=frequency=440:duration=3 tests/fixtures/sample.wav
```

## Integration Tests (tests/integration.rs)

```rust
use convx::{ConvxEngine, ConversionOptions, ConversionStatus, Format};
use std::path::Path;
use tempfile::TempDir;

fn setup() -> (ConvxEngine, TempDir) {
    let engine = ConvxEngine::new().expect("Failed to create engine");
    let temp = TempDir::new().expect("Failed to create temp dir");
    (engine, temp)
}

#[test]
fn test_png_to_webp() {
    let (engine, temp) = setup();
    let input = Path::new("tests/fixtures/sample.png");
    if !input.exists() {
        eprintln!("Skipping test: sample.png not found");
        return;
    }
    
    let output = temp.path().join("output.webp");
    let options = ConversionOptions {
        output_format: Format::WebP,
        quality: Some(80),
        ..Default::default()
    };

    let result = engine.convert(input, &output, options).unwrap();
    assert_eq!(result.status, ConversionStatus::Completed);
    assert!(output.exists());
}

#[test]
fn test_png_to_jpg() {
    let (engine, temp) = setup();
    let input = Path::new("tests/fixtures/sample.png");
    if !input.exists() { return; }
    
    let output = temp.path().join("output.jpg");
    let options = ConversionOptions {
        output_format: Format::Jpg,
        quality: Some(90),
        ..Default::default()
    };

    let result = engine.convert(input, &output, options).unwrap();
    assert_eq!(result.status, ConversionStatus::Completed);
    assert!(output.exists());
}

#[test]
fn test_mp4_to_gif() {
    let (engine, temp) = setup();
    let input = Path::new("tests/fixtures/sample.mp4");
    if !input.exists() { return; }
    
    let output = temp.path().join("output.gif");
    let options = ConversionOptions {
        output_format: Format::Gif,
        video: Some(convx::VideoOptions {
            fps: Some(10.0),
            width: Some(320),
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = engine.convert(input, &output, options).unwrap();
    assert_eq!(result.status, ConversionStatus::Completed);
    assert!(output.exists());
}

#[test]
fn test_wav_to_mp3() {
    let (engine, temp) = setup();
    let input = Path::new("tests/fixtures/sample.wav");
    if !input.exists() { return; }
    
    let output = temp.path().join("output.mp3");
    let options = ConversionOptions {
        output_format: Format::Mp3,
        audio: Some(convx::AudioOptions {
            bitrate: Some("192k".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = engine.convert(input, &output, options).unwrap();
    assert_eq!(result.status, ConversionStatus::Completed);
    assert!(output.exists());
    // MP3 should be smaller than WAV
    assert!(result.output_size.unwrap() < result.input_size);
}

#[test]
fn test_unsupported_conversion() {
    let (engine, temp) = setup();
    let input = Path::new("tests/fixtures/sample.png");
    if !input.exists() { return; }
    
    let output = temp.path().join("output.mp4");
    let options = ConversionOptions {
        output_format: Format::Mp4,
        ..Default::default()
    };

    let result = engine.convert(input, &output, options);
    assert!(result.is_err());
}
```

## Run Tests

```bash
# Generate fixtures first
mkdir -p tests/fixtures
ffmpeg -f lavfi -i testsrc=duration=1:size=640x480:rate=1 -frames:v 1 tests/fixtures/sample.png -y
ffmpeg -f lavfi -i testsrc=duration=3:size=320x240:rate=30 -f lavfi -i sine=frequency=440:duration=3 -c:v libx264 -c:a aac tests/fixtures/sample.mp4 -y
ffmpeg -f lavfi -i sine=frequency=440:duration=3 tests/fixtures/sample.wav -y

# Run tests
cargo test
```

---

# SPECIFICATION 3: QUASAR FRONTEND

After the Rust engine works, create the Quasar app:

```bash
npm init quasar
# Select: Quasar v2, Vite, TypeScript, Pinia, SCSS
# Name: convx-app

cd convx-app
npm install
quasar mode add tauri
```

## Key Files to Create

1. `src/services/bridge/index.ts` - Unified API
2. `src/services/bridge/tauri.ts` - Tauri IPC bridge
3. `src/pages/IndexPage.vue` - Main conversion UI
4. `src/components/FileDropZone.vue` - Drag & drop
5. `src/components/FormatSelector.vue` - Format picker
6. `src-tauri/src/main.rs` - Tauri backend
7. `src-tauri/src/commands.rs` - IPC handlers

The frontend calls Rust via Tauri's `invoke()`:

```typescript
import { invoke } from '@tauri-apps/api/tauri'

const result = await invoke('convert_file', {
  input: '/path/to/file.png',
  output: '/path/to/output.webp',
  options: { outputFormat: 'webp', quality: 80 }
})
```

---

# BUILD PHASES

## Phase 1: Project Setup
```bash
cargo new convx-core --lib
# Add Cargo.toml content
# Create directory structure
cargo build
```
**Checkpoint:** `cargo build` succeeds

## Phase 2: Core Types
- Create Format enum
- Create ConversionOptions
- Create ConversionResult  
- Create ConvxError

**Checkpoint:** `cargo build` succeeds

## Phase 3: Converters
- ImageConverter (vips CLI)
- VideoConverter (ffmpeg CLI)
- AudioConverter (ffmpeg CLI)

**Checkpoint:** `cargo build` succeeds

## Phase 4: Engine
- ConvxEngine struct
- Routing logic

**Checkpoint:** `cargo build` succeeds

## Phase 5: CLI
- Clap argument parsing
- Convert command
- Formats command

**Checkpoint:** 
```bash
cargo build --release
./target/release/convx --help
./target/release/convx formats
```

## Phase 6: Tests
- Generate fixtures
- Run integration tests

**Checkpoint:** `cargo test` passes

## Phase 7: Quasar Frontend
- Create Quasar app
- Add Tauri mode
- Create bridge layer
- Create UI components
- Wire up to Rust

**Checkpoint:** `quasar dev -m tauri` launches app

---

# DEFINITION OF DONE

The project is complete when:

1. `cargo test` passes all tests
2. CLI works:
   ```bash
   ./target/release/convx convert image.png --to webp
   ./target/release/convx convert video.mp4 --to gif
   ./target/release/convx convert audio.wav --to mp3
   ```
3. Desktop app launches: `quasar dev -m tauri`
4. Can drag & drop a file and convert it in the GUI

---

## PROMPT END

Start with Phase 1. Report back after each phase.
