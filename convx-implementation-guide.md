# convx Implementation Guide

**For:** Coding agents (Claude Code, Cursor, Aider, Copilot)  
**Goal:** Build the convx core engine from spec  
**Estimated effort:** 40-60 hours

---

## Build Order

Complete each phase before moving to the next. Each phase has a checkpoint test.

---

## Phase 1: Project Scaffolding

### 1.1 Create project structure

```bash
cargo new convx-core --lib
cd convx-core
mkdir -p src/{types,converters,config,batch,progress,utils}
mkdir -p tests/{integration,fixtures}
```

### 1.2 Set up Cargo.toml

```toml
[package]
name = "convx-core"
version = "0.1.0"
edition = "2021"
license = "MIT"
description = "Local-first file conversion engine"

[lib]
name = "convx"
path = "src/lib.rs"

[[bin]]
name = "convx"
path = "src/main.rs"

[dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"

# Error handling
thiserror = "1"
anyhow = "1"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"

# CLI
clap = { version = "4", features = ["derive"] }
indicatif = "0.17"
console = "0.15"

# File handling
walkdir = "2"
glob = "0.3"
tempfile = "3"

# Utilities
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
bytesize = { version = "1", features = ["serde"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[dev-dependencies]
assert_cmd = "2"
predicates = "3"
assert_fs = "1"
```

### 1.3 Verify FFmpeg and libvips are available

```rust
// src/utils/deps.rs

use std::process::Command;
use crate::ConvxError;

pub fn check_ffmpeg() -> Result<String, ConvxError> {
    let output = Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map_err(|_| ConvxError::FfmpegNotFound)?;
    
    let version = String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .unwrap_or("unknown")
        .to_string();
    
    Ok(version)
}

pub fn check_vips() -> Result<String, ConvxError> {
    let output = Command::new("vips")
        .arg("--version")
        .output()
        .map_err(|_| ConvxError::DependencyError {
            name: "libvips".into(),
            reason: "vips command not found".into(),
        })?;
    
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
```

### Checkpoint 1
```bash
cargo build
cargo test test_dependencies
```

---

## Phase 2: Core Types

### 2.1 Create Format enum

**File:** `src/types/format.rs`

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

    // extension() and category() follow the same pattern.
    // See convx-core/src/types/format.rs for the full 54-format implementation.
    // Categories: Image, Video, Audio, Document, Data, Ebook

    pub fn detect(path: &Path) -> Option<Self> {
        path.extension()
            .and_then(|e| e.to_str())
            .and_then(Self::from_extension)
    }
}
```

### 2.2 Create ConversionOptions

**File:** `src/types/options.rs`

Implement all option structs from the spec:
- `ConversionOptions`
- `ImageOptions`
- `VideoOptions`
- `AudioOptions`
- `DocumentOptions`

### 2.3 Create Result types

**File:** `src/types/result.rs`

Implement:
- `ConversionResult`
- `ConversionStatus`
- `BatchResult`
- `BatchStatus`

### 2.4 Create Error types

**File:** `src/types/error.rs`

Implement `ConvxError` enum with all variants from spec.

### 2.5 Wire up module exports

**File:** `src/types/mod.rs`

```rust
mod format;
mod options;
mod result;
mod error;

pub use format::{Format, FormatCategory};
pub use options::*;
pub use result::*;
pub use error::ConvxError;
```

**File:** `src/lib.rs`

```rust
pub mod types;
pub mod converters;
pub mod config;
pub mod utils;

pub use types::*;
```

### Checkpoint 2
```bash
cargo build
cargo test test_format_detection
cargo test test_format_categories
```

---

## Phase 3: Converters

### 3.1 Create Converter trait

**File:** `src/converters/mod.rs`

```rust
use crate::{Format, ConversionOptions, ConversionResult, ConvxError};
use std::path::Path;

