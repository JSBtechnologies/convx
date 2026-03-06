# ConvX — Fix Prompt (Round 2)

You are working on **ConvX**, a local-first file conversion tool with CLI, desktop (Tauri), and MCP server interfaces. The first round of fixes shipped the Converter trait, MCP server, presets, watch mode, batch conversion, real progress reporting, path validation, and comprehensive error types.

This prompt addresses the remaining issues: a broken preset, duplicated code, unimplemented options, trait inconsistency, and missing integration tests. Everything here is concrete — exact files, exact code, exact reasoning.

**Repository layout:**

```
convx/
├── Cargo.toml                          # workspace root
├── convx-core/
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── main.rs                     # CLI binary
│   │   ├── engine.rs                   # ConvxEngine
│   │   ├── mcp_server.rs              # MCP JSON-RPC server
│   │   ├── watch.rs                   # Watch mode
│   │   ├── mcp/main.rs               # convx-mcp binary
│   │   ├── converters/
│   │   │   ├── mod.rs                 # Converter trait + extract_tool_error
│   │   │   ├── image.rs              # ImageConverter (vips)
│   │   │   ├── video.rs              # VideoConverter (ffmpeg)
│   │   │   └── audio.rs             # AudioConverter (ffmpeg)
│   │   ├── presets/mod.rs            # Built-in presets + resolve_options
│   │   ├── types/
│   │   │   ├── mod.rs
│   │   │   ├── format.rs            # Format enum
│   │   │   ├── options.rs           # ConversionOptions, ImageOptions, etc.
│   │   │   ├── result.rs            # ConversionResult
│   │   │   ├── error.rs             # ConvxError
│   │   │   └── preset.rs           # Preset struct
│   │   └── utils/
│   │       ├── mod.rs
│   │       └── deps.rs              # DependencyChecker
│   └── tests/
│       ├── generate_fixtures.sh
│       └── fixtures/                  # Generated test media files
├── convx-app/                         # Tauri desktop app
│   └── src-tauri/src/commands.rs
└── .github/workflows/ci.yml
```

---

## Issue 1 — `extract-audio` preset is broken (BUG, fix first)

**Problem:** The `extract-audio` preset sets `output_format: Format::Mp3`, but the conversion matrix only supports same-category conversions. Video → Audio is not a registered conversion path in any converter. Using `--preset extract-audio` on a video file returns `UnsupportedConversion`.

This is user-facing and the preset's entire reason for existing. Fix it.

**Files:** `convx-core/src/converters/audio.rs`, `convx-core/src/types/format.rs`

**Fix:** Expand `AudioConverter::can_convert` to accept video inputs when the output is audio. FFmpeg already handles this natively — `ffmpeg -i video.mp4 -vn -c:a libmp3lame output.mp3` just works. The converter needs to know it's allowed.

In `audio.rs`, change `can_convert`:

```rust
pub fn can_convert(&self, from: Format, to: Format) -> bool {
    let to_audio = matches!(to.category(), FormatCategory::Audio);
    let from_audio = matches!(from.category(), FormatCategory::Audio);
    let from_video = matches!(from.category(), FormatCategory::Video);

    to_audio && (from_audio || from_video)
}
```

In `build_args`, when the input is a video file, add `-vn` to strip the video stream:

```rust
// After codec selection, before bitrate
let input_format = Format::detect(input);
let is_video_input = input_format
    .map(|f| matches!(f.category(), FormatCategory::Video))
    .unwrap_or(false);

if is_video_input {
    args.push("-vn".to_string());
}
```

In `format.rs`, update `convertible_targets()` to include audio targets for video inputs:

```rust
(FormatCategory::Video, FormatCategory::Audio) => true,
```

Add this arm alongside the existing `(FormatCategory::Video, FormatCategory::Image) if target == Format::Gif => true` arm.

**Test:** `convx convert sample.mp4 --preset extract-audio` should produce `sample.mp3`. Also: `convx convert sample.mp4 --to wav` should work.

---

## Issue 2 — Deduplicate ffprobe helpers into `utils/probe.rs`

**Problem:** The following functions are copy-pasted identically in `main.rs` and `mcp_server.rs`:

