# ConvX Fix Prompt — Pass 2

> **Context:** You are working on ConvX (`/Users/jeffriebudde/convx/`), a Rust-based local-first file conversion tool with a CLI, Tauri desktop app, and MCP server. The first pass shipped: trait-based converter dispatch, real FFmpeg progress reporting with cancellation, full MCP server (9 tools over JSON-RPC stdin/stdout), presets system, watch mode, batch conversion with rayon parallelism, `--json` output on all commands, path validation in Tauri, CI pipeline, and comprehensive unit tests. This second pass addresses the remaining issues: duplicated code, trait inconsistencies, a broken preset, unimplemented options, and missing integration tests.

---

## Issue 1 — Extract ffprobe helpers into shared module

**Priority:** High (code health — 150+ lines duplicated three times)

**Problem:** The following functions are copy-pasted identically in `convx-core/src/main.rs` AND `convx-core/src/mcp_server.rs`:

- `probe_with_ffprobe(path) -> Result<Value>`
- `ffprobe_duration_seconds(json) -> Option<f64>`
- `ffprobe_dimensions(json) -> Option<(Option<u32>, Option<u32>)>`
- `ffprobe_fps(json) -> Option<f64>`
- `ffprobe_video_codec(json) -> Option<String>`
- `ffprobe_audio_codec(json) -> Option<String>`
- `ffprobe_audio_sample_rate(json) -> Option<u32>`
- `ffprobe_audio_channels(json) -> Option<u32>`
- `parse_ffprobe_fraction(value) -> Option<f64>`

Additionally, `VideoConverter` and `AudioConverter` each have their own `probe_duration_seconds` that does the same thing but returns only the duration.

**Fix:**

Create `convx-core/src/utils/ffprobe.rs` with a struct that holds parsed probe data:

```rust
// convx-core/src/utils/ffprobe.rs

use serde_json::Value;
use std::path::Path;
use std::process::Command;
use crate::utils::DependencyChecker;

pub struct FfprobeInfo {
    raw: Value,
}

impl FfprobeInfo {
    /// Probe a file. Returns None if ffprobe is unavailable or the file can't be probed.
    pub fn probe(path: &Path) -> Option<Self> {
        let ffprobe = DependencyChecker::ffprobe_executable()?;
        let output = Command::new(ffprobe)
            .args([
                "-v", "quiet",
                "-print_format", "json",
                "-show_format",
                "-show_streams",
            ])
            .arg(path)
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let raw: Value = serde_json::from_slice(&output.stdout).ok()?;
        Some(Self { raw })
    }

    pub fn duration_seconds(&self) -> Option<f64> {
        self.raw.get("format")
            .and_then(|f| f.get("duration"))
            .and_then(Value::as_str)
            .and_then(|s| s.parse::<f64>().ok())
            .filter(|d| *d > 0.0)
    }

    pub fn dimensions(&self) -> (Option<u32>, Option<u32>) {
        let stream = self.video_stream().or_else(|| self.first_stream());
        match stream {
            Some(s) => (
                s.get("width").and_then(Value::as_u64).and_then(|v| u32::try_from(v).ok()),
                s.get("height").and_then(Value::as_u64).and_then(|v| u32::try_from(v).ok()),
            ),
            None => (None, None),
        }
    }

    pub fn fps(&self) -> Option<f64> {
        let video = self.video_stream()?;
        parse_fraction(video.get("r_frame_rate").and_then(Value::as_str).unwrap_or("0/0"))
    }

    pub fn video_codec(&self) -> Option<String> {
        self.video_stream()
            .and_then(|s| s.get("codec_name"))
            .and_then(Value::as_str)
            .map(ToString::to_string)
    }

    pub fn audio_codec(&self) -> Option<String> {
        self.audio_stream()
            .and_then(|s| s.get("codec_name"))
            .and_then(Value::as_str)
            .map(ToString::to_string)
    }

    pub fn audio_sample_rate(&self) -> Option<u32> {
        self.audio_stream()
            .and_then(|s| s.get("sample_rate"))
            .and_then(Value::as_str)
            .and_then(|s| s.parse::<u32>().ok())
    }

    pub fn audio_channels(&self) -> Option<u32> {
        self.audio_stream()
            .and_then(|s| s.get("channels"))
            .and_then(Value::as_u64)
            .and_then(|v| u32::try_from(v).ok())
    }

    fn streams(&self) -> Option<&Vec<Value>> {
        self.raw.get("streams")?.as_array()
    }

    fn video_stream(&self) -> Option<&Value> {
        self.streams()?.iter().find(|s| {
            s.get("codec_type").and_then(Value::as_str) == Some("video")
        })
    }

    fn audio_stream(&self) -> Option<&Value> {
        self.streams()?.iter().find(|s| {
            s.get("codec_type").and_then(Value::as_str) == Some("audio")
        })
    }

    fn first_stream(&self) -> Option<&Value> {
        self.streams()?.first()
    }
}

fn parse_fraction(value: &str) -> Option<f64> {
    let (num, den) = value.split_once('/')?;
    let num = num.parse::<f64>().ok()?;
    let den = den.parse::<f64>().ok()?;
    if den == 0.0 { return None; }
    Some(num / den)
}
```