pub trait Converter: Send + Sync {
    /// Formats this converter can handle as input
    fn supported_inputs(&self) -> &[Format];
    
    /// Formats this converter can produce as output
    fn supported_outputs(&self) -> &[Format];
    
    /// Check if this converter handles a specific conversion
    fn can_convert(&self, from: Format, to: Format) -> bool;
    
    /// Perform the conversion
    fn convert(
        &self,
        input: &Path,
        output: &Path,
        options: &ConversionOptions,
    ) -> Result<ConversionResult, ConvxError>;
}
```

### 3.2 Implement ImageConverter

**File:** `src/converters/image.rs`

Use `vips` CLI for now (simpler than FFI bindings):

```rust
use std::process::Command;

pub struct ImageConverter;

impl ImageConverter {
    fn build_vips_command(
        &self,
        input: &Path,
        output: &Path,
        options: &ImageOptions,
    ) -> Command {
        let mut cmd = Command::new("vips");
        
        // Basic conversion
        cmd.arg("copy");
        cmd.arg(input);
        cmd.arg(output);
        
        // Quality (for lossy formats)
        if let Some(q) = options.quality {
            cmd.arg(format!("[Q={}]", q));
        }
        
        cmd
    }
}

impl Converter for ImageConverter {
    fn convert(
        &self,
        input: &Path,
        output: &Path,
        options: &ConversionOptions,
    ) -> Result<ConversionResult, ConvxError> {
        let start = std::time::Instant::now();
        let input_size = std::fs::metadata(input)?.len();
        
        let mut cmd = self.build_vips_command(input, output, &options.image);
        let status = cmd.status()?;
        
        if !status.success() {
            return Err(ConvxError::ConversionFailed {
                reason: "vips command failed".into(),
            });
        }
        
        let output_size = std::fs::metadata(output)?.len();
        
        Ok(ConversionResult {
            id: uuid::Uuid::new_v4(),
            status: ConversionStatus::Completed,
            input_path: input.to_path_buf(),
            output_path: Some(output.to_path_buf()),
            input_format: Format::detect(input).unwrap(),
            output_format: Format::detect(output).unwrap(),
            input_size,
            output_size: Some(output_size),
            space_saved: Some(input_size as i64 - output_size as i64),
            duration_ms: start.elapsed().as_millis() as u64,
            error: None,
            warnings: vec![],
            timestamp: chrono::Utc::now(),
        })
    }
}
```

### 3.3 Implement VideoConverter

**File:** `src/converters/video.rs`

Use FFmpeg CLI:

```rust
pub struct VideoConverter;

impl VideoConverter {
    fn build_ffmpeg_args(
        &self,
        input: &Path,
        output: &Path,
        options: &VideoOptions,
    ) -> Vec<String> {
        let mut args = vec![
            "-i".to_string(),
            input.to_string_lossy().to_string(),
            "-y".to_string(), // Overwrite
        ];
        
        // Video codec
        if let Some(codec) = &options.codec {
            args.push("-c:v".to_string());
            args.push(match codec {
                VideoCodec::H264 => "libx264",
                VideoCodec::H265 => "libx265",
                VideoCodec::Vp9 => "libvpx-vp9",
                VideoCodec::Av1 => "libaom-av1",
                _ => "copy",
            }.to_string());
        }
        
        // CRF quality
        if let Some(crf) = options.crf {
            args.push("-crf".to_string());
            args.push(crf.to_string());
        }
        
        // Resolution
        if let (Some(w), Some(h)) = (options.width, options.height) {
            args.push("-vf".to_string());
            args.push(format!("scale={}:{}", w, h));
        }
        
        // Output
        args.push(output.to_string_lossy().to_string());
        
        args
    }
}
```

### 3.4 Implement AudioConverter

**File:** `src/converters/audio.rs`

Similar pattern using FFmpeg.

### 3.5 Implement DocumentConverter

**File:** `src/converters/document.rs`

Uses Pandoc CLI for document conversions (PDF, DOCX, etc.) and weasyprint as PDF engine.

### 3.6 Implement DataConverter

**File:** `src/converters/data.rs`

Handles all data format conversions:
- **Pure Rust** (no deps): CSV↔JSON, JSON↔YAML, XML↔JSON, CSV↔XLSX, TSV↔CSV, JSONL↔JSON/CSV
- **Cross-category**: Data → HTML tables, Data → Markdown tables, Data → PDF (via Pandoc+weasyprint)
- **Python-backed ML formats**: Parquet↔CSV/JSON (pyarrow), Arrow↔CSV/JSON (pyarrow), SQLite→CSV/JSON (stdlib), NPY/NPZ→CSV (numpy), HDF5→CSV/JSON (h5py)

### 3.7 Implement EbookConverter

**File:** `src/converters/ebook.rs`

Uses ebook-convert CLI (Calibre) for MOBI↔EPUB conversions.

### 3.8 Create ConverterRegistry

**File:** `src/converters/registry.rs`

```rust
use crate::Format;
use super::Converter;