- `probe_with_ffprobe`
- `ffprobe_duration_seconds`
- `ffprobe_dimensions`
- `ffprobe_fps`
- `ffprobe_video_codec`
- `ffprobe_audio_codec`
- `ffprobe_audio_sample_rate`
- `ffprobe_audio_channels`
- `parse_ffprobe_fraction`

Additionally, `video.rs` and `audio.rs` each have their own `probe_duration_seconds` that does the same thing with slightly different return types.

That's ~150 lines duplicated 3 times. When you need to fix a probe bug, you'll forget one copy.

**Files to create:** `convx-core/src/utils/probe.rs`

**Files to modify:** `convx-core/src/utils/mod.rs`, `convx-core/src/main.rs`, `convx-core/src/mcp_server.rs`, `convx-core/src/converters/video.rs`, `convx-core/src/converters/audio.rs`, `convx-core/src/lib.rs`

**Implementation:**

Create `convx-core/src/utils/probe.rs`:

```rust
use crate::utils::DependencyChecker;
use serde_json::Value;
use std::path::Path;
use std::process::Command;

/// Structured ffprobe output for a media file.
#[derive(Debug, Clone, Default)]
pub struct ProbeInfo {
    pub duration_seconds: Option<f64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fps: Option<f64>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub audio_sample_rate: Option<u32>,
    pub audio_channels: Option<u32>,
}

impl ProbeInfo {
    /// Run ffprobe on the given path and parse all available metadata.
    pub fn probe(path: &Path) -> Option<Self> {
        let raw = probe_raw_json(path).ok()?;
        Some(Self {
            duration_seconds: extract_duration(&raw),
            width: extract_video_field(&raw, "width").and_then(|v| v.as_u64()).and_then(|v| u32::try_from(v).ok()),
            height: extract_video_field(&raw, "height").and_then(|v| v.as_u64()).and_then(|v| u32::try_from(v).ok()),
            fps: extract_video_field(&raw, "r_frame_rate")
                .and_then(|v| v.as_str().map(String::from))
                .and_then(|s| parse_fraction(&s)),
            video_codec: extract_video_field(&raw, "codec_name").and_then(|v| v.as_str().map(String::from)),
            audio_codec: extract_audio_field(&raw, "codec_name").and_then(|v| v.as_str().map(String::from)),
            audio_sample_rate: extract_audio_field(&raw, "sample_rate")
                .and_then(|v| v.as_str().and_then(|s| s.parse::<u32>().ok())),
            audio_channels: extract_audio_field(&raw, "channels")
                .and_then(|v| v.as_u64())
                .and_then(|v| u32::try_from(v).ok()),
        })
    }

    /// Quick probe that only extracts duration. Used by converters for progress calculation.
    pub fn duration(path: &Path) -> Option<f64> {
        let ffprobe = DependencyChecker::ffprobe_executable()?;
        let out = Command::new(ffprobe)
            .args([
                "-v", "error",
                "-show_entries", "format=duration",
                "-of", "default=noprint_wrappers=1:nokey=1",
            ])
            .arg(path)
            .output()
            .ok()?;

        if !out.status.success() {
            return None;
        }

        String::from_utf8_lossy(&out.stdout)
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|d| *d > 0.0)
    }
}

fn probe_raw_json(path: &Path) -> anyhow::Result<Value> {
    let ffprobe = DependencyChecker::ffprobe_executable()
        .ok_or_else(|| anyhow::anyhow!("ffprobe not found"))?;

    let output = Command::new(ffprobe)
        .args(["-v", "quiet", "-print_format", "json", "-show_format", "-show_streams"])
        .arg(path)
        .output()?;

    if !output.status.success() {
        anyhow::bail!("ffprobe failed for {}", path.display());
    }

    Ok(serde_json::from_slice(&output.stdout)?)
}

fn extract_duration(json: &Value) -> Option<f64> {
    json.get("format")?
        .get("duration")?
        .as_str()?
        .parse::<f64>()
        .ok()
}

fn find_stream<'a>(json: &'a Value, codec_type: &str) -> Option<&'a Value> {
    json.get("streams")?
        .as_array()?
        .iter()
        .find(|s| s.get("codec_type").and_then(Value::as_str) == Some(codec_type))
}

fn extract_video_field(json: &Value, field: &str) -> Option<Value> {
    find_stream(json, "video")?.get(field).cloned()
}

fn extract_audio_field(json: &Value, field: &str) -> Option<Value> {
    find_stream(json, "audio")?.get(field).cloned()
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
pub mod probe;

pub use deps::DependencyChecker;
pub use probe::ProbeInfo;
```