Update `convx-core/src/utils/mod.rs`:
```rust
pub mod deps;
pub mod ffprobe;

pub use deps::DependencyChecker;
pub use ffprobe::FfprobeInfo;
```

Then in `convx-core/src/lib.rs`, add the re-export:
```rust
pub use utils::FfprobeInfo;
```

**Then replace all call sites:**

In `main.rs`, the `Commands::Info` handler becomes:
```rust
let probe = convx::FfprobeInfo::probe(&path);
let duration_seconds = probe.as_ref().and_then(|p| p.duration_seconds());
let (width, height) = probe.as_ref().map(|p| p.dimensions()).unwrap_or((None, None));
// ... etc
```

Delete all nine duplicated functions from `main.rs`.

In `mcp_server.rs`, the `get_file_info` tool handler becomes the same pattern. Delete all nine duplicated functions from `mcp_server.rs`.

In `VideoConverter` and `AudioConverter`, replace the private `probe_duration_seconds` method with:
```rust
let duration_us = FfprobeInfo::probe(input)
    .and_then(|p| p.duration_seconds())
    .map(|d| d * 1_000_000.0);
```

Delete `probe_duration_seconds` from both converter files.

**Result:** ~150 lines of duplicated code eliminated. One place to fix probe bugs. One place to add new metadata fields.

---

## Issue 2 — `convert_with_progress` should use trait dispatch

**Priority:** Medium (architectural consistency)

**File:** `convx-core/src/engine.rs`

**Problem:** `ConvxEngine::convert()` correctly iterates `self.converters` and calls the `Converter` trait:

```rust
for converter in &self.converters {
    if converter.can_convert(input_format, output_format) {
        return converter.convert(input, output, &options);
    }
}
```

But `convert_with_progress()` bypasses this entirely and hardcodes concrete types with category matching:

```rust
match (input_format.category(), output_format.category()) {
    (FormatCategory::Video, FormatCategory::Video) => {
        let video = VideoConverter;  // ← hardcoded, ignores self.converters
        video.convert_with_progress(...)
    }
    // ...
}
```

If someone adds a new converter to the `converters` vec, `convert()` will use it but `convert_with_progress()` won't. The trait and the engine disagree about who does dispatch.

**Fix:**

Add `convert_with_progress` as an optional method on the `Converter` trait with a default implementation that falls back to `convert`:

```rust
// convx-core/src/converters/mod.rs

pub trait Converter: Send + Sync {
    fn can_convert(&self, from: Format, to: Format) -> bool;

    fn convert(
        &self,
        input: &Path,
        output: &Path,
        options: &ConversionOptions,
    ) -> Result<ConversionResult, ConvxError>;

    /// Convert with progress reporting. Default implementation calls convert()
    /// with synthetic 0.5 → 1.0 progress for converters that don't support
    /// real progress tracking.
    fn convert_with_progress(
        &self,
        input: &Path,
        output: &Path,
        options: &ConversionOptions,
        on_progress: &mut dyn FnMut(f32),
        cancel_flag: Option<&AtomicBool>,
    ) -> Result<ConversionResult, ConvxError> {
        // Check cancellation before starting
        if let Some(flag) = cancel_flag {
            if flag.load(std::sync::atomic::Ordering::Relaxed) {
                return Err(ConvxError::Cancelled);
            }
        }
        on_progress(0.5);
        let result = self.convert(input, output, options)?;
        on_progress(1.0);
        Ok(result)
    }
}
```

Then implement `convert_with_progress` on `VideoConverter` and `AudioConverter` (they already have the method — just add it to their `impl Converter for` block). `ImageConverter` can rely on the default.

Update `ConvxEngine::convert_with_progress()` to use trait dispatch:

```rust
pub fn convert_with_progress(
    &self,
    input: &Path,
    output: &Path,
    options: ConversionOptions,
    on_progress: &mut dyn FnMut(f32),
    cancel_flag: Option<&AtomicBool>,
) -> Result<ConversionResult, ConvxError> {
    let (input_format, output_format) = self.validate_request(input, output, &options)?;

    for converter in &self.converters {
        if converter.can_convert(input_format, output_format) {
            return converter.convert_with_progress(
                input, output, &options, on_progress, cancel_flag,
            );
        }
    }

    Err(ConvxError::UnsupportedConversion {
        from: input_format,
        to: output_format,
    })
}
```

**Result:** Both `convert()` and `convert_with_progress()` use the same dispatch path. Adding a new converter to the vec automatically works for both methods. The engine no longer knows about concrete converter types.

---

## Issue 3 — `extract-audio` preset is broken (video → audio not supported)

**Priority:** High (user-facing bug — preset exists but fails at runtime)

**File:** `convx-core/src/presets/mod.rs`, `convx-core/src/converters/audio.rs`, `convx-core/src/types/format.rs`

**Problem:** The `extract-audio` preset sets `output_format: Format::Mp3`. But `AudioConverter::can_convert()` only returns true when BOTH formats are audio:

```rust
fn can_convert(&self, from: Format, to: Format) -> bool {
    matches!(from.category(), FormatCategory::Audio)
        && matches!(to.category(), FormatCategory::Audio)
}
```

And `Format::convertible_targets()` doesn't include cross-category video→audio. So `convx convert video.mp4 --preset extract-audio` will fail with `UnsupportedConversion`.

FFmpeg absolutely supports this — it's just `ffmpeg -i input.mp4 -vn -c:a libmp3lame output.mp3`. The engine needs to know this path exists.

**Fix:**

Step 1 — Update `AudioConverter::can_convert` to also accept video input when target is audio:

```rust
pub fn can_convert(&self, from: Format, to: Format) -> bool {
    let to_audio = matches!(to.category(), FormatCategory::Audio);
    let from_audio_or_video = matches!(from.category(), FormatCategory::Audio | FormatCategory::Video);
    to_audio && from_audio_or_video
}
```

Step 2 — Update `AudioConverter::build_args` to add `-vn` (strip video) when the input is a video format:

In the `build_args` function, after the codec selection, add:

```rust
// Strip video track when extracting audio from video files
let input_format = Format::detect(input);
if let Some(fmt) = input_format {
    if matches!(fmt.category(), FormatCategory::Video) {
        args.push("-vn".to_string());
    }
}
```