pub struct ConverterRegistry {
    image: ImageConverter,
    video: VideoConverter,
    audio: AudioConverter,
}

impl ConverterRegistry {
    pub fn new() -> Self {
        Self {
            image: ImageConverter,
            video: VideoConverter,
            audio: AudioConverter,
        }
    }
    
    pub fn get_converter(&self, from: Format, to: Format) -> Option<&dyn Converter> {
        // Route to appropriate converter based on format categories
        match (from.category(), to.category()) {
            (FormatCategory::Image, FormatCategory::Image) => Some(&self.image),
            (FormatCategory::Video, FormatCategory::Video) => Some(&self.video),
            (FormatCategory::Video, FormatCategory::Image) if to == Format::Gif => Some(&self.video),
            (FormatCategory::Audio, FormatCategory::Audio) => Some(&self.audio),
            _ => None,
        }
    }
}
```

### Checkpoint 3
```bash
cargo test test_image_conversion
cargo test test_video_conversion
cargo test test_audio_conversion
```

---

## Phase 4: Engine

### 4.1 Create ConvxEngine

**File:** `src/engine.rs`

```rust
use crate::{
    Format, ConversionOptions, ConversionResult, ConvxError,
    converters::ConverterRegistry,
};
use std::path::Path;

pub struct ConvxEngine {
    registry: ConverterRegistry,
}

impl ConvxEngine {
    pub fn new() -> Result<Self, ConvxError> {
        // Check dependencies
        crate::utils::deps::check_ffmpeg()?;
        
        Ok(Self {
            registry: ConverterRegistry::new(),
        })
    }
    
    pub fn convert(
        &self,
        input: &Path,
        output: &Path,
        options: ConversionOptions,
    ) -> Result<ConversionResult, ConvxError> {
        // Detect formats
        let input_format = Format::detect(input)
            .ok_or_else(|| ConvxError::FormatDetectionFailed {
                path: input.to_path_buf(),
            })?;
        
        let output_format = options.output_format;
        
        // Get converter
        let converter = self.registry
            .get_converter(input_format, output_format)
            .ok_or_else(|| ConvxError::UnsupportedConversion {
                from: input_format,
                to: output_format,
            })?;
        
        // Run conversion
        converter.convert(input, output, &options)
    }
    
