# convx Core Engine Specification

**Version:** 0.1.0  
**Last Updated:** February 16, 2026  
**Status:** Draft

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Core Types](#core-types)
4. [Conversion Engine API](#conversion-engine-api)
5. [Supported Formats](#supported-formats)
6. [Configuration & Options](#configuration--options)
7. [Presets](#presets)
8. [Batch Processing](#batch-processing)
9. [Progress Reporting](#progress-reporting)
10. [Error Handling](#error-handling)
11. [File I/O](#file-io)
12. [Watch Mode](#watch-mode)
13. [Plugin System](#plugin-system)
14. [CLI Interface](#cli-interface)
15. [FFI Bindings](#ffi-bindings)
16. [Dependencies](#dependencies)
17. [Security Considerations](#security-considerations)
18. [Performance Requirements](#performance-requirements)

---

## 1. Overview

### Purpose

convx is a local-first, privacy-focused file conversion engine that currently implements **246** conversion paths across image/video/audio categories. The core engine is written in Rust and provides a unified API consumed by:

- CLI tool
- Desktop application (Tauri)
- Web application (WASM)
- Mobile application (native bindings via Expo)

### Design Principles

1. **Local-first:** All processing happens on the user's machine by default
2. **Zero uploads:** Files never leave the device unless explicitly requested
3. **Single engine:** One codebase powers all interfaces
4. **Extensible:** Plugin system for community-contributed converters
5. **Developer-friendly:** First-class CLI with scriptable interface
6. **User-friendly:** Sensible defaults, advanced options when needed

---

## 2. Architecture

### High-Level Diagram

The current implementation routes by format category:

- **Image → Image** via `libvips`
- **Video → Video** via `FFmpeg`
- **Video → GIF** via `FFmpeg`
- **Audio → Audio** via `FFmpeg`

The same core engine is consumed by CLI and Tauri today, with WASM/FFI expansion planned.

### Module Structure

```
convx-core/
├── Cargo.toml
├── src/
│   ├── lib.rs                 # Public API exports
│   ├── engine.rs              # Main conversion engine
│   ├── types/
│   │   ├── mod.rs
│   │   ├── format.rs          # Format enum and detection
│   │   ├── options.rs         # Conversion options
│   │   ├── preset.rs          # Preset definitions
│   │   ├── result.rs          # Conversion results
│   │   └── error.rs           # Error types
│   ├── converters/
│   │   ├── mod.rs             # Converter trait and registry
│   │   ├── image.rs           # Image conversions (libvips)
│   │   ├── video.rs           # Video conversions (FFmpeg)
│   │   ├── audio.rs           # Audio conversions (FFmpeg)
│   │   ├── document.rs        # Document conversions (Pandoc)
│   │   └── archive.rs         # Archive conversions
│   ├── config/
│   │   ├── mod.rs
│   │   ├── manager.rs         # Config file handling
│   │   └── defaults.rs        # Default settings
│   ├── batch/
│   │   ├── mod.rs
│   │   ├── processor.rs       # Batch processing logic
│   │   └── queue.rs           # Job queue management
│   ├── watch/
│   │   ├── mod.rs
│   │   └── watcher.rs         # File system watcher
│   ├── progress/
│   │   ├── mod.rs
│   │   ├── reporter.rs        # Progress reporting
│   │   └── callback.rs        # Callback definitions
│   ├── plugins/
│   │   ├── mod.rs
│   │   ├── loader.rs          # Plugin loading
│   │   └── interface.rs       # Plugin interface trait
│   └── utils/
│       ├── mod.rs
│       ├── detection.rs       # Format detection
│       ├── validation.rs      # Input validation
│       └── temp.rs            # Temp file management
├── bindings/
│   ├── wasm/                  # WASM bindings
│   ├── ffi/                   # C FFI for mobile
│   └── napi/                  # Node.js bindings (optional)
└── tests/
    ├── integration/
    └── fixtures/
```

---

## 3. Core Types

### Format Enum

```rust
/// Represents all supported file formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    // Images
    Png,
    Jpg,
    Jpeg,
    WebP,
    Gif,
    Bmp,
    Tiff,
    Ico,
    Svg,
    Heic,
    Heif,
    Avif,
    Raw,
    Psd,
    
    // Video
    Mp4,
    Mov,
    Webm,
    Avi,
    Mkv,
    Wmv,
    Flv,
    M4v,
    Mpeg,
    Mpg,
    ThreeGp,
    
    // Audio
    Mp3,
    Wav,
    Flac,
    M4a,
    Aac,
    Ogg,
    Wma,
    Aiff,
    Opus,
    
    // Documents
    Pdf,
    Docx,
    Doc,
    Txt,
    Md,
    Html,
    Rtf,
    Odt,
    Epub,
    Mobi,
    Latex,
    
    // Data
    Csv,
    Json,
    Xml,
    Yaml,
    Toml,
    Xlsx,
    Xls,
    
    // Archives
    Zip,
    TarGz,
    SevenZ,
    Rar,
    
    // Unknown/Custom
    Unknown(String),
}

impl Format {
    /// Detect format from file extension
    pub fn from_extension(ext: &str) -> Self;
    
    /// Detect format from file magic bytes
    pub fn from_magic_bytes(bytes: &[u8]) -> Option<Self>;
    
    /// Detect format from file (tries magic bytes first, then extension)
    pub fn detect(path: &Path) -> Result<Self, ConvxError>;
    
    /// Get the standard file extension for this format
    pub fn extension(&self) -> &str;
    
    /// Get the MIME type for this format
    pub fn mime_type(&self) -> &str;
    
    /// Get the category of this format
    pub fn category(&self) -> FormatCategory;
    
    /// Check if conversion to target format is supported
    pub fn can_convert_to(&self, target: Format) -> bool;
    
    /// Get all formats this format can convert to
    pub fn convertible_targets(&self) -> Vec<Format>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatCategory {
    Image,
    Video,
    Audio,
    Document,
    Data,
    Archive,
    Unknown,
}
```

### Conversion Options

```rust
/// Main options struct for all conversions
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConversionOptions {
    /// Output format (required)
    pub output_format: Format,
    
    /// Quality setting (0-100, format-dependent)
    pub quality: Option<u8>,
    
    /// Image-specific options
    pub image: Option<ImageOptions>,
    
    /// Video-specific options
    pub video: Option<VideoOptions>,
    
    /// Audio-specific options
    pub audio: Option<AudioOptions>,
    
    /// Document-specific options
    pub document: Option<DocumentOptions>,
    
    /// Use a named preset (overrides other options)
    pub preset: Option<String>,
    
    /// Target file size constraint (engine will adjust quality)
    pub max_file_size: Option<ByteSize>,
    
    /// Preserve metadata (EXIF, etc.)
    pub preserve_metadata: bool,
    
    /// Overwrite existing files
    pub overwrite: bool,
    
    /// Output filename template
    pub output_template: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImageOptions {
    /// Resize width (maintains aspect ratio if height not set)
    pub width: Option<u32>,
    
    /// Resize height (maintains aspect ratio if width not set)
    pub height: Option<u32>,
    
    /// Resize mode
    pub resize_mode: ResizeMode,
    
    /// Rotation in degrees (0, 90, 180, 270)
    pub rotate: Option<u16>,
    
    /// Flip horizontally
    pub flip_h: bool,
    
    /// Flip vertically
    pub flip_v: bool,
    
    /// Strip metadata
    pub strip_metadata: bool,
    
    /// Background color for transparent images (hex)
    pub background: Option<String>,
    
    /// Enable progressive encoding (JPEG/PNG)
    pub progressive: bool,
    
    /// Compression level (PNG: 0-9)
    pub compression: Option<u8>,
    
    /// Enable lossless mode (WebP, AVIF)
    pub lossless: bool,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub enum ResizeMode {
    /// Fit within dimensions, maintain aspect ratio
    #[default]
    Fit,
    /// Fill dimensions, crop if needed
    Fill,
    /// Exact dimensions, may distort
    Exact,
    /// Scale by percentage
    Scale(f32),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VideoOptions {
    /// Video codec
    pub codec: Option<VideoCodec>,
    
    /// Video bitrate (e.g., "2M", "5000k")
    pub bitrate: Option<String>,
    
    /// Frame rate
    pub fps: Option<f32>,
    
    /// Resolution width
    pub width: Option<u32>,
    
    /// Resolution height
    pub height: Option<u32>,
    
    /// Start time for trimming (seconds or HH:MM:SS)
    pub start_time: Option<String>,
    
    /// End time or duration for trimming
    pub end_time: Option<String>,
    
    /// Duration for trimming
    pub duration: Option<String>,
    
    /// Remove audio track
    pub no_audio: bool,
    
    /// Audio options for video files
    pub audio: Option<AudioOptions>,
    
    /// CRF quality (0-51, lower = better)
    pub crf: Option<u8>,
    
    /// Encoding speed preset
    pub speed_preset: Option<SpeedPreset>,
    
    /// Two-pass encoding
    pub two_pass: bool,
    
    /// Hardware acceleration
    pub hw_accel: Option<HwAccel>,
    
    // GIF-specific
    /// GIF loop count (0 = infinite)
    pub gif_loop: Option<u32>,
    
    /// GIF optimization level
    pub gif_optimize: bool,
    
    /// GIF dither mode
    pub gif_dither: Option<GifDither>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum VideoCodec {
    H264,
    H265,
    Vp8,
    Vp9,
    Av1,
    ProRes,
    Gif,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SpeedPreset {
    Ultrafast,
    Superfast,
    Veryfast,
    Faster,
    Fast,
    Medium,
    Slow,
    Slower,
    Veryslow,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum HwAccel {
    Auto,
    Cuda,
    Videotoolbox,
    Vaapi,
    Qsv,
    None,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum GifDither {
    None,
    Bayer,
    FloydSteinberg,
    Sierra,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AudioOptions {
    /// Audio codec
    pub codec: Option<AudioCodec>,
    
    /// Audio bitrate (e.g., "128k", "320k")
    pub bitrate: Option<String>,
    
    /// Sample rate (e.g., 44100, 48000)
    pub sample_rate: Option<u32>,
    
    /// Number of channels (1 = mono, 2 = stereo)
    pub channels: Option<u8>,
    
    /// Volume adjustment (1.0 = no change, 2.0 = double)
    pub volume: Option<f32>,
    
    /// Normalize audio levels
    pub normalize: bool,
    
    /// Remove silence from start/end
    pub trim_silence: bool,
    
    /// Start time for trimming
    pub start_time: Option<String>,
    
    /// End time for trimming
    pub end_time: Option<String>,
    
    /// Fade in duration (seconds)
    pub fade_in: Option<f32>,
    
    /// Fade out duration (seconds)
    pub fade_out: Option<f32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AudioCodec {
    Aac,
    Mp3,
    Opus,
    Vorbis,
    Flac,
    Pcm,
    Alac,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DocumentOptions {
    /// Page size for PDF output
    pub page_size: Option<PageSize>,
    
    /// Margins (in points)
    pub margins: Option<Margins>,
    
    /// Font family
    pub font: Option<String>,
    
    /// Font size (points)
    pub font_size: Option<f32>,
    
    /// Line height
    pub line_height: Option<f32>,
    
    /// Table of contents
    pub toc: bool,
    
    /// Page numbers
    pub page_numbers: bool,
    
    /// Syntax highlighting for code (markdown/html)
    pub syntax_highlight: bool,
    
    /// CSS file for HTML output
    pub css: Option<PathBuf>,
    
    /// PDF engine (for Pandoc)
    pub pdf_engine: Option<PdfEngine>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PageSize {
    Letter,
    A4,
    A5,
    Legal,
    Custom { width: f32, height: f32 },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Margins {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PdfEngine {
    Pdflatex,
    Xelatex,
    Lualatex,
    Wkhtmltopdf,
    Weasyprint,
}
```

### Conversion Result

```rust
/// Result of a single conversion operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionResult {
    /// Unique ID for this conversion
    pub id: Uuid,
    
    /// Status of the conversion
    pub status: ConversionStatus,
    
    /// Input file path
    pub input_path: PathBuf,
    
    /// Output file path (if successful)
    pub output_path: Option<PathBuf>,
    
    /// Input format detected
    pub input_format: Format,
    
    /// Output format requested
    pub output_format: Format,
    
    /// Input file size in bytes
    pub input_size: u64,
    
    /// Output file size in bytes (if successful)
    pub output_size: Option<u64>,
    
    /// Space saved (positive = smaller, negative = larger)
    pub space_saved: Option<i64>,
    
    /// Processing time in milliseconds
    pub duration_ms: u64,
    
    /// Error message (if failed)
    pub error: Option<String>,
    
    /// Warnings generated during conversion
    pub warnings: Vec<String>,
    
    /// Timestamp of conversion
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConversionStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Cancelled,
    Skipped,
}

/// Result of a batch conversion operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult {
    /// Unique ID for this batch
    pub id: Uuid,
    
    /// Overall status
    pub status: BatchStatus,
    
    /// Individual conversion results
    pub results: Vec<ConversionResult>,
    
    /// Total files processed
    pub total_files: usize,
    
    /// Successfully converted
    pub successful: usize,
    
    /// Failed conversions
    pub failed: usize,
    
    /// Skipped files
    pub skipped: usize,
    
    /// Total input size
    pub total_input_size: u64,
    
    /// Total output size
    pub total_output_size: u64,
    
    /// Total space saved
    pub total_space_saved: i64,
    
    /// Total processing time in milliseconds
    pub total_duration_ms: u64,
    
    /// Timestamp started
    pub started_at: DateTime<Utc>,
    
    /// Timestamp completed
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BatchStatus {
    Pending,
    Processing,
    Completed,
    CompletedWithErrors,
    Failed,
    Cancelled,
}
```

---

## 4. Conversion Engine API

### Core Engine Struct

```rust
/// Main conversion engine
pub struct ConvxEngine {
    /// Configuration
    config: Config,
    
    /// Format registry
    registry: FormatRegistry,
    
    /// Plugin manager
    plugins: PluginManager,
    
    /// Progress callback
    progress_callback: Option<Box<dyn Fn(ProgressEvent) + Send + Sync>>,
}

impl ConvxEngine {
    /// Create a new engine with default configuration
    pub fn new() -> Result<Self, ConvxError>;
    
    /// Create engine with custom configuration
    pub fn with_config(config: Config) -> Result<Self, ConvxError>;
    
    /// Load configuration from file
    pub fn from_config_file(path: &Path) -> Result<Self, ConvxError>;
    
    // ============ Single File Conversion ============
    
    /// Convert a single file
    pub fn convert(
        &self,
        input: &Path,
        output: &Path,
        options: ConversionOptions,
    ) -> Result<ConversionResult, ConvxError>;
    
    /// Convert a single file asynchronously
    pub async fn convert_async(
        &self,
        input: &Path,
        output: &Path,
        options: ConversionOptions,
    ) -> Result<ConversionResult, ConvxError>;
    
    /// Convert file in memory (for WASM/streaming)
    pub fn convert_bytes(
        &self,
        input: &[u8],
        input_format: Format,
        options: ConversionOptions,
    ) -> Result<Vec<u8>, ConvxError>;
    
    // ============ Batch Conversion ============
    
    /// Convert multiple files
    pub fn convert_batch(
        &self,
        inputs: &[PathBuf],
        output_dir: &Path,
        options: ConversionOptions,
    ) -> Result<BatchResult, ConvxError>;
    
    /// Convert multiple files asynchronously with parallelism
    pub async fn convert_batch_async(
        &self,
        inputs: &[PathBuf],
        output_dir: &Path,
        options: ConversionOptions,
        parallelism: usize,
    ) -> Result<BatchResult, ConvxError>;
    
    /// Convert all files in a directory (optionally recursive)
    pub fn convert_directory(
        &self,
        input_dir: &Path,
        output_dir: &Path,
        options: ConversionOptions,
        recursive: bool,
        filter: Option<&[Format]>,
    ) -> Result<BatchResult, ConvxError>;
    
    // ============ Format Detection ============
    
    /// Detect format of a file
    pub fn detect_format(&self, path: &Path) -> Result<Format, ConvxError>;
    
    /// Detect format from bytes
    pub fn detect_format_bytes(&self, bytes: &[u8]) -> Option<Format>;
    
    /// Get file info without converting
    pub fn get_file_info(&self, path: &Path) -> Result<FileInfo, ConvxError>;
    
    // ============ Capability Queries ============
    
    /// Check if a conversion path is supported
    pub fn can_convert(&self, from: Format, to: Format) -> bool;
    
    /// Get all supported input formats
    pub fn supported_input_formats(&self) -> Vec<Format>;
    
    /// Get all supported output formats
    pub fn supported_output_formats(&self) -> Vec<Format>;
    
    /// Get all formats a given format can convert to
    pub fn get_conversion_targets(&self, from: Format) -> Vec<Format>;
    
    /// Get conversion path info (which converter will be used)
    pub fn get_conversion_info(&self, from: Format, to: Format) -> Option<ConversionInfo>;
    
    // ============ Presets ============
    
    /// Get all available presets
    pub fn get_presets(&self) -> Vec<PresetInfo>;
    
    /// Get a specific preset by name
    pub fn get_preset(&self, name: &str) -> Option<Preset>;
    
    /// Register a custom preset
    pub fn register_preset(&mut self, preset: Preset) -> Result<(), ConvxError>;
    
    // ============ Progress & Callbacks ============
    
    /// Set progress callback
    pub fn set_progress_callback<F>(&mut self, callback: F)
    where
        F: Fn(ProgressEvent) + Send + Sync + 'static;
    
    /// Remove progress callback
    pub fn clear_progress_callback(&mut self);
    
    // ============ Watch Mode ============
    
    /// Start watching a directory for new files
    pub fn watch(
        &self,
        input_dir: &Path,
        output_dir: &Path,
        options: ConversionOptions,
    ) -> Result<WatchHandle, ConvxError>;
    
    // ============ Plugins ============
    
    /// Load a plugin from file
    pub fn load_plugin(&mut self, path: &Path) -> Result<(), ConvxError>;
    
    /// List loaded plugins
    pub fn list_plugins(&self) -> Vec<PluginInfo>;
    
    /// Unload a plugin
    pub fn unload_plugin(&mut self, name: &str) -> Result<(), ConvxError>;
    
    // ============ Utilities ============
    
    /// Estimate output size (rough estimate based on format and options)
    pub fn estimate_output_size(
        &self,
        input: &Path,
        options: &ConversionOptions,
    ) -> Result<EstimatedSize, ConvxError>;
    
    /// Find optimal settings for target file size
    pub fn optimize_for_size(
        &self,
        input: &Path,
        target_size: ByteSize,
        output_format: Format,
    ) -> Result<ConversionOptions, ConvxError>;
    
    /// Validate options for a conversion
    pub fn validate_options(
        &self,
        from: Format,
        to: Format,
        options: &ConversionOptions,
    ) -> Result<(), ConvxError>;
    
    /// Cancel an ongoing operation
    pub fn cancel(&self, id: Uuid) -> Result<(), ConvxError>;
}
```

### File Info

```rust
/// Information about a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: PathBuf,
    pub format: Format,
    pub size: u64,
    pub created: Option<DateTime<Utc>>,
    pub modified: Option<DateTime<Utc>>,
    
    /// Format-specific metadata
    pub metadata: FormatMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FormatMetadata {
    Image(ImageMetadata),
    Video(VideoMetadata),
    Audio(AudioMetadata),
    Document(DocumentMetadata),
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageMetadata {
    pub width: u32,
    pub height: u32,
    pub color_space: String,
    pub bit_depth: u8,
    pub has_alpha: bool,
    pub dpi: Option<(f32, f32)>,
    pub exif: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoMetadata {
    pub width: u32,
    pub height: u32,
    pub duration_secs: f64,
    pub frame_rate: f32,
    pub video_codec: String,
    pub audio_codec: Option<String>,
    pub bitrate: u64,
    pub has_audio: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioMetadata {
    pub duration_secs: f64,
    pub sample_rate: u32,
    pub channels: u8,
    pub codec: String,
    pub bitrate: u64,
    pub tags: Option<AudioTags>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioTags {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub year: Option<u32>,
    pub track: Option<u32>,
    pub genre: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub page_count: Option<u32>,
    pub word_count: Option<u32>,
    pub title: Option<String>,
    pub author: Option<String>,
    pub created: Option<DateTime<Utc>>,
}
```

---

## 5. Supported Formats

### Current Implementation (v0.1.x)

The current engine implementation supports conversion across image, video, and audio categories.

- **Implemented directed conversion paths:** **246**
- **Convertible extensions:** **28**

#### Convertible Images (11)

png, jpg, webp, gif, bmp, tiff, ico, svg, heic, heif, avif

#### Convertible Video (8)

mp4, mov, webm, avi, mkv, wmv, flv, m4v

> Additionally, video → gif is supported.

#### Convertible Audio (9)

mp3, wav, flac, m4a, aac, ogg, wma, aiff, opus

### Notes

- The `Format` enum includes additional document variants for forward compatibility.
- Document/data conversion matrices in this spec represent roadmap scope, not current production behavior.

---

## 6. Configuration & Options

### Config File Format

Location: `~/.config/convx/config.toml` (Linux/Mac) or `%APPDATA%\convx\config.toml` (Windows)

```toml
# convx configuration file

[general]
# Default output directory (empty = same as input)
default_output_dir = ""

# Overwrite existing files without prompting
overwrite = false

# Preserve original file metadata
preserve_metadata = true

# Number of parallel conversions (0 = auto-detect)
parallelism = 0

# Temp directory (empty = system default)
temp_dir = ""

# Log level: error, warn, info, debug, trace
log_level = "info"

[defaults.image]
# Default image quality (0-100)
quality = 85

# Default resize mode: fit, fill, exact
resize_mode = "fit"

# Strip metadata by default
strip_metadata = false

# Enable progressive encoding
progressive = true

[defaults.video]
# Default video codec: h264, h265, vp9, av1
codec = "h264"

# Default CRF (0-51, lower = better quality)
crf = 23

# Default speed preset
speed_preset = "medium"

# Hardware acceleration: auto, none, cuda, videotoolbox, vaapi, qsv
hw_accel = "auto"

[defaults.audio]
# Default audio codec: aac, mp3, opus, flac
codec = "aac"

# Default bitrate
bitrate = "192k"

# Default sample rate
sample_rate = 44100

# Normalize audio by default
normalize = false

[defaults.document]
# Default page size: letter, a4, a5, legal
page_size = "letter"

# Default font
font = "Arial"

# Default font size
font_size = 12

[presets]
# Custom presets defined here (see Presets section)

[plugins]
# Plugin directory
plugin_dir = "~/.config/convx/plugins"

# Enabled plugins
enabled = ["webp-optimizer", "heic-support"]
```

### Environment Variables

```bash
# Override config file location
CONVX_CONFIG=/path/to/config.toml

# Override default output directory
CONVX_OUTPUT_DIR=/path/to/output

# Override temp directory
CONVX_TEMP_DIR=/path/to/temp

# Set log level
CONVX_LOG_LEVEL=debug

# Disable hardware acceleration
CONVX_NO_HW_ACCEL=1

# Set parallelism
CONVX_PARALLELISM=4
```

---

## 7. Presets

### Built-in Presets

```rust
/// Preset definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    /// Unique name
    pub name: String,
    
    /// Display name
    pub display_name: String,
    
    /// Description
    pub description: String,
    
    /// Applicable input formats (empty = all)
    pub input_formats: Vec<Format>,
    
    /// Output format
    pub output_format: Format,
    
    /// Conversion options
    pub options: ConversionOptions,
    
    /// Target file size constraint (optional)
    pub max_file_size: Option<ByteSize>,
    
    /// Category for organization
    pub category: PresetCategory,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PresetCategory {
    Web,
    Social,
    Email,
    Archive,
    Print,
    Mobile,
    Custom,
}
```

### Default Presets

```toml
# Web-optimized presets
[presets.web-image]
display_name = "Web Optimized"
description = "Optimized for web use, good balance of quality and size"
output_format = "webp"
category = "web"
[presets.web-image.options]
quality = 80
[presets.web-image.options.image]
progressive = true
strip_metadata = true

[presets.web-image-small]
display_name = "Web Small"
description = "Smaller file size for faster loading"
output_format = "webp"
max_file_size = "100KB"
category = "web"
[presets.web-image-small.options]
quality = 70
[presets.web-image-small.options.image]
strip_metadata = true
width = 1200

# Social media presets
[presets.twitter-image]
display_name = "Twitter/X Image"
description = "Optimized for Twitter/X posts"
input_formats = ["png", "jpg", "webp", "gif"]
output_format = "jpg"
max_file_size = "5MB"
category = "social"
[presets.twitter-image.options]
quality = 85
[presets.twitter-image.options.image]
width = 1200
height = 675
resize_mode = "fill"

[presets.twitter-gif]
display_name = "Twitter/X GIF"
description = "Optimized GIF for Twitter/X"
input_formats = ["mp4", "mov", "webm", "gif"]
output_format = "gif"
max_file_size = "15MB"
category = "social"
[presets.twitter-gif.options.video]
width = 480
fps = 15
gif_optimize = true
duration = "60"

[presets.discord-video]
display_name = "Discord Video"
description = "Video under 8MB for Discord free tier"
input_formats = ["mp4", "mov", "webm", "avi", "mkv"]
output_format = "mp4"
max_file_size = "8MB"
category = "social"
[presets.discord-video.options.video]
codec = "h264"
crf = 28
speed_preset = "fast"

[presets.discord-nitro]
display_name = "Discord Nitro Video"
description = "Video under 50MB for Discord Nitro"
input_formats = ["mp4", "mov", "webm", "avi", "mkv"]
output_format = "mp4"
max_file_size = "50MB"
category = "social"
[presets.discord-nitro.options.video]
codec = "h264"
crf = 23

[presets.instagram-story]
display_name = "Instagram Story"
description = "Optimized for Instagram Stories"
input_formats = ["mp4", "mov"]
output_format = "mp4"
max_file_size = "4GB"
category = "social"
[presets.instagram-story.options.video]
width = 1080
height = 1920
fps = 30
codec = "h264"
duration = "60"

# Email presets
[presets.email-friendly]
display_name = "Email Friendly"
description = "Small enough for email attachments"
output_format = "jpg"
max_file_size = "1MB"
category = "email"
[presets.email-friendly.options]
quality = 75
[presets.email-friendly.options.image]
width = 1200
strip_metadata = true

[presets.email-attachment]
display_name = "Email PDF"
description = "Compressed PDF for email"
input_formats = ["pdf", "docx", "doc"]
output_format = "pdf"
max_file_size = "10MB"
category = "email"

# Archive/backup presets
[presets.archive-lossless]
display_name = "Archive (Lossless)"
description = "Lossless compression for archival"
output_format = "png"
category = "archive"
[presets.archive-lossless.options.image]
compression = 9
lossless = true

[presets.archive-audio]
display_name = "Archive Audio (FLAC)"
description = "Lossless audio archival"
input_formats = ["mp3", "wav", "m4a", "aac", "ogg"]
output_format = "flac"
category = "archive"

# Mobile presets
[presets.mobile-video]
display_name = "Mobile Video"
description = "Optimized for mobile playback"
output_format = "mp4"
category = "mobile"
[presets.mobile-video.options.video]
width = 720
codec = "h264"
crf = 25

# Conversion shortcuts
[presets.heic-to-jpg]
display_name = "HEIC → JPG"
description = "Convert iPhone photos to JPG"
input_formats = ["heic", "heif"]
output_format = "jpg"
category = "custom"
[presets.heic-to-jpg.options]
quality = 90

[presets.video-to-gif]
display_name = "Video → GIF"
description = "Create GIF from video"
input_formats = ["mp4", "mov", "webm", "avi"]
output_format = "gif"
max_file_size = "10MB"
category = "custom"
[presets.video-to-gif.options.video]
width = 480
fps = 12
gif_optimize = true

[presets.extract-audio]
display_name = "Extract Audio"
description = "Extract audio track from video"
input_formats = ["mp4", "mov", "webm", "avi", "mkv"]
output_format = "mp3"
category = "custom"
[presets.extract-audio.options.audio]
bitrate = "192k"
```

---

## 8. Batch Processing

### Batch Processor

```rust
/// Batch processing configuration
#[derive(Debug, Clone, Default)]
pub struct BatchConfig {
    /// Number of parallel conversions
    pub parallelism: usize,
    
    /// Continue on error (don't stop batch)
    pub continue_on_error: bool,
    
    /// Skip existing files
    pub skip_existing: bool,
    
    /// Flatten output directory structure
    pub flatten: bool,
    
    /// Output filename template
    /// Placeholders: {name}, {ext}, {index}, {date}, {format}
    pub output_template: Option<String>,
    
    /// Filter input files by format
    pub format_filter: Option<Vec<Format>>,
    
    /// Filter by file size (min, max)
    pub size_filter: Option<(Option<ByteSize>, Option<ByteSize>)>,
    
    /// Recursive directory scanning
    pub recursive: bool,
    
    /// Dry run (don't actually convert)
    pub dry_run: bool,
}

/// Batch job for tracking
#[derive(Debug)]
pub struct BatchJob {
    pub id: Uuid,
    pub config: BatchConfig,
    pub options: ConversionOptions,
    pub inputs: Vec<PathBuf>,
    pub output_dir: PathBuf,
    pub status: BatchStatus,
    pub progress: BatchProgress,
}

#[derive(Debug, Clone, Default)]
pub struct BatchProgress {
    pub total: usize,
    pub completed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub current_file: Option<String>,
    pub bytes_processed: u64,
    pub bytes_total: u64,
}

impl ConvxEngine {
    /// Create a new batch job
    pub fn create_batch(
        &self,
        inputs: Vec<PathBuf>,
        output_dir: PathBuf,
        options: ConversionOptions,
        config: BatchConfig,
    ) -> Result<BatchJob, ConvxError>;
    
    /// Start a batch job
    pub async fn run_batch(&self, job: &mut BatchJob) -> Result<BatchResult, ConvxError>;
    
    /// Pause a batch job
    pub fn pause_batch(&self, job_id: Uuid) -> Result<(), ConvxError>;
    
    /// Resume a batch job
    pub fn resume_batch(&self, job_id: Uuid) -> Result<(), ConvxError>;
    
    /// Cancel a batch job
    pub fn cancel_batch(&self, job_id: Uuid) -> Result<(), ConvxError>;
    
    /// Get batch job status
    pub fn get_batch_status(&self, job_id: Uuid) -> Option<BatchProgress>;
}
```

---

## 9. Progress Reporting

### Progress Events

```rust
/// Progress event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProgressEvent {
    /// Conversion started
    Started {
        id: Uuid,
        input: PathBuf,
        output: PathBuf,
        format_from: Format,
        format_to: Format,
    },
    
    /// Progress update
    Progress {
        id: Uuid,
        /// Progress percentage (0.0 - 1.0)
        percent: f32,
        /// Bytes processed so far
        bytes_processed: u64,
        /// Total bytes (if known)
        bytes_total: Option<u64>,
        /// Current operation description
        stage: String,
        /// Estimated time remaining (seconds)
        eta_seconds: Option<u32>,
    },
    
    /// Conversion completed successfully
    Completed {
        id: Uuid,
        result: ConversionResult,
    },
    
    /// Conversion failed
    Failed {
        id: Uuid,
        error: String,
    },
    
    /// Conversion cancelled
    Cancelled {
        id: Uuid,
    },
    
    /// Warning during conversion
    Warning {
        id: Uuid,
        message: String,
    },
    
    // Batch events
    BatchStarted {
        id: Uuid,
        total_files: usize,
    },
    
    BatchProgress {
        id: Uuid,
        completed: usize,
        failed: usize,
        total: usize,
    },
    
    BatchCompleted {
        id: Uuid,
        result: BatchResult,
    },
}

/// Progress callback trait
pub trait ProgressCallback: Send + Sync {
    fn on_progress(&self, event: ProgressEvent);
}

/// Simple closure-based callback
impl<F> ProgressCallback for F
where
    F: Fn(ProgressEvent) + Send + Sync,
{
    fn on_progress(&self, event: ProgressEvent) {
        self(event)
    }
}
```

---

## 10. Error Handling

### Error Types

```rust
/// Main error type for convx
#[derive(Debug, thiserror::Error)]
pub enum ConvxError {
    // File errors
    #[error("File not found: {path}")]
    FileNotFound { path: PathBuf },
    
    #[error("Cannot read file: {path}: {reason}")]
    FileReadError { path: PathBuf, reason: String },
    
    #[error("Cannot write file: {path}: {reason}")]
    FileWriteError { path: PathBuf, reason: String },
    
    #[error("File already exists: {path}")]
    FileExists { path: PathBuf },
    
    #[error("Permission denied: {path}")]
    PermissionDenied { path: PathBuf },
    
    // Format errors
    #[error("Unknown format: {format}")]
    UnknownFormat { format: String },
    
    #[error("Cannot detect format for: {path}")]
    FormatDetectionFailed { path: PathBuf },
    
    #[error("Unsupported conversion: {from} → {to}")]
    UnsupportedConversion { from: Format, to: Format },
    
    #[error("Corrupted or invalid file: {path}: {reason}")]
    InvalidFile { path: PathBuf, reason: String },
    
    // Conversion errors
    #[error("Conversion failed: {reason}")]
    ConversionFailed { reason: String },
    
    #[error("Conversion cancelled")]
    Cancelled,
    
    #[error("Timeout after {seconds} seconds")]
    Timeout { seconds: u64 },
    
    #[error("Target file size {target} cannot be achieved (minimum: {minimum})")]
    FileSizeUnachievable { target: ByteSize, minimum: ByteSize },
    
    // Options errors
    #[error("Invalid option: {option}: {reason}")]
    InvalidOption { option: String, reason: String },
    
    #[error("Preset not found: {name}")]
    PresetNotFound { name: String },
    
    // Dependency errors
    #[error("FFmpeg not found. Please install FFmpeg.")]
    FfmpegNotFound,
    
    #[error("Pandoc not found. Please install Pandoc for document conversion.")]
    PandocNotFound,
    
    #[error("Dependency error: {name}: {reason}")]
    DependencyError { name: String, reason: String },
    
    // Plugin errors
    #[error("Plugin error: {plugin}: {reason}")]
    PluginError { plugin: String, reason: String },
    
    #[error("Plugin not found: {name}")]
    PluginNotFound { name: String },
    
    // Config errors
    #[error("Configuration error: {reason}")]
    ConfigError { reason: String },
    
    // System errors
    #[error("Out of memory")]
    OutOfMemory,
    
    #[error("Out of disk space")]
    OutOfDiskSpace,
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    // Internal
    #[error("Internal error: {reason}")]
    Internal { reason: String },
}

impl ConvxError {
    /// Get error code for CLI exit status
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::FileNotFound { .. } => 1,
            Self::PermissionDenied { .. } => 2,
            Self::UnsupportedConversion { .. } => 3,
            Self::ConversionFailed { .. } => 4,
            Self::Cancelled => 5,
            Self::InvalidOption { .. } => 6,
            Self::FfmpegNotFound | Self::PandocNotFound => 7,
            _ => 1,
        }
    }
    
    /// Is this error recoverable?
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::Timeout { .. } | Self::OutOfMemory | Self::Cancelled
        )
    }
}
```

---

## 11. File I/O

### Temp File Management

```rust
/// Temp file manager
pub struct TempManager {
    /// Base directory for temp files
    base_dir: PathBuf,
    
    /// Active temp files (cleaned up on drop)
    active: Arc<Mutex<HashSet<PathBuf>>>,
}

impl TempManager {
    /// Create a new temp file
    pub fn create_temp_file(&self, extension: &str) -> Result<TempFile, ConvxError>;
    
    /// Create a temp directory
    pub fn create_temp_dir(&self) -> Result<TempDir, ConvxError>;
    
    /// Clean up all temp files
    pub fn cleanup(&self) -> Result<(), ConvxError>;
    
    /// Get total size of temp files
    pub fn temp_size(&self) -> u64;
}

/// RAII temp file that deletes on drop
pub struct TempFile {
    path: PathBuf,
    manager: Arc<TempManager>,
}

impl TempFile {
    pub fn path(&self) -> &Path;
    
    /// Keep the file (don't delete on drop)
    pub fn persist(self) -> PathBuf;
}

impl Drop for TempFile {
    fn drop(&mut self) {
        // Delete the temp file
    }
}
```

### Output Path Generation

```rust
/// Generate output path from template
pub fn generate_output_path(
    input: &Path,
    output_dir: &Path,
    output_format: Format,
    template: Option<&str>,
    index: usize,
) -> PathBuf {
    // Template placeholders:
    // {name} - original filename without extension
    // {ext} - new extension
    // {original_ext} - original extension
    // {index} - batch index (padded)
    // {date} - YYYY-MM-DD
    // {time} - HH-MM-SS
    // {datetime} - YYYY-MM-DD_HH-MM-SS
    // {format} - output format name
}
```

---

## 12. Watch Mode

### File Watcher

```rust
/// Watch mode configuration
#[derive(Debug, Clone)]
pub struct WatchConfig {
    /// Directories to watch
    pub watch_dirs: Vec<PathBuf>,
    
    /// Output directory
    pub output_dir: PathBuf,
    
    /// Conversion options
    pub options: ConversionOptions,
    
    /// Watch subdirectories
    pub recursive: bool,
    
    /// File patterns to watch (glob)
    pub patterns: Vec<String>,
    
    /// Ignore patterns
    pub ignore: Vec<String>,
    
    /// Debounce time in milliseconds
    pub debounce_ms: u64,
    
    /// Delete source after conversion
    pub delete_source: bool,
    
    /// Move source to directory after conversion
    pub move_source_to: Option<PathBuf>,
}

/// Watch handle for controlling the watcher
pub struct WatchHandle {
    id: Uuid,
    cancel_token: CancellationToken,
    status_rx: Receiver<WatchStatus>,
}

impl WatchHandle {
    /// Stop watching
    pub fn stop(&self);
    
    /// Pause watching
    pub fn pause(&self);
    
    /// Resume watching
    pub fn resume(&self);
    
    /// Get current status
    pub fn status(&self) -> WatchStatus;
    
    /// Wait for watcher to stop
    pub async fn wait(&self);
}

#[derive(Debug, Clone)]
pub struct WatchStatus {
    pub running: bool,
    pub paused: bool,
    pub files_converted: usize,
    pub errors: usize,
    pub last_conversion: Option<DateTime<Utc>>,
}
```

---

## 13. Plugin System

### Plugin Interface

```rust
/// Plugin trait that all plugins must implement
pub trait ConvxPlugin: Send + Sync {
    /// Plugin name
    fn name(&self) -> &str;
    
    /// Plugin version
    fn version(&self) -> &str;
    
    /// Plugin description
    fn description(&self) -> &str;
    
    /// Initialize the plugin
    fn init(&mut self, config: &PluginConfig) -> Result<(), ConvxError>;
    
    /// Shutdown the plugin
    fn shutdown(&mut self) -> Result<(), ConvxError>;
    
    /// Get supported conversions
    fn supported_conversions(&self) -> Vec<(Format, Format)>;
    
    /// Perform conversion
    fn convert(
        &self,
        input: &Path,
        output: &Path,
        options: &ConversionOptions,
        progress: &dyn ProgressCallback,
    ) -> Result<ConversionResult, ConvxError>;
    
    /// Check if plugin can handle this conversion
    fn can_convert(&self, from: Format, to: Format) -> bool {
        self.supported_conversions()
            .contains(&(from, to))
    }
}

/// Plugin configuration passed during init
#[derive(Debug, Clone)]
pub struct PluginConfig {
    pub data_dir: PathBuf,
    pub temp_dir: PathBuf,
    pub settings: HashMap<String, String>,
}

/// Plugin metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: Option<String>,
    pub homepage: Option<String>,
    pub supported_conversions: Vec<(Format, Format)>,
    pub enabled: bool,
}
```

### Plugin Loading

```rust
/// Plugin manager
pub struct PluginManager {
    plugins: HashMap<String, Box<dyn ConvxPlugin>>,
    plugin_dir: PathBuf,
}

impl PluginManager {
    /// Load all plugins from plugin directory
    pub fn load_all(&mut self) -> Result<(), ConvxError>;
    
    /// Load a specific plugin
    pub fn load(&mut self, path: &Path) -> Result<PluginInfo, ConvxError>;
    
    /// Unload a plugin
    pub fn unload(&mut self, name: &str) -> Result<(), ConvxError>;
    
    /// Get plugin for a conversion
    pub fn get_plugin_for(&self, from: Format, to: Format) -> Option<&dyn ConvxPlugin>;
    
    /// List all plugins
    pub fn list(&self) -> Vec<PluginInfo>;
}
```

---

## 14. CLI Interface

### Command Structure

```bash
convx [OPTIONS] <COMMAND>

Commands:
  convert    Convert one or more files
  batch      Batch convert files in a directory
  watch      Watch a directory and convert new files
  info       Get information about a file
  formats    List supported formats
  presets    List and manage presets
  config     Manage configuration
  plugins    Manage plugins
  version    Show version information
  help       Show help

Global Options:
  -v, --verbose     Increase verbosity (-v, -vv, -vvv)
  -q, --quiet       Suppress all output except errors
  -c, --config      Path to config file
  --no-color        Disable colored output
  --json            Output in JSON format
```

### Convert Command

```bash
convx convert [OPTIONS] <INPUT> [OUTPUT]

Arguments:
  <INPUT>   Input file or glob pattern
  [OUTPUT]  Output file or directory (default: same directory)

Options:
  -t, --to <FORMAT>           Output format
  -p, --preset <NAME>         Use a preset
  -q, --quality <0-100>       Quality level
  -s, --size <SIZE>           Target file size (e.g., "5MB", "500KB")
  -o, --output <PATH>         Output path
  --overwrite                 Overwrite existing files
  --keep                      Keep original file
  --delete                    Delete original after conversion
  
Image Options:
  -w, --width <PIXELS>        Resize width
  -h, --height <PIXELS>       Resize height
  --resize <MODE>             Resize mode (fit, fill, exact)
  --rotate <DEGREES>          Rotate image
  --strip                     Strip metadata
  --lossless                  Use lossless compression
  
Video Options:
  --codec <CODEC>             Video codec (h264, h265, vp9, av1)
  --fps <FPS>                 Frame rate
  --crf <0-51>                Quality (lower = better)
  --start <TIME>              Start time (HH:MM:SS or seconds)
  --end <TIME>                End time
  --duration <TIME>           Duration
  --no-audio                  Remove audio
  --hw-accel <TYPE>           Hardware acceleration
  
Audio Options:
  --audio-codec <CODEC>       Audio codec
  --bitrate <RATE>            Audio bitrate (e.g., "192k")
  --sample-rate <RATE>        Sample rate
  --channels <N>              Number of channels
  --normalize                 Normalize audio levels
  
Document Options:
  --page-size <SIZE>          Page size (letter, a4)
  --font <NAME>               Font family
  --font-size <PT>            Font size
  --toc                       Generate table of contents

Examples:
  # Basic conversion
  convx convert image.png image.webp
  convx convert video.mov --to mp4
  
  # With quality setting
  convx convert photo.jpg --to webp -q 80
  
  # Resize image
  convx convert large.png -w 800 --to jpg
  
  # Video to GIF
  convx convert video.mp4 --to gif --fps 15 -w 480
  
  # Using preset
  convx convert video.mov -p discord-video
  
  # Target file size
  convx convert video.mp4 --to mp4 -s 8MB
  
  # Batch with glob
  convx convert "*.png" --to webp -q 85
  
  # Extract audio
  convx convert video.mp4 --to mp3
```

### Batch Command

```bash
convx batch [OPTIONS] <INPUT_DIR> <OUTPUT_DIR>

Options:
  -t, --to <FORMAT>           Output format
  -p, --preset <NAME>         Use a preset
  -r, --recursive             Process subdirectories
  -j, --jobs <N>              Parallel conversions (default: CPU cores)
  --filter <FORMATS>          Only process these formats (comma-separated)
  --skip-existing             Skip if output exists
  --flatten                   Don't preserve directory structure
  --dry-run                   Show what would be converted
  --continue-on-error         Don't stop on conversion errors
  --template <TEMPLATE>       Output filename template

Examples:
  # Convert all PNGs to WebP
  convx batch ./images ./output --to webp
  
  # Recursive with filter
  convx batch ./media ./converted -r --filter "mp4,mov" --to webp
  
  # With parallelism
  convx batch ./photos ./optimized --to jpg -q 85 -j 8
  
  # Dry run
  convx batch ./input ./output --to webp --dry-run
```

### Watch Command

```bash
convx watch [OPTIONS] <INPUT_DIR> <OUTPUT_DIR>

Options:
  -t, --to <FORMAT>           Output format
  -p, --preset <NAME>         Use a preset
  -r, --recursive             Watch subdirectories
  --pattern <GLOB>            File patterns to watch
  --ignore <GLOB>             Patterns to ignore
  --debounce <MS>             Debounce time (default: 500)
  --delete-source             Delete source after conversion
  --move-source <DIR>         Move source after conversion

Examples:
  # Watch for new images
  convx watch ~/Downloads ~/Converted --to webp --pattern "*.png,*.jpg"
  
  # Auto-convert iPhone photos
  convx watch ~/Photos ~/Converted --to jpg --pattern "*.heic" --delete-source
```

### Info Command

```bash
convx info [OPTIONS] <FILE>

Options:
  --json                      Output as JSON
  --full                      Show all metadata

Examples:
  convx info video.mp4
  convx info image.png --full
  convx info document.pdf --json
```

### Formats Command

```bash
convx formats [OPTIONS]

Options:
  --from <FORMAT>             Show what this format can convert to
  --to <FORMAT>               Show what can convert to this format
  --category <CAT>            Filter by category (image, video, audio, document)
  --json                      Output as JSON

Examples:
  convx formats
  convx formats --from png
  convx formats --category video
```

### Presets Command

```bash
convx presets [COMMAND]

Commands:
  list                        List all presets
  show <NAME>                 Show preset details
  add <NAME> <OPTIONS>        Create custom preset
  remove <NAME>               Remove custom preset
  export <FILE>               Export presets to file
  import <FILE>               Import presets from file

Examples:
  convx presets list
  convx presets show twitter-gif
  convx presets add my-preset --to webp -q 90 -w 1200
```

---

## 15. FFI Bindings

### C FFI for Mobile

```rust
// convx-ffi/src/lib.rs

/// Opaque engine handle
pub struct ConvxHandle {
    engine: ConvxEngine,
}

/// Initialize the engine
#[no_mangle]
pub extern "C" fn convx_init() -> *mut ConvxHandle;

/// Free the engine
#[no_mangle]
pub extern "C" fn convx_free(handle: *mut ConvxHandle);

/// Convert a file
#[no_mangle]
pub extern "C" fn convx_convert(
    handle: *mut ConvxHandle,
    input_path: *const c_char,
    output_path: *const c_char,
    options_json: *const c_char,
) -> *mut c_char; // Returns JSON result

/// Convert bytes in memory
#[no_mangle]
pub extern "C" fn convx_convert_bytes(
    handle: *mut ConvxHandle,
    input_data: *const u8,
    input_len: usize,
    input_format: *const c_char,
    options_json: *const c_char,
    output_len: *mut usize,
) -> *mut u8; // Returns output bytes

/// Free a result string
#[no_mangle]
pub extern "C" fn convx_free_string(s: *mut c_char);

/// Free output bytes
#[no_mangle]
pub extern "C" fn convx_free_bytes(data: *mut u8, len: usize);

/// Get last error
#[no_mangle]
pub extern "C" fn convx_get_error() -> *const c_char;

/// Set progress callback
#[no_mangle]
pub extern "C" fn convx_set_progress_callback(
    handle: *mut ConvxHandle,
    callback: extern "C" fn(*const c_char),
);

/// Get file info
#[no_mangle]
pub extern "C" fn convx_get_file_info(
    handle: *mut ConvxHandle,
    path: *const c_char,
) -> *mut c_char; // Returns JSON

/// List supported formats
#[no_mangle]
pub extern "C" fn convx_list_formats() -> *mut c_char; // Returns JSON

/// Check if conversion is supported
#[no_mangle]
pub extern "C" fn convx_can_convert(
    from_format: *const c_char,
    to_format: *const c_char,
) -> bool;

/// Cancel ongoing operation
#[no_mangle]
pub extern "C" fn convx_cancel(handle: *mut ConvxHandle, id: *const c_char) -> bool;
```

### WASM Bindings

```rust
// convx-wasm/src/lib.rs

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct ConvxWasm {
    engine: ConvxEngine,
}

#[wasm_bindgen]
impl ConvxWasm {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<ConvxWasm, JsValue>;
    
    /// Convert bytes
    #[wasm_bindgen]
    pub fn convert(
        &self,
        input: &[u8],
        input_format: &str,
        output_format: &str,
        options: JsValue, // JSON options
    ) -> Result<Vec<u8>, JsValue>;
    
    /// Get file info from bytes
    #[wasm_bindgen]
    pub fn get_info(&self, data: &[u8]) -> Result<JsValue, JsValue>;
    
    /// Check if conversion supported
    #[wasm_bindgen]
    pub fn can_convert(&self, from: &str, to: &str) -> bool;
    
    /// List formats
    #[wasm_bindgen]
    pub fn list_formats(&self) -> JsValue;
    
    /// Get presets
    #[wasm_bindgen]
    pub fn get_presets(&self) -> JsValue;
}
```

---

## 16. Dependencies

### Rust Crates

```toml
[dependencies]
# Core
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"
thiserror = "1"
anyhow = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"

# Image processing
image = "0.25"
libvips = "1"  # Via vips-rs bindings
resvg = "0.35"  # SVG rendering
ravif = "0.11"  # AVIF
webp = "0.2"

# Video/Audio processing
# (FFmpeg via command-line or ffmpeg-next bindings)
ffmpeg-next = "6"

# Document processing
# (Pandoc via command-line)
pandoc = "0.8"  # Rust bindings

# File system
notify = "6"  # File watching
walkdir = "2"
glob = "0.3"
tempfile = "3"

# CLI
clap = { version = "4", features = ["derive"] }
indicatif = "0.17"  # Progress bars
console = "0.15"  # Colors/styling
dialoguer = "0.11"  # Interactive prompts

# Utilities
uuid = { version = "1", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
bytesize = "1"
rayon = "1"  # Parallelism
tracing = "0.1"
tracing-subscriber = "0.3"
once_cell = "1"

# WASM (optional)
wasm-bindgen = { version = "0.2", optional = true }
js-sys = { version = "0.3", optional = true }
web-sys = { version = "0.3", optional = true }

[features]
default = ["cli"]
cli = ["clap", "indicatif", "console", "dialoguer"]
wasm = ["wasm-bindgen", "js-sys", "web-sys"]
ffi = []
```

### System Dependencies

| Dependency | Purpose | Required |
|------------|---------|----------|
| FFmpeg | Video/audio conversion | Yes |
| libvips | Image processing | Yes |
| Pandoc | Document conversion | Optional |
| libheif | HEIC support | Optional |
| libavif | AVIF support | Optional |

---

## 17. Security Considerations

### Input Validation

```rust
/// Validate input file before processing
pub fn validate_input(path: &Path) -> Result<(), ConvxError> {
    // 1. Check file exists
    // 2. Check read permissions
    // 3. Check file size limits
    // 4. Validate magic bytes match extension
    // 5. Scan for path traversal attacks
    // 6. Check for symlink attacks
}

/// Validate output path
pub fn validate_output(path: &Path, overwrite: bool) -> Result<(), ConvxError> {
    // 1. Check parent directory exists
    // 2. Check write permissions
    // 3. Check if file exists (if !overwrite)
    // 4. Check available disk space
    // 5. Scan for path traversal attacks
}
```

### Sandboxing

- All conversions run in isolated temp directories
- No network access during conversion
- File system access restricted to input/output paths
- Resource limits (memory, CPU time) enforced

### Memory Safety

- All external library calls wrapped in safe Rust abstractions
- Input size limits enforced
- Streaming processing for large files
- Explicit cleanup of temp files

---

## 18. Performance Requirements

### Benchmarks

| Operation | Target | Max Memory |
|-----------|--------|------------|
| 10MB JPEG → WebP | < 500ms | 100MB |
| 100MB MP4 → MP4 (transcode) | < 30s | 500MB |
| 1GB video → GIF | < 5min | 2GB |
| Batch 100 images | < 10s | 500MB |
| Format detection | < 10ms | 10MB |

### Optimization Strategies

1. **Streaming processing** for large files
2. **Memory-mapped I/O** where possible
3. **Hardware acceleration** (CUDA, VideoToolbox, VAAPI)
4. **Parallel batch processing** with work stealing
5. **Lazy loading** of conversion backends
6. **Caching** of format detection results

---

## Appendix A: Version History

| Version | Date | Changes |
|---------|------|---------|
| 0.1.0 | 2026-01-26 | Initial specification |

---

## Appendix B: Glossary

| Term | Definition |
|------|------------|
| CRF | Constant Rate Factor - quality setting for video encoding |
| DXA | Device-independent pixels (used in documents) |
| FFI | Foreign Function Interface |
| WASM | WebAssembly |
| HW Accel | Hardware Acceleration |

---

*End of Specification*