Note: `build_args` doesn't currently have access to the input format. You have two options:
- (A) Pass `input` as a `&Path` to `build_args` and detect the format (you already pass `input` — it's there, just detect from it), OR
- (B) Add a `is_extraction: bool` parameter.

Option A is simpler since `build_args` already receives `input`.

Step 3 — Update `Format::convertible_targets()` to include video→audio paths:

```rust
pub fn convertible_targets(&self) -> Vec<Format> {
    Self::all()
        .iter()
        .filter(|&&target| {
            if target == *self {
                return false;
            }
            match (self.category(), target.category()) {
                (FormatCategory::Image, FormatCategory::Image) => {
                    !matches!(target, Format::Svg)
                }
                (FormatCategory::Video, FormatCategory::Video) => true,
                (FormatCategory::Video, FormatCategory::Image) if target == Format::Gif => true,
                (FormatCategory::Video, FormatCategory::Audio) => true,  // ← ADD THIS
                (FormatCategory::Audio, FormatCategory::Audio) => true,
                _ => false,
            }
        })
        .copied()
        .collect()
}
```

Step 4 — Add a test:

```rust
#[test]
fn video_targets_include_audio_formats() {
    let targets = Format::Mp4.convertible_targets();
    assert!(targets.contains(&Format::Mp3));
    assert!(targets.contains(&Format::Wav));
    assert!(targets.contains(&Format::Flac));
}
```

**Result:** `convx convert video.mp4 --preset extract-audio` works. `convx convert video.mp4 --to mp3` works. `get_conversion_targets` MCP tool correctly reports audio as a target for video input. The feature the preset promises is actually delivered.

---

## Issue 4 — `max_file_size` on presets is declared but never enforced

**Priority:** Low (aspirational field — not a bug, but misleading)

**File:** `convx-core/src/types/preset.rs`, `convx-core/src/presets/mod.rs`

**Problem:** The `discord` preset says `max_file_size: Some(8 * 1024 * 1024)` but nothing in the conversion pipeline checks whether the output exceeds this. Implementing constrained encoding (two-pass with target bitrate) is significant work and not worth doing now.

**Fix — choose one:**

**Option A (recommended):** Keep the field but document it clearly as advisory. Add a comment in the `Preset` struct:

```rust
/// Target maximum file size in bytes.
/// NOTE: This is currently advisory only — the engine does not enforce it.
/// Future versions may implement two-pass encoding to hit size targets.
pub max_file_size: Option<u64>,
```

And in the MCP `list_presets` / `get_preset` tool output, add a note in the description strings:
```
"Discord upload-friendly video (target: <8MB, not enforced yet)"
```

**Option B:** Remove the field entirely until you implement it. Simpler but loses the intent.

Go with Option A. Users and AI agents reading preset metadata should know the field exists but isn't enforced. Better to be transparent than to silently ignore a constraint.

---

## Issue 5 — ImageConverter ignores width, height, and strip_metadata options

**Priority:** High (options struct promises capabilities the converter doesn't deliver)

**File:** `convx-core/src/converters/image.rs`

**Problem:** `ImageOptions` has `width`, `height`, and `strip_metadata` fields. The `ImageConverter::convert` method never reads any of them. The vips command is just `vips copy input output[Q=...]`. No resize, no metadata stripping.

libvips supports both via the CLI:
- Resize: `vips thumbnail input output[Q=80] 1200` (fit within 1200px wide)
- Strip metadata: `vips copy input output[strip]` (the `strip` option on save)

**Fix:**

In `ImageConverter::convert()`, replace the vips command construction with logic that branches on whether resize is needed. Extract a helper for building the save-options suffix:

```rust
fn build_vips_save_suffix(options: &ConversionOptions) -> String {
    let image_opts = options.image.as_ref();
    let mut parts: Vec<String> = Vec::new();

    if let Some(q) = options.quality {
        match options.output_format {
            Format::Jpg | Format::WebP | Format::Heic | Format::Heif | Format::Avif => {
                parts.push(format!("Q={}", q));
            }
            Format::Png => {
                let compression = Self::quality_to_png_compression(q);
                parts.push(format!("compression={}", compression));
            }
            _ => {}
        }
    }

    if image_opts.map(|o| o.strip_metadata).unwrap_or(false) {
        parts.push("strip".to_string());
    }

    if parts.is_empty() {
        String::new()
    } else {
        format!("[{}]", parts.join(","))
    }
}
```

Then in the main convert method, after the ICO special case:

```rust
let image_opts = options.image.as_ref();
let needs_resize = image_opts.map(|o| o.width.is_some() || o.height.is_some()).unwrap_or(false);
let save_suffix = Self::build_vips_save_suffix(options);

if needs_resize {
    // Use `vips thumbnail` for resize — handles aspect ratio correctly
    let mut cmd = Command::new(&vips);
    cmd.arg("thumbnail");
    cmd.arg(input);

    let output_str = format!("{}{}", output.display(), save_suffix);
    cmd.arg(&output_str);

    // Width (required positional arg for thumbnail)
    let width = image_opts.and_then(|o| o.width).unwrap_or(9999);
    cmd.arg(width.to_string());

    // Height constraint (optional)
    if let Some(h) = image_opts.and_then(|o| o.height) {
        cmd.arg("--height");
        cmd.arg(h.to_string());
    }

    let status = cmd.output().map_err(|_| ConvxError::VipsNotFound)?;
    if !status.status.success() {
        let stderr = String::from_utf8_lossy(&status.stderr).to_string();
        tracing::debug!(stderr = %stderr, "vips thumbnail failed");
        return Err(ConvxError::ConversionFailed {
            reason: extract_tool_error(&stderr),
        });
    }
} else {
    // No resize — use `vips copy`
    let mut cmd = Command::new(&vips);
    cmd.arg("copy");
    cmd.arg(input);

    let output_str = format!("{}{}", output.display(), save_suffix);
    cmd.arg(&output_str);

    let status = cmd.output().map_err(|_| ConvxError::VipsNotFound)?;
    if !status.status.success() {
        let stderr = String::from_utf8_lossy(&status.stderr).to_string();
        tracing::debug!(stderr = %stderr, "vips copy failed");
        return Err(ConvxError::ConversionFailed {
            reason: extract_tool_error(&stderr),
        });
    }
}
```

**Result:** `--preset web-image` actually strips metadata. `--preset twitter-image` actually resizes to 1200px width. `--preset email-friendly` does both. The presets that set image options now produce the output they describe.

---

## Issue 6 — MCP `convert_file` creates unnecessary ImageOptions/VideoOptions

**Priority:** Low (cosmetic — doesn't cause bugs)

**File:** `convx-core/src/mcp_server.rs`

**Problem:** The `convert_file` tool handler always creates both `ImageOptions` and `VideoOptions`:

```rust
let options = ConversionOptions {
    // ...
    image: Some(ImageOptions { width: p.width, ..Default::default() }),
    video: Some(VideoOptions { width: p.width, fps: p.fps, ..Default::default() }),
    // ...
};
```

When converting an audio file, both are irrelevant.

**Fix:**

Only create the option structs when relevant parameters are provided:

```rust
let options = ConversionOptions {
    output_format,
    quality: p.quality,
    image: if p.width.is_some() {
        Some(ImageOptions { width: p.width, ..Default::default() })
    } else {
        None
    },
    video: if p.width.is_some() || p.fps.is_some() {
        Some(VideoOptions { width: p.width, fps: p.fps, ..Default::default() })
    } else {
        None
    },
    overwrite: p.overwrite.unwrap_or(false),
    ..Default::default()
};
```

The preset merge in `resolve_options` will fill in preset-specific values regardless.

---

## Issue 7 — Add integration tests that actually convert files

**Priority:** Medium (CI generates fixtures but no test uses them)

**File:** Create `convx-core/tests/integration.rs`

**Problem:** The CI pipeline generates `tests/fixtures/sample.png`, `tests/fixtures/sample.mp4`, `tests/fixtures/sample.wav` via `generate_fixtures.sh`. Unit tests cover logic. But no test actually invokes the engine on real files and verifies output.

**Fix:**

Create `convx-core/tests/integration.rs`:

```rust
//! Integration tests that require FFmpeg and libvips to be installed.
//! Run with: cargo test -- --ignored
//! CI runs these after installing system dependencies.

use convx::{ConversionOptions, ConvxEngine, Format};
use std::path::PathBuf;
use tempfile::TempDir;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn assert_file_nonempty(path: &std::path::Path) {
    assert!(path.exists(), "Output file does not exist: {}", path.display());
    let size = std::fs::metadata(path).expect("metadata").len();
    assert!(size > 0, "Output file is empty: {}", path.display());
}

// ── Image conversions ──────────────────────────────────────

#[test]
#[ignore]
fn png_to_webp() {
    let engine = ConvxEngine::new();
    let temp = TempDir::new().unwrap();
    let input = fixtures_dir().join("sample.png");
    let output = temp.path().join("output.webp");

    let result = engine.convert(&input, &output, ConversionOptions {
        output_format: Format::WebP,
        quality: Some(80),
        ..Default::default()
    }).expect("conversion should succeed");

    assert_file_nonempty(&output);
    assert!(result.output_size.unwrap() > 0);
    assert!(result.duration_ms < 30_000);
}

#[test]
#[ignore]
fn png_to_jpg_with_quality() {
    let engine = ConvxEngine::new();
    let temp = TempDir::new().unwrap();
    let input = fixtures_dir().join("sample.png");
    let output = temp.path().join("output.jpg");

    let result = engine.convert(&input, &output, ConversionOptions {
        output_format: Format::Jpg,
        quality: Some(50),
        ..Default::default()
    }).expect("conversion should succeed");

    assert_file_nonempty(&output);
    assert!(result.output_size.unwrap() > 0);
}

// ── Video conversions ──────────────────────────────────────

#[test]
#[ignore]
fn mp4_to_webm() {
    let engine = ConvxEngine::new();
    let temp = TempDir::new().unwrap();
    let input = fixtures_dir().join("sample.mp4");
    let output = temp.path().join("output.webm");

    engine.convert(&input, &output, ConversionOptions {
        output_format: Format::Webm,
        ..Default::default()
    }).expect("conversion should succeed");

    assert_file_nonempty(&output);
}

#[test]
#[ignore]
fn mp4_to_gif() {
    let engine = ConvxEngine::new();
    let temp = TempDir::new().unwrap();
    let input = fixtures_dir().join("sample.mp4");
    let output = temp.path().join("output.gif");

    engine.convert(&input, &output, ConversionOptions {
        output_format: Format::Gif,
        ..Default::default()
    }).expect("conversion should succeed");

    assert_file_nonempty(&output);
}

// ── Audio conversions ──────────────────────────────────────

#[test]
#[ignore]
fn wav_to_mp3() {
    let engine = ConvxEngine::new();
    let temp = TempDir::new().unwrap();
    let input = fixtures_dir().join("sample.wav");
    let output = temp.path().join("output.mp3");

    engine.convert(&input, &output, ConversionOptions {
        output_format: Format::Mp3,
        quality: Some(80),
        ..Default::default()
    }).expect("conversion should succeed");

    assert_file_nonempty(&output);
}

#[test]
#[ignore]
fn wav_to_flac() {
    let engine = ConvxEngine::new();
    let temp = TempDir::new().unwrap();
    let input = fixtures_dir().join("sample.wav");
    let output = temp.path().join("output.flac");

    engine.convert(&input, &output, ConversionOptions {
        output_format: Format::Flac,
        ..Default::default()
    }).expect("conversion should succeed");

    assert_file_nonempty(&output);
}

// ── Cross-category: video → audio extraction ───────────────
// (Only works after Issue 3 fix)

#[test]
#[ignore]
fn mp4_to_mp3_extract_audio() {
    let engine = ConvxEngine::new();
    let temp = TempDir::new().unwrap();
    let input = fixtures_dir().join("sample.mp4");
    let output = temp.path().join("extracted.mp3");

    engine.convert(&input, &output, ConversionOptions {
        output_format: Format::Mp3,
        ..Default::default()
    }).expect("audio extraction should succeed");

    assert_file_nonempty(&output);
}

// ── Error cases ────────────────────────────────────────────

#[test]
#[ignore]
fn overwrite_false_rejects_existing_output() {
    let engine = ConvxEngine::new();
    let temp = TempDir::new().unwrap();
    let input = fixtures_dir().join("sample.png");
    let output = temp.path().join("output.webp");

    engine.convert(&input, &output, ConversionOptions {
        output_format: Format::WebP,
        overwrite: false,
        ..Default::default()
    }).expect("first conversion should succeed");

    let result = engine.convert(&input, &output, ConversionOptions {
        output_format: Format::WebP,
        overwrite: false,
        ..Default::default()
    });

    assert!(result.is_err());
}

#[test]
#[ignore]
fn overwrite_true_replaces_existing_output() {
    let engine = ConvxEngine::new();
    let temp = TempDir::new().unwrap();
    let input = fixtures_dir().join("sample.png");
    let output = temp.path().join("output.webp");

    engine.convert(&input, &output, ConversionOptions {
        output_format: Format::WebP,
        overwrite: false,
        ..Default::default()
    }).expect("first conversion");

    engine.convert(&input, &output, ConversionOptions {
        output_format: Format::WebP,
        overwrite: true,
        ..Default::default()
    }).expect("overwrite should succeed");
}
```

Update CI to run ignored tests explicitly. In `.github/workflows/ci.yml`, change the test step:
```yaml
      - name: Test (unit)
        working-directory: convx-core
        run: cargo test

      - name: Test (integration)
        working-directory: convx-core
        run: cargo test -- --ignored
```

**Result:** CI actually verifies that conversions produce real output. If someone breaks ffmpeg arguments, the build breaks.

---

## Issue 8 — `convx-mcp` binary pulls all CLI dependencies

**Priority:** Low (binary size optimization — not urgent)

**File:** `convx-core/Cargo.toml`

**Problem:** The `convx-mcp` binary links against the `convx` library crate and pulls in `clap`, `rayon`, `glob`, `indicatif`, `console` — all CLI-only dependencies it doesn't need.

**Fix:**

Use Cargo features to gate CLI-only dependencies:

```toml
[features]
default = ["cli"]
cli = ["dep:clap", "dep:rayon", "dep:glob", "dep:indicatif", "dep:console"]

[dependencies]
clap = { version = "4", features = ["derive"], optional = true }
rayon = { version = "1.10", optional = true }
glob = { version = "0.3", optional = true }
indicatif = { version = "0.17", optional = true }
console = { version = "0.15", optional = true }
```

Update the binary targets:
```toml
[[bin]]
name = "convx"
path = "src/main.rs"
required-features = ["cli"]

[[bin]]
name = "convx-mcp"
path = "src/mcp/main.rs"
```

**This is optional and can be deferred.** Only matters if distributing `convx-mcp` independently.

---

## Issue 9 — Watch mode has no graceful shutdown

**Priority:** Low (quality of life)

**File:** `convx-core/src/watch.rs`

**Problem:** `run_watch` calls `rx.recv()` in an infinite loop. Ctrl+C kills the process with no cleanup.

**Fix:**

Add `ctrlc = "3"` to `Cargo.toml`. Then in `run_watch`, before the loop:

```rust
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

let shutdown = Arc::new(AtomicBool::new(false));
let shutdown_clone = shutdown.clone();

ctrlc::set_handler(move || {
    shutdown_clone.store(true, Ordering::SeqCst);
}).expect("Error setting Ctrl-C handler");
```

Change the loop to use `recv_timeout`:

```rust
loop {
    if shutdown.load(Ordering::Relaxed) {
        if !opts.json_output {
            println!("\nStopping watch.");
        }
        break;
    }

    let evt = match rx.recv_timeout(Duration::from_millis(200)) {
        Ok(evt) => evt,
        Err(mpsc::RecvTimeoutError::Timeout) => continue,
        Err(mpsc::RecvTimeoutError::Disconnected) => break,
    };

    // ... rest of event handling
}
```

**Result:** Clean shutdown message. File handles released. Process exits cleanly.

---

## Issue 10 — README needs rewriting for the actual feature set

**Priority:** Medium (this is what people see first)

**File:** Root `README.md`

**Fix:** Write a README that leads with the value proposition and shows all three modes:

```markdown
# ConvX

Local-first file conversion. Your files never leave your machine.

Convert images, video, and audio between 30+ formats — from the command line,
a desktop app, or as an MCP server for AI agents. No uploads, no subscriptions,
no API keys.

## Install

\```bash
# macOS
brew install ffmpeg vips
cargo install convx

# Or build from source
git clone ... && cd convx && cargo build --release
\```

## Quick Start

\```bash
# Single file
convx convert photo.heic --to jpg

# Batch with glob
convx convert "*.png" --to webp --quality 80 --jobs 4

# Use a preset
convx convert video.mp4 --preset discord

# Extract audio from video
convx convert interview.mp4 --to mp3

# Watch a folder
convx watch ./screenshots --to webp

# File info
convx info video.mp4

# MCP server (for Claude, Cursor, etc.)
convx mcp
\```

## MCP Server

ConvX runs as a Model Context Protocol server over stdin/stdout.
AI agents can convert files locally without uploading anywhere.

\```json
{
  "mcpServers": {
    "convx": {
      "command": "convx",
      "args": ["mcp"]
    }
  }
}
\```

**Tools:** convert_file, batch_convert, get_supported_formats,
get_conversion_targets, can_convert, get_file_info, list_presets,
get_preset, check_dependencies

## Presets

\```bash
convx presets list
convx presets show discord
\```

Built-in: discord, discord-nitro, twitter-image, twitter-gif,
instagram-story, web-image, email-friendly, heic-to-jpg,
archive-lossless, extract-audio

## Supported Formats

**Images:** png, jpg, webp, gif, bmp, tiff, ico, svg, heic, heif, avif
**Video:** mp4, mov, webm, avi, mkv, wmv, flv, m4v
**Audio:** mp3, wav, flac, m4a, aac, ogg, wma, aiff, opus

## Requirements

- FFmpeg (video/audio)
- libvips (images)

\```bash
convx check  # verify dependencies
\```
```

---

## Execution Order

Do them in this order for maximum impact with minimum wasted work:

1. **Issue 1 — ffprobe dedup** (30 min) — Do first because Issues 3, 5, and 7 touch the same files. Clean foundation.
2. **Issue 3 — Fix extract-audio** (30 min) — High priority, user-facing bug. Unblocks the integration test for it.
3. **Issue 5 — ImageConverter options** (45 min) — High priority. Makes presets actually work.
4. **Issue 2 — Trait dispatch for progress** (20 min) — Clean architectural fix, touches engine.rs once.
5. **Issue 7 — Integration tests** (30 min) — Now that Issues 3 and 5 are fixed, tests will exercise real paths.
6. **Issue 4 — Document max_file_size** (5 min) — One-line comment.
7. **Issue 6 — MCP options cleanup** (10 min) — Quick conditional.
8. **Issue 10 — README** (20 min) — Now that everything works, document it.
9. **Issue 9 — Graceful watch shutdown** (15 min) — Nice to have.
10. **Issue 8 — Feature-gate CLI deps** (20 min) — Only if distributing MCP binary separately.

**Total estimated time: ~3.5 hours.**