Update `convx-core/src/lib.rs` — add to the pub use block:

```rust
pub use utils::ProbeInfo;
```

**Then refactor consumers:**

In `video.rs` and `audio.rs`, replace `probe_duration_seconds` with:

```rust
use crate::utils::ProbeInfo;

// In convert_with_progress, replace:
//   let duration_secs = Self::probe_duration_seconds(input);
// With:
let duration_secs = ProbeInfo::duration(input);
```

Delete the `probe_duration_seconds` method from both `VideoConverter` and `AudioConverter`.

In `main.rs`, replace all the ffprobe helper functions with:

```rust
use convx::ProbeInfo;
```

Then in the `Commands::Info` handler:

```rust
let probe = ProbeInfo::probe(&path);
let duration_seconds = probe.as_ref().and_then(|p| p.duration_seconds);
let width = probe.as_ref().and_then(|p| p.width);
let height = probe.as_ref().and_then(|p| p.height);
let fps = if is_image { None } else { probe.as_ref().and_then(|p| p.fps) };
let video_codec = if is_image { None } else { probe.as_ref().and_then(|p| p.video_codec.clone()) };
let audio_codec = probe.as_ref().and_then(|p| p.audio_codec.clone());
let audio_sample_rate = probe.as_ref().and_then(|p| p.audio_sample_rate);
let audio_channels = probe.as_ref().and_then(|p| p.audio_channels);
```

Delete `probe_with_ffprobe`, `ffprobe_duration_seconds`, `ffprobe_dimensions`, `ffprobe_fps`, `ffprobe_video_codec`, `ffprobe_audio_codec`, `ffprobe_audio_sample_rate`, `ffprobe_audio_channels`, and `parse_ffprobe_fraction` from `main.rs`.

Do the same in `mcp_server.rs`. The `get_file_info` tool handler becomes:

```rust
"get_file_info" => {
    let p: FileInfoParams = serde_json::from_value(arguments).map_err(|e| format!("Invalid params: {}", e))?;
    let path = PathBuf::from(p.path);
    let metadata = std::fs::metadata(&path).map_err(|e| e.to_string())?;
    let format = Format::detect(&path);
    let targets: Vec<String> = format
        .map(|f| f.convertible_targets().into_iter().map(|t| t.extension().to_string()).collect())
        .unwrap_or_default();

    let is_image = matches!(format.map(|f| f.category()), Some(crate::FormatCategory::Image));
    let probe = crate::ProbeInfo::probe(&path);

    Ok(json!({
        "path": path,
        "name": path.file_name().and_then(|v| v.to_str()).unwrap_or_default(),
        "size": metadata.len(),
        "format": format.map(|f| f.extension()),
        "conversion_targets": targets,
        "duration_seconds": probe.as_ref().and_then(|p| p.duration_seconds),
        "width": probe.as_ref().and_then(|p| p.width),
        "height": probe.as_ref().and_then(|p| p.height),
        "fps": if is_image { None } else { probe.as_ref().and_then(|p| p.fps) },
        "video_codec": if is_image { None } else { probe.as_ref().and_then(|p| p.video_codec.clone()) },
        "audio_codec": probe.as_ref().and_then(|p| p.audio_codec.clone()),
        "audio_sample_rate": probe.as_ref().and_then(|p| p.audio_sample_rate),
        "audio_channels": probe.as_ref().and_then(|p| p.audio_channels),
    }))
}
```

Delete all the standalone ffprobe functions from `mcp_server.rs`.

**Net result:** ~450 lines of duplicated code replaced by one ~100-line module. One place to fix probe bugs forever.

---

## Issue 3 — `convert_with_progress` bypasses Converter trait dispatch

**Problem:** `ConvxEngine::convert()` correctly iterates `self.converters` and calls the trait method. But `convert_with_progress()` hardcodes match arms for each category, instantiating converters directly:

```rust
// Current code in engine.rs — bypasses trait dispatch
(FormatCategory::Video, FormatCategory::Video) => {
    let video = VideoConverter;           // ← direct instantiation
    video.convert_with_progress(...)      // ← not the trait
}
```

If someone adds a new converter to the `converters` vec, `convert()` will use it but `convert_with_progress()` won't.

**Files:** `convx-core/src/converters/mod.rs`, `convx-core/src/engine.rs`, `convx-core/src/converters/video.rs`, `convx-core/src/converters/audio.rs`, `convx-core/src/converters/image.rs`

**Fix:** Add `convert_with_progress` to the `Converter` trait with a default implementation that falls back to `convert()` with synthetic 0→1 progress.

In `convx-core/src/converters/mod.rs`, extend the trait:

```rust
pub trait Converter: Send + Sync {
    fn can_convert(&self, from: Format, to: Format) -> bool;

    fn convert(
        &self,
        input: &Path,
        output: &Path,
        options: &ConversionOptions,
    ) -> Result<ConversionResult, ConvxError>;

    /// Convert with progress reporting and optional cancellation.
    /// Default implementation calls `convert()` with synthetic 0 → 1 progress.
    fn convert_with_progress(
        &self,
        input: &Path,
        output: &Path,
        options: &ConversionOptions,
        on_progress: &mut dyn FnMut(f32),
        cancel_flag: Option<&std::sync::atomic::AtomicBool>,
    ) -> Result<ConversionResult, ConvxError> {
        if let Some(flag) = cancel_flag {
            if flag.load(std::sync::atomic::Ordering::Relaxed) {
                return Err(ConvxError::Cancelled);
            }
        }
        on_progress(0.0);
        let result = self.convert(input, output, options)?;
        on_progress(1.0);
        Ok(result)
    }
}
```

In `video.rs` and `audio.rs`, implement the trait method by delegating to the existing inherent `convert_with_progress`:

```rust
impl Converter for VideoConverter {
    fn can_convert(&self, from: Format, to: Format) -> bool {
        Self::can_convert(self, from, to)
    }

    fn convert(
        &self,
        input: &Path,
        output: &Path,
        options: &ConversionOptions,
    ) -> Result<ConversionResult, ConvxError> {
        Self::convert(self, input, output, options)
    }

    fn convert_with_progress(
        &self,
        input: &Path,
        output: &Path,
        options: &ConversionOptions,
        on_progress: &mut dyn FnMut(f32),
        cancel_flag: Option<&std::sync::atomic::AtomicBool>,
    ) -> Result<ConversionResult, ConvxError> {
        // Delegate to the inherent method that has real FFmpeg progress parsing
        Self::convert_with_progress(self, input, output, options, on_progress, cancel_flag)
    }
}
```

Do the same for `AudioConverter`.

For `ImageConverter`, the default trait implementation is fine (images convert near-instantly), so you don't need to override it. But if you want the 0.5 midpoint hint for the Tauri UI, override it:

```rust
impl Converter for ImageConverter {
    // ... existing can_convert and convert ...

    fn convert_with_progress(
        &self,
        input: &Path,
        output: &Path,
        options: &ConversionOptions,
        on_progress: &mut dyn FnMut(f32),
        _cancel_flag: Option<&std::sync::atomic::AtomicBool>,
    ) -> Result<ConversionResult, ConvxError> {
        on_progress(0.0);
        let result = self.convert(input, output, options)?;
        on_progress(1.0);
        Ok(result)
    }
}
```