    pub fn can_convert(&self, from: Format, to: Format) -> bool {
        self.registry.get_converter(from, to).is_some()
    }
}
```

### Checkpoint 4
```bash
cargo test test_engine_convert
cargo test test_engine_unsupported
```

---

## Phase 5: CLI

### 5.1 Create CLI structure

**File:** `src/main.rs`

```rust
use clap::{Parser, Subcommand};
use convx::{ConvxEngine, Format, ConversionOptions};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "convx")]
#[command(about = "Local-first file conversion")]
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
        
        /// Output file (optional)
        output: Option<PathBuf>,
        
        /// Output format
        #[arg(short, long)]
        to: Option<String>,
        
        /// Quality (0-100)
        #[arg(short, long)]
        quality: Option<u8>,
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
        Commands::Convert { input, output, to, quality } => {
            let output = output.unwrap_or_else(|| {
                let mut out = input.clone();
                out.set_extension(to.as_deref().unwrap_or("webp"));
                out
            });
            
            let output_format = to
                .and_then(|t| Format::from_extension(&t))
                .or_else(|| Format::detect(&output))
                .expect("Could not determine output format");
            
            let options = ConversionOptions {
                output_format,
                quality,
                ..Default::default()
            };
            
            let result = engine.convert(&input, &output, options)?;
            
            println!("✓ Converted: {} → {}", input.display(), output.display());
            println!("  Size: {} → {} ({:+.1}%)",
                humansize(result.input_size),
                humansize(result.output_size.unwrap_or(0)),
                savings_percent(&result),
            );
        }
        
        Commands::Formats => {
            println!("Supported formats:");
            println!("  Images: png, jpg, webp, gif, bmp, tiff, heic, avif");
            println!("  Video:  mp4, mov, webm, avi, mkv, gif");
            println!("  Audio:  mp3, wav, flac, m4a, aac, ogg, opus");
        }
        
        Commands::Version => {
            println!("convx {}", env!("CARGO_PKG_VERSION"));
        }
    }
    
    Ok(())
}
```

### Checkpoint 5
```bash
cargo build --release
./target/release/convx --help
./target/release/convx convert test.png --to webp
```

---

## Phase 6: Integration Tests

See `convx-test-suite.md` for full test specifications.

### Checkpoint 6
```bash
cargo test --test integration
```

---

## Definition of Done

The engine is complete when:

- [ ] `cargo build --release` succeeds
- [ ] All unit tests pass: `cargo test`
- [ ] All integration tests pass: `cargo test --test integration`
- [ ] CLI works: `convx convert image.png --to webp`
- [ ] These conversions work:
  - [ ] PNG → WebP
  - [ ] PNG → JPG
  - [ ] JPG → PNG
  - [ ] MP4 → WebM
  - [ ] MP4 → GIF
  - [ ] MP3 → WAV
  - [ ] WAV → MP3
  - [ ] CSV → JSON
  - [ ] JSON → CSV
  - [ ] TSV → CSV
  - [ ] JSONL → JSON
  - [ ] CSV → HTML (styled table)
  - [ ] CSV → Markdown (GFM table)
  - [ ] Parquet → CSV (requires pyarrow)

---

## Notes for Coding Agents

1. **Start with the happy path.** Get one conversion working end-to-end before handling edge cases.

2. **Use CLI tools first.** Don't bother with FFI bindings for FFmpeg/libvips. Shell out to the CLI tools. It's simpler and works.

3. **Test with real files.** The test suite includes URLs to download sample files. Use them.

4. **Error messages matter.** When FFmpeg fails, capture stderr and include it in the error.

5. **Don't over-engineer.** Skip plugins, presets, watch mode, and batch processing for MVP. Just get single-file conversion working.

---

## Appendix: Minimum Viable Commands

### Image conversion (vips)
```bash
vips copy input.png output.webp[Q=80]
vips thumbnail input.jpg output.jpg 800  # resize to 800px wide
```

### Video conversion (ffmpeg)
```bash
ffmpeg -i input.mp4 -c:v libx264 -crf 23 output.mp4
ffmpeg -i input.mp4 -vf "scale=480:-1,fps=15" output.gif
```

### Audio conversion (ffmpeg)
```bash
ffmpeg -i input.mp3 -c:a libopus -b:a 128k output.opus
ffmpeg -i input.wav -c:a libmp3lame -q:a 2 output.mp3
```

These are the commands your converters need to generate.