Now rewrite `ConvxEngine::convert_with_progress` to use trait dispatch:

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
            return converter.convert_with_progress(input, output, &options, on_progress, cancel_flag);
        }
    }

    Err(ConvxError::UnsupportedConversion {
        from: input_format,
        to: output_format,
    })
}
```

This is now structurally identical to `convert()`. Any converter added to the vec gets progress support automatically (with at minimum the 0→1 default).

---

## Issue 4 — ImageConverter ignores width, height, and strip_metadata

**Problem:** `ImageOptions` has `width`, `height`, and `strip_metadata` fields. The image converter's `convert()` method never reads them. The vips command is always just `vips copy input output[Q=...]`. No resizing, no metadata stripping.

This means:
- `convx convert photo.jpg --to webp --width 800` silently ignores `--width`
- The `web-image` preset sets `strip_metadata: true` but metadata is preserved
- The `email-friendly` preset sets `width: Some(1200)` but images aren't resized
- The `twitter-image` preset sets `width: Some(1200)` but images aren't resized

**Files:** `convx-core/src/converters/image.rs`

**Fix:** Use `vipsthumbnail` for resize operations and `vips copy` with `[strip]` suffix for metadata stripping. When both are needed, chain them.

Replace the non-ICO branch in `ImageConverter::convert()`:

```rust
// Inside the `else` block (non-ICO conversions)
let vips = DependencyChecker::vips_executable()
    .ok_or(ConvxError::VipsNotFound)?;

let image_opts = options.image.as_ref();
let needs_resize = image_opts
    .map(|o| o.width.is_some() || o.height.is_some())
    .unwrap_or(false);
let strip_metadata = image_opts.map(|o| o.strip_metadata).unwrap_or(false);

if needs_resize {
    // Use vipsthumbnail for resizing
    let thumbnail_bin = DependencyChecker::vipsthumbnail_executable()
        .ok_or(ConvxError::VipsNotFound)?;

    let mut cmd = Command::new(&thumbnail_bin);
    cmd.arg(input);

    // Build size spec: WIDTHxHEIGHT or just WIDTH
    let width = image_opts.and_then(|o| o.width);
    let height = image_opts.and_then(|o| o.height);
    let size_spec = match (width, height) {
        (Some(w), Some(h)) => format!("{}x{}", w, h),
        (Some(w), None) => format!("{}", w),
        (None, Some(h)) => format!("x{}", h),
        (None, None) => unreachable!(), // guarded by needs_resize
    };
    cmd.args(["--size", &size_spec]);

    // Output with quality and strip options
    let output_spec = Self::build_vips_output_spec(output, options, strip_metadata);
    cmd.args(["-o", &output_spec]);

    let status = cmd.output().map_err(|_| ConvxError::VipsNotFound)?;
    if !status.status.success() {
        let stderr = String::from_utf8_lossy(&status.stderr).to_string();
        tracing::debug!(stderr = %stderr, "vipsthumbnail conversion failed");
        return Err(ConvxError::ConversionFailed {
            reason: extract_tool_error(&stderr),
        });
    }
} else {
    // No resize needed — use vips copy
    let mut cmd = Command::new(&vips);
    cmd.arg("copy");
    cmd.arg(input);

    let output_spec = Self::build_vips_output_spec(output, options, strip_metadata);
    cmd.arg(&output_spec);

    let status = cmd.output().map_err(|_| ConvxError::VipsNotFound)?;
    if !status.status.success() {
        let stderr = String::from_utf8_lossy(&status.stderr).to_string();
        tracing::debug!(stderr = %stderr, "vips conversion failed");
        return Err(ConvxError::ConversionFailed {
            reason: extract_tool_error(&stderr),
        });
    }
}
```

Add the shared helper method to `ImageConverter`:

```rust
fn build_vips_output_spec(output: &Path, options: &ConversionOptions, strip: bool) -> String {
    let mut suffixes = Vec::new();

    if let Some(q) = options.quality {
        match options.output_format {
            Format::Jpg | Format::WebP => suffixes.push(format!("Q={}", q)),
            Format::Heic | Format::Heif | Format::Avif => suffixes.push(format!("Q={}", q)),
            Format::Png => {
                let compression = Self::quality_to_png_compression(q);
                suffixes.push(format!("compression={}", compression));
            }
            _ => {}
        }
    }

    if strip {
        suffixes.push("strip".to_string());
    }

    if suffixes.is_empty() {
        output.display().to_string()
    } else {
        format!("{}[{}]", output.display(), suffixes.join(","))
    }
}
```

Add `vipsthumbnail_executable` to `DependencyChecker` in `convx-core/src/utils/deps.rs`:

```rust
pub fn vipsthumbnail_executable() -> Option<String> {
    Self::resolve_binary("vipsthumbnail", "--version")
}
```

---

## Issue 5 — Remove or document `max_file_size` on presets

**Problem:** The `Preset` struct has `max_file_size: Option<u64>`. The `discord` preset sets it to 8MB, `discord-nitro` to 50MB, `email-friendly` to 1MB. But nothing in the conversion pipeline reads this field. No two-pass encoding, no output size check, no warning.

Users will expect `--preset discord` to produce a file under 8MB. It won't.

**Option A (recommended now):** Remove the field entirely. Add it back when you implement constrained encoding.

**Option B:** Keep the field but add a post-conversion size check that warns (not errors) if the output exceeds the limit.

**Files:** `convx-core/src/types/preset.rs`, `convx-core/src/presets/mod.rs`

**If Option A:**

In `preset.rs`, remove the field:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub name: &'static str,
    pub description: &'static str,
    pub output_format: Format,
    pub quality: Option<u8>,
    pub video: Option<VideoOptions>,
    pub audio: Option<AudioOptions>,
    pub image: Option<ImageOptions>,
}
```

In `presets/mod.rs`, remove all `max_file_size` lines from every preset definition.

**If Option B:**

Add a post-conversion check in the CLI convert handler (in `main.rs`). After a successful conversion with a preset that has `max_file_size`, check the output:

```rust
if let Some(preset) = &preset {
    if let Some(max_size) = preset.max_file_size {
        if let Some(output_size) = result.output_size {
            if output_size > max_size {
                eprintln!(
                    "⚠ Output ({}) exceeds preset limit ({}). Consider lowering quality.",
                    format_size(output_size),
                    format_size(max_size),
                );
            }
        }
    }
}
```

This is an honest warning, not a lie. The user knows the file is too big and can adjust.

---

## Issue 6 — Integration tests using generated fixtures

**Problem:** Unit tests validate logic (format parsing, quality mapping, error paths) but nothing tests that actual conversions work. The CI pipeline generates fixtures via `generate_fixtures.sh` but no test uses them.

**Files to create:** `convx-core/tests/integration.rs`

**Implementation:**

```rust
//! Integration tests — require ffmpeg and libvips installed.
//! Run with: cargo test -- --ignored
//! CI runs these after generate_fixtures.sh.

use std::path::{Path, PathBuf};
use convx::{ConvxEngine, ConversionOptions, Format, ImageOptions, VideoOptions, AudioOptions};

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn fixture(name: &str) -> PathBuf {
    let p = fixtures_dir().join(name);
    assert!(p.exists(), "Fixture missing: {}. Run tests/generate_fixtures.sh", p.display());
    p
}

fn temp_output(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("convx-integration-tests");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir.join(name)
}

fn cleanup(path: &Path) {
    let _ = std::fs::remove_file(path);
}

#[test]
#[ignore]
fn image_png_to_webp() {
    let engine = ConvxEngine::new();
    let input = fixture("sample.png");
    let output = temp_output("test_png_to_webp.webp");
    cleanup(&output);

    let result = engine.convert(&input, &output, ConversionOptions {
        output_format: Format::WebP,
        quality: Some(80),
        overwrite: true,
        ..Default::default()
    });

    assert!(result.is_ok(), "Conversion failed: {:?}", result.err());
    assert!(output.exists(), "Output file not created");
    assert!(std::fs::metadata(&output).unwrap().len() > 0, "Output file is empty");
    cleanup(&output);
}

#[test]
#[ignore]
fn image_png_to_jpg() {
    let engine = ConvxEngine::new();
    let input = fixture("sample.png");
    let output = temp_output("test_png_to_jpg.jpg");
    cleanup(&output);

    let result = engine.convert(&input, &output, ConversionOptions {
        output_format: Format::Jpg,
        quality: Some(85),
        overwrite: true,
        ..Default::default()
    });

    assert!(result.is_ok());
    assert!(output.exists());
    assert!(std::fs::metadata(&output).unwrap().len() > 0);
    cleanup(&output);
}

#[test]
#[ignore]
fn video_mp4_to_webm() {
    let engine = ConvxEngine::new();
    let input = fixture("sample.mp4");
    let output = temp_output("test_mp4_to_webm.webm");
    cleanup(&output);

    let result = engine.convert(&input, &output, ConversionOptions {
        output_format: Format::Webm,
        quality: Some(60),
        overwrite: true,
        ..Default::default()
    });

    assert!(result.is_ok(), "Conversion failed: {:?}", result.err());
    assert!(output.exists());
    assert!(std::fs::metadata(&output).unwrap().len() > 0);
    cleanup(&output);
}

#[test]
#[ignore]
fn video_mp4_to_gif() {
    let engine = ConvxEngine::new();
    let input = fixture("sample.mp4");
    let output = temp_output("test_mp4_to_gif.gif");
    cleanup(&output);

    let result = engine.convert(&input, &output, ConversionOptions {
        output_format: Format::Gif,
        overwrite: true,
        video: Some(VideoOptions {
            fps: Some(10.0),
            width: Some(320),
            ..Default::default()
        }),
        ..Default::default()
    });

    assert!(result.is_ok(), "Conversion failed: {:?}", result.err());
    assert!(output.exists());
    assert!(std::fs::metadata(&output).unwrap().len() > 0);
    cleanup(&output);
}

#[test]
#[ignore]
fn audio_wav_to_mp3() {
    let engine = ConvxEngine::new();
    let input = fixture("sample.wav");
    let output = temp_output("test_wav_to_mp3.mp3");
    cleanup(&output);

    let result = engine.convert(&input, &output, ConversionOptions {
        output_format: Format::Mp3,
        quality: Some(75),
        overwrite: true,
        ..Default::default()
    });

    assert!(result.is_ok(), "Conversion failed: {:?}", result.err());
    assert!(output.exists());
    assert!(std::fs::metadata(&output).unwrap().len() > 0);
    cleanup(&output);
}

#[test]
#[ignore]
fn audio_wav_to_flac() {
    let engine = ConvxEngine::new();
    let input = fixture("sample.wav");
    let output = temp_output("test_wav_to_flac.flac");
    cleanup(&output);

    let result = engine.convert(&input, &output, ConversionOptions {
        output_format: Format::Flac,
        overwrite: true,
        ..Default::default()
    });

    assert!(result.is_ok());
    assert!(output.exists());
    assert!(std::fs::metadata(&output).unwrap().len() > 0);
    cleanup(&output);
}

#[test]
#[ignore]
fn video_to_audio_extraction() {
    // This test validates Issue 1 (extract-audio preset / video→audio path)
    let engine = ConvxEngine::new();
    let input = fixture("sample.mp4");
    let output = temp_output("test_extract_audio.mp3");
    cleanup(&output);

    let result = engine.convert(&input, &output, ConversionOptions {
        output_format: Format::Mp3,
        overwrite: true,
        audio: Some(AudioOptions {
            bitrate: Some("192k".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    });

    assert!(result.is_ok(), "Video→Audio extraction failed: {:?}", result.err());
    assert!(output.exists());
    assert!(std::fs::metadata(&output).unwrap().len() > 0);
    cleanup(&output);
}

#[test]
#[ignore]
fn overwrite_false_rejects_existing_output() {
    let engine = ConvxEngine::new();
    let input = fixture("sample.png");
    let output = temp_output("test_overwrite_guard.webp");

    // Create the output file first
    std::fs::write(&output, b"existing").expect("write existing");

    let result = engine.convert(&input, &output, ConversionOptions {
        output_format: Format::WebP,
        overwrite: false,
        ..Default::default()
    });

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), convx::ConvxError::OutputAlreadyExists { .. }));
    cleanup(&output);
}

#[test]
#[ignore]
fn progress_callback_fires_for_video() {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    let engine = ConvxEngine::new();
    let input = fixture("sample.mp4");
    let output = temp_output("test_progress.webm");
    cleanup(&output);

    let call_count = Arc::new(AtomicU32::new(0));
    let count_clone = call_count.clone();

    let mut on_progress = move |_pct: f32| {
        count_clone.fetch_add(1, Ordering::SeqCst);
    };

    let result = engine.convert_with_progress(
        &input,
        &output,
        ConversionOptions {
            output_format: Format::Webm,
            overwrite: true,
            ..Default::default()
        },
        &mut on_progress,
        None,
    );

    assert!(result.is_ok());
    assert!(call_count.load(Ordering::SeqCst) > 0, "Progress callback never fired");
    cleanup(&output);
}
```

Update CI to run ignored tests after fixture generation. In `.github/workflows/ci.yml`, change the Test step:

```yaml
      - name: Test (unit)
        working-directory: convx-core
        run: cargo test

      - name: Test (integration)
        working-directory: convx-core
        run: cargo test -- --ignored
```

---

## Issue 7 — MCP server options leak (minor cleanup)

**Problem:** In `mcp_server.rs`, the `convert_file` tool handler creates both `ImageOptions` and `VideoOptions` regardless of input type:

```rust
let options = ConversionOptions {
    output_format,
    quality: p.quality,
    image: Some(ImageOptions {
        width: p.width,
        ..Default::default()
    }),
    video: Some(VideoOptions {
        width: p.width,
        fps: p.fps,
        ..Default::default()
    }),
    overwrite: p.overwrite.unwrap_or(false),
    ..Default::default()
};
```

This means every conversion carries empty `ImageOptions` and `VideoOptions` structs even when irrelevant. Doesn't cause bugs, but it's noisy and means `strip_metadata` is always false even when a preset says otherwise (because `Some(ImageOptions { ..Default })` overwrites the preset's image options during merge).

**Fix:** Only create the options that have actual values:

```rust
let has_image_opts = p.width.is_some();
let has_video_opts = p.width.is_some() || p.fps.is_some();

let options = ConversionOptions {
    output_format,
    quality: p.quality,
    image: if has_image_opts {
        Some(ImageOptions {
            width: p.width,
            ..Default::default()
        })
    } else {
        None
    },
    video: if has_video_opts {
        Some(VideoOptions {
            width: p.width,
            fps: p.fps,
            ..Default::default()
        })
    } else {
        None
    },
    overwrite: p.overwrite.unwrap_or(false),
    ..Default::default()
};
```

This way, when no explicit width/fps is passed, the preset's options flow through `resolve_options` unobstructed.

---

## Issue 8 — Watch mode graceful shutdown (polish)

**Problem:** `run_watch` in `watch.rs` loops forever on `rx.recv()`. Ctrl+C kills the process without cleanup — no "stopping watch" message, no explicit watcher drop.

**Files:** `convx-core/src/watch.rs`

**Fix:** Use `ctrlc` crate or a crossbeam channel with a timeout to check a shutdown flag.

Simpler approach — use `recv_timeout` and check a flag:

```rust
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub fn run_watch(engine: &ConvxEngine, opts: WatchRunOptions) -> anyhow::Result<()> {
    // ... existing validation ...

    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = shutdown.clone();

    ctrlc::set_handler(move || {
        shutdown_clone.store(true, Ordering::SeqCst);
    }).ok(); // Ignore if handler already set

    // ... existing watcher setup ...

    loop {
        // Use recv_timeout so we can check shutdown flag
        let evt = match rx.recv_timeout(Duration::from_millis(250)) {
            Ok(evt) => evt,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                if shutdown.load(Ordering::SeqCst) {
                    break;
                }
                continue;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        };

        // ... existing event handling ...
    }

    if !opts.json_output {
        println!("\nWatch stopped.");
    }

    Ok(())
}
```

Add `ctrlc` to `Cargo.toml`:

```toml
ctrlc = "3"
```

---

## Execution Order

1. **Issue 1** — Fix `extract-audio` preset (bug, user-facing, 30 min)
2. **Issue 4** — Implement image resize + strip_metadata (bug, user-facing, 45 min)
3. **Issue 3** — Unify `convert_with_progress` through trait dispatch (architecture, 30 min)
4. **Issue 2** — Deduplicate ffprobe into `utils/probe.rs` (code health, 45 min)
5. **Issue 5** — Remove or document `max_file_size` (cleanup, 10 min)
6. **Issue 7** — Fix MCP options leak (cleanup, 10 min)
7. **Issue 6** — Add integration tests (quality, 30 min)
8. **Issue 8** — Watch mode graceful shutdown (polish, 15 min)

Total: ~3.5 hours of focused work. After this, the codebase has zero known bugs, zero dead options, clean trait dispatch, one source of truth for probe logic, and real integration test coverage. Ship-ready.
