# ConvX Codebase Fix & Enhancement Prompt

> **Context:** You are working on ConvX (`/Users/jeffriebudde/convx/`), a Rust-based local-first file conversion tool with a CLI (`convx-core/`), a Tauri + Quasar desktop app (`convx-app/`), and planned MCP server support. The current implementation has a working core engine with 246 conversion paths across 28 formats (image via libvips, video/audio via FFmpeg), a functional CLI, and a functional desktop app. A comprehensive code review has identified critical bugs, architectural gaps, and missing features. This prompt covers everything that needs to be fixed, improved, or built — organized by priority tier.

---

## TIER 1: Critical Code Bugs & Correctness Issues

These are bugs in the current codebase that should be fixed before any new features.

---

### 1.1 — Remove Dead `Format::Jpeg` Variant

**File:** `convx-core/src/types/format.rs`

**Problem:** The `Format` enum has both `Jpg` and `Jpeg` as separate variants. However, `from_extension()` maps both `"jpg"` and `"jpeg"` to `Format::Jpg`, meaning `Format::Jpeg` is a dead variant that can never be constructed via normal detection paths. If someone pattern-matches on `Format::Jpeg` or constructs it manually, it will behave inconsistently — `extension()` returns `"jpg"` for both, but they are different enum values so `PartialEq` comparisons between a manually-constructed `Jpeg` and a detected `Jpg` would fail.

**Fix:**
- Remove `Jpeg` from the enum entirely.
- Update `extension()` to only handle `Jpg`.
- Audit everywhere `Jpeg` appears (including serde deserialization — someone could send `"jpeg"` in JSON and get a `Jpeg` variant due to `#[serde(rename_all = "lowercase")]`). Add a serde alias or custom deserializer so `"jpeg"` in JSON still deserializes to `Format::Jpg`.
- Grep the entire codebase for `Jpeg` references and remove/redirect them all.

---

### 1.2 — `ConvxEngine::new()` Should Be Infallible (or Actually Validate)

**File:** `convx-core/src/engine.rs`

**Problem:** `ConvxEngine::new()` returns `Result<Self, ConvxError>` but the body can never fail — it just constructs three zero-size unit structs (`ImageConverter`, `VideoConverter`, `AudioConverter`). This is misleading API design. Callers (including `main.rs` and Tauri's `lib.rs`) use `?` or `.expect()` on a function that cannot error.

**Fix — choose one of:**
- **Option A (preferred):** Make `new()` infallible. Change signature to `pub fn new() -> Self`. Remove the `Result` wrapper. Update all call sites.
- **Option B:** Move dependency checking into `new()` so it actually fails fast if FFmpeg/vips are missing, rather than failing at conversion time. This gives better DX — the user finds out at startup, not after they've picked a file and hit convert. Signature stays `Result<Self, ConvxError>` but now it has a real reason. If you go this route, also add a `ConvxEngine::new_unchecked() -> Self` escape hatch for contexts (like CI tests) where deps may not be present.

---

### 1.3 — CLI `formats` Command Hardcodes Format Strings

**File:** `convx-core/src/main.rs`, inside `Commands::Formats` handler.

**Problem:** The formats list is hardcoded as println strings:
```rust
println!("  Images: png, jpg, webp, gif, bmp, tiff, ico, svg, heic, heif, avif");
```
This means if you add a format to the `Format` enum, the CLI output won't update. This also doesn't match the Tauri command `get_supported_formats()` which has its own hardcoded list in `commands.rs`.

**Fix:**
- Add a method to `Format` (or a standalone function) that returns all formats, e.g.:
  ```rust
  impl Format {
      pub fn all() -> &'static [Format] { ... }
      pub fn all_by_category(cat: FormatCategory) -> Vec<Format> { ... }
  }
  ```
- Rewrite the CLI `Formats` handler to use this method, grouping by `category()`.
- Rewrite the Tauri `get_supported_formats()` command to use the same method.
- Single source of truth. Never hardcode format lists again.

---

### 1.4 — Path Traversal Vulnerability in Tauri Commands

**File:** `convx-app/src-tauri/src/commands.rs`

**Problem:** The `convert_file` command takes raw `String` paths from the JavaScript frontend and passes them directly to the engine:
```rust
pub async fn convert_file(..., input: String, output: String, ...) -> Result<...> {
    // ...
    let result = state.engine.convert(Path::new(&input), Path::new(&output), conv_options)
```
A compromised or malicious webview could pass paths like `../../etc/passwd` or `/System/...`. Similarly, `get_file_info`, `path_exists`, and `reveal_in_file_manager` all accept arbitrary paths without validation.

**Fix:**
- Add a path validation/sanitization function that:
  1. Canonicalizes the path (resolve symlinks, `..`, etc.)
  2. Verifies the resolved path is within allowed directories (e.g., user home, or a configurable allowlist)
  3. Rejects paths to sensitive system directories
- Apply this validation at the top of every Tauri command that accepts a file path.
- Consider using Tauri's built-in scope system (`fs` scope in `tauri.conf.json`) as an additional layer.

---

### 1.5 — Overwrite Flag Ordering in FFmpeg Args

**Files:** `convx-core/src/converters/video.rs`, `convx-core/src/converters/audio.rs`

**Problem:** The `-y` (overwrite) or `-n` (no overwrite) flag is added *after* the `-i input` argument:
```rust
let mut args: Vec<String> = vec![
    "-i".to_string(),
    input.to_string_lossy().to_string(),
];
if options.overwrite {
    args.push("-y".to_string());
} else {
    args.push("-n".to_string());
}
```
While FFmpeg is generally tolerant of flag ordering, the `-y`/`-n` global flags are conventionally placed before `-i`. Some FFmpeg builds or edge cases may behave unexpectedly. More importantly, this pushes the overwrite flag into the middle of the argument list, which makes debugging harder when you print the args.

**Fix:** Place `-y`/`-n` as the first argument, before `-i`:
```rust
let mut args: Vec<String> = Vec::new();
args.push(if options.overwrite { "-y" } else { "-n" }.to_string());
args.extend(["-i".to_string(), input.to_string_lossy().to_string()]);
```

---

## TIER 2: Architectural Improvements

These improve reliability, testability, and maintainability of the existing codebase.

---

### 2.1 — Replace Fake Progress Reporting with Real FFmpeg Progress

**Files:** `convx-app/src-tauri/src/commands.rs` (Tauri progress events), `convx-core/src/converters/video.rs`, `convx-core/src/converters/audio.rs`

**Problem:** The Tauri `convert_file` command emits fake progress stages at hardcoded percentages (5% → 60% → 90% → 100%). This is because the core engine uses `Command::new(ffmpeg).output()` which blocks until completion and provides no streaming progress. The user sees the progress bar jump from 5% to "done" with no intermediate updates. For large video files this can mean minutes of zero feedback.

**Fix — implement real progress reporting:**

1. **In the core engine converters (video.rs, audio.rs):** Switch from `.output()` to `.spawn()` with piped stderr. FFmpeg writes progress to stderr. Better yet, use FFmpeg's `-progress pipe:1` flag which outputs machine-readable progress lines to stdout:
   ```
   out_time_ms=5230000
   speed=2.1x
   progress=continue
   ```
   Parse these lines in a streaming loop and emit progress events.

2. **Add a progress callback to the engine API:**
   ```rust
   pub fn convert_with_progress(
       &self,
       input: &Path,
       output: &Path,
       options: ConversionOptions,
       on_progress: impl Fn(f32) + Send,  // 0.0 to 1.0
   ) -> Result<ConversionResult, ConvxError>;
   ```
   For video/audio: first probe the input duration (via `ffprobe -show_entries format=duration`), then compute `percent = current_time / total_duration` as FFmpeg reports progress.
   For images: libvips operations are typically fast enough that progress isn't needed, but emit a single 50% midpoint if desired.

3. **In the Tauri commands:** Replace the fake emit stages with real progress forwarding from the callback. Use `window.emit()` from within the callback closure.

4. **Add cancellation support:** With `spawn()` instead of `output()`, you get a `Child` process handle. Store it in a `Mutex<Option<Child>>` on `ConvxState`. Add a `cancel_conversion` Tauri command that calls `child.kill()`. The conversion loop should check a cancellation flag and return `ConvxError::Cancelled`.

---

### 2.2 — Add Converter Trait for Consistent Interface

**Files:** `convx-core/src/converters/mod.rs`, `image.rs`, `video.rs`, `audio.rs`

**Problem:** The three converters have matching method signatures but no shared trait. The engine does manual dispatch via match. This makes it harder to add new converters (document, archive, etc.) and prevents the plugin system from being trait-based.

**Fix:**
```rust
// converters/mod.rs
pub trait Converter: Send + Sync {
    fn can_convert(&self, from: Format, to: Format) -> bool;
    fn convert(
        &self,
        input: &Path,
        output: &Path,
        options: &ConversionOptions,
    ) -> Result<ConversionResult, ConvxError>;
}
```
- Implement `Converter` for `ImageConverter`, `VideoConverter`, `AudioConverter`.
- Store converters as `Vec<Box<dyn Converter>>` in the engine.
- The engine's `convert()` method iterates converters and dispatches to the first one where `can_convert()` returns true.
- This also enables the future plugin system: plugins just implement `Converter`.

---

### 2.3 — Expand Test Coverage

**File:** `convx-core/tests/integration.rs`, and new unit test files.

**Problem:** There are only 5 integration tests, all require pre-generated fixtures, and there are zero unit tests. Format detection, quality mapping, error handling, and edge cases are all untested.

**Fix — add the following test categories:**

**Unit tests (no fixtures needed, add as `#[cfg(test)] mod tests` blocks in each file):**

- `types/format.rs`:
  - `from_extension` for every supported extension including edge cases (`"jpeg"`, `"tif"`, `"aif"`, `"htm"`, `"markdown"`)
  - `from_extension` returns `None` for unknown extensions (`"xyz"`, `""`, `"UNKNOWN"`)
  - `category()` returns correct category for every variant
  - `extension()` round-trips: `Format::from_extension(f.extension()) == Some(f)` for all variants
  - `convertible_targets()` never includes self
  - `convertible_targets()` for an image format does not include `Svg` (raster→SVG is blocked)
  - `convertible_targets()` for a video format includes `Gif`
  - `convertible_targets()` for audio doesn't include video formats

- `converters/video.rs`:
  - `quality_to_crf` boundary values: quality 1 → CRF 35, quality 100 → CRF 18, quality 50 → ~CRF 26
  - `quality_to_vp9_crf` boundary values
  - `select_video_audio_codecs` returns correct codecs for each output format

- `converters/audio.rs`:
  - `quality_to_bitrate_kbps` boundary values: quality 1 → 96, quality 100 → 320

- `converters/image.rs`:
  - `quality_to_png_compression` boundary values: quality 1 → 9, quality 100 → 1

- `engine.rs`:
  - `can_convert` returns true for valid paths (image→image, video→video, video→gif, audio→audio)
  - `can_convert` returns false for invalid paths (image→video, audio→image, etc.)
  - `convert` returns `FileNotFound` for nonexistent input
  - `convert` returns `OutputAlreadyExists` when output exists and overwrite=false

**Integration tests (require fixtures — add fixture generation to CI):**
- Add a `tests/generate_fixtures.sh` script that creates test files via FFmpeg:
  ```bash
  ffmpeg -f lavfi -i testsrc=duration=1:size=640x480:rate=1 -frames:v 1 tests/fixtures/sample.png -y
  ffmpeg -f lavfi -i testsrc=duration=3:size=320x240:rate=30 -f lavfi -i sine=frequency=440:duration=3 -c:v libx264 -c:a aac tests/fixtures/sample.mp4 -y
  ffmpeg -f lavfi -i sine=frequency=440:duration=3 tests/fixtures/sample.wav -y
  ```
- Add a CI step that runs this script before `cargo test`.
- Add tests for:
  - Every image format pair: png→jpg, png→webp, jpg→png, webp→gif, etc. (at least 10 representative paths)
  - Video format conversions: mp4→webm, mp4→avi, mov→mp4
  - Audio format conversions: wav→flac, mp3→ogg, flac→aac
  - Quality parameter actually affects output file size (convert at quality 10 vs quality 90, assert different sizes)
  - Width parameter actually resizes output
  - Overwrite flag: test that overwrite=true overwrites, overwrite=false errors

---

### 2.4 — Proper Error Messages for End Users

**Files:** `convx-core/src/types/error.rs`, all converters

**Problem:** When FFmpeg or libvips fails, the error message is the raw stderr dump, which can be hundreds of lines of codec information, library versions, and encoder details. This is shown to users in the desktop app's error state.

**Fix:**
- In each converter, after checking `!status.status.success()`, parse the stderr to extract only the meaningful error line. FFmpeg's actual error is usually on the last line or the line containing "Error" or "Invalid" or "No such file or directory".
- Add a helper:
  ```rust
  fn extract_ffmpeg_error(stderr: &str) -> String {
      stderr.lines()
          .rev()
          .find(|line| {
              let lower = line.to_lowercase();
              lower.contains("error") || lower.contains("invalid") || lower.contains("no such")
          })
          .unwrap_or("Conversion failed (unknown error)")
          .trim()
          .to_string()
  }
  ```
- Use this in `ConvxError::ConversionFailed` instead of the raw stderr.
- Keep the full stderr available as a debug log (via `tracing::debug!`).

---

## TIER 3: MCP Server (Highest Priority New Feature)

This is the most strategically important feature to build. It differentiates ConvX from every other converter.

---

### 3.1 — Build the MCP Server

**New directory:** `convx-core/src/mcp/` (or a new crate `convx-mcp/`)

**What to build:** A Model Context Protocol server that exposes ConvX's conversion capabilities as tools that AI agents (Claude, Cursor, Windsurf, custom agents via Claude Code, etc.) can invoke.

**Architecture:**
- The MCP server should be a separate binary target in the workspace, or a feature-gated module in `convx-core`.
- It communicates via stdin/stdout JSON-RPC (standard MCP transport).
- It imports and uses `ConvxEngine` directly — no shelling out.

**MCP Tools to expose:**

```json
{
  "tools": [
    {
      "name": "convert_file",
      "description": "Convert a file from one format to another. Supports images (png, jpg, webp, gif, heic, avif, etc.), video (mp4, mov, webm, avi, mkv, etc.), and audio (mp3, wav, flac, m4a, aac, etc.). All processing is local — files never leave the machine.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "input_path": { "type": "string", "description": "Absolute path to the input file" },
          "output_format": { "type": "string", "description": "Target format extension (e.g., 'webp', 'mp4', 'mp3')" },
          "output_path": { "type": "string", "description": "Optional. Absolute path for output file. Defaults to input path with new extension." },
          "quality": { "type": "integer", "minimum": 1, "maximum": 100, "description": "Optional. Quality level (1-100). Higher is better quality/larger file." },
          "width": { "type": "integer", "description": "Optional. Output width in pixels (maintains aspect ratio)." },
          "fps": { "type": "number", "description": "Optional. Frames per second (for GIF output)." },
          "overwrite": { "type": "boolean", "description": "Optional. Overwrite output file if it exists. Default: false." }
        },
        "required": ["input_path", "output_format"]
      }
    },
    {
      "name": "get_supported_formats",
      "description": "List all supported file formats grouped by category (image, video, audio).",
      "inputSchema": { "type": "object", "properties": {} }
    },
    {
      "name": "get_conversion_targets",
      "description": "Get all formats that a given input format can be converted to.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "input_format": { "type": "string", "description": "Input format extension (e.g., 'png', 'mp4')" }
        },
        "required": ["input_format"]
      }
    },
    {
      "name": "can_convert",
      "description": "Check if a specific conversion path is supported.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "from": { "type": "string", "description": "Source format extension" },
          "to": { "type": "string", "description": "Target format extension" }
        },
        "required": ["from", "to"]
      }
    },
    {
      "name": "get_file_info",
      "description": "Get information about a file: name, size, format, and what it can be converted to.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "path": { "type": "string", "description": "Absolute path to the file" }
        },
        "required": ["path"]
      }
    },
    {
      "name": "batch_convert",
      "description": "Convert multiple files at once. Provide a list of input paths and a target format. Output files are placed alongside inputs with new extensions, or in a specified output directory.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "input_paths": { "type": "array", "items": { "type": "string" }, "description": "List of absolute file paths to convert" },
          "output_format": { "type": "string", "description": "Target format for all files" },
          "output_directory": { "type": "string", "description": "Optional. Directory for output files." },
          "quality": { "type": "integer", "minimum": 1, "maximum": 100 },
          "overwrite": { "type": "boolean" }
        },
        "required": ["input_paths", "output_format"]
      }
    },
    {
      "name": "check_dependencies",
      "description": "Check if required system dependencies (FFmpeg, libvips) are installed and return version information.",
      "inputSchema": { "type": "object", "properties": {} }
    }
  ]
}
```

**Implementation approach:**

1. Use the `rmcp` crate (Rust MCP SDK) or implement the JSON-RPC protocol manually over stdin/stdout. The protocol is simple: read JSON-RPC requests from stdin, dispatch to tool handlers, write JSON-RPC responses to stdout.

2. Each tool handler is a thin wrapper around existing `ConvxEngine` methods:
   ```rust
   async fn handle_convert_file(engine: &ConvxEngine, params: ConvertParams) -> Result<Value, McpError> {
       let input = PathBuf::from(&params.input_path);
       let format = Format::from_extension(&params.output_format)
           .ok_or(McpError::invalid_params("Unknown format"))?;
       let output = params.output_path
           .map(PathBuf::from)
           .unwrap_or_else(|| {
               let mut p = input.clone();
               p.set_extension(format.extension());
               p
           });
       let options = ConversionOptions {
           output_format: format,
           quality: params.quality.map(|q| q as u8),
           video: Some(VideoOptions {
               fps: params.fps,
               width: params.width,
               ..Default::default()
           }),
           overwrite: params.overwrite.unwrap_or(false),
           ..Default::default()
       };
       let result = engine.convert(&input, &output, options)
           .map_err(|e| McpError::internal(e.to_string()))?;
       Ok(serde_json::to_value(result)?)
   }
   ```

3. Add a new binary target:
   ```toml
   # In convx-core/Cargo.toml
   [[bin]]
   name = "convx-mcp"
   path = "src/mcp/main.rs"
   ```

4. The MCP server binary:
   ```rust
   // src/mcp/main.rs
   fn main() {
       let engine = ConvxEngine::new();
       // Start JSON-RPC server on stdin/stdout
       // Register tool handlers
       // Run event loop
   }
   ```

5. Add a CLI subcommand to start the MCP server:
   ```bash
   convx mcp  # starts MCP server on stdin/stdout
   ```
   This lets users configure it in their MCP client config:
   ```json
   {
     "mcpServers": {
       "convx": {
         "command": "convx",
         "args": ["mcp"]
       }
     }
   }
   ```

---

## TIER 4: High-Value Feature Additions

These are features that significantly increase the product's value proposition.

---

### 4.1 — Batch Conversion in CLI

**File:** `convx-core/src/main.rs`

**Problem:** The CLI can only convert one file at a time. Users want `convx convert *.png --to webp`.

**Implementation:**
1. Change the `input` argument in the `Convert` command from `PathBuf` to `Vec<PathBuf>`:
   ```rust
   Convert {
       /// Input file(s) or glob pattern
       #[arg(required = true)]
       input: Vec<PathBuf>,
       // ...
   }
   ```
2. If multiple inputs are provided and `--output` is a directory (or not specified), iterate and convert each.
3. Add a `--jobs` / `-j` flag for parallelism (use `rayon` or `tokio::spawn`).
4. Add a `--output-dir` / `-d` flag to specify output directory for batch.
5. Print a summary table at the end:
   ```
   ✓ Converted 47 files in 3.2s
     Total: 128.4 MB → 34.2 MB (73% smaller)
     Failed: 0
   ```
6. Add glob expansion for shells that don't expand (Windows):
   ```rust
   let inputs: Vec<PathBuf> = cli_inputs.iter()
       .flat_map(|p| {
           glob::glob(&p.to_string_lossy())
               .into_iter()
               .flatten()
               .filter_map(Result::ok)
       })
       .collect();
   ```
   Add `glob = "0.3"` to dependencies.

---

### 4.2 — Presets System

**New files:** `convx-core/src/types/preset.rs`, `convx-core/src/presets/` directory

**Implementation:**

1. Define the preset type:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct Preset {
       pub name: String,
       pub description: String,
       pub output_format: Format,
       pub quality: Option<u8>,
       pub max_file_size: Option<u64>,  // bytes
       pub video: Option<VideoOptions>,
       pub audio: Option<AudioOptions>,
       pub image: Option<ImageOptions>,
   }
   ```

2. Embed built-in presets (at minimum these high-value ones):
   - `discord` — mp4, h264, CRF 28, target <8MB
   - `discord-nitro` — mp4, h264, CRF 23, target <50MB
   - `twitter-image` — jpg, quality 85, 1200px wide
   - `twitter-gif` — gif, 480px wide, 15fps
   - `instagram-story` — mp4, 1080x1920, 30fps
   - `web-image` — webp, quality 80, strip metadata
   - `email-friendly` — jpg, quality 75, 1200px wide, target <1MB
   - `heic-to-jpg` — jpg, quality 90
   - `archive-lossless` — png, max compression
   - `extract-audio` — mp3, 192kbps

3. Add `--preset` / `-p` flag to CLI:
   ```bash
   convx convert video.mov --preset discord
   convx convert video.mov -p discord
   ```

4. Add `convx presets` subcommand to list available presets:
   ```bash
   convx presets list
   convx presets show discord
   ```

5. In the engine, preset resolution overrides individual options:
   ```rust
   pub fn resolve_options(options: ConversionOptions, preset: Option<&str>) -> Result<ConversionOptions, ConvxError> {
       if let Some(preset_name) = preset {
           let preset = get_preset(preset_name)?;
           // Merge: preset values are defaults, explicit CLI options override
       }
       Ok(options)
   }
   ```

---

### 4.3 — Watch Mode

**New file:** `convx-core/src/watch.rs`

**Implementation:**
1. Add `notify = "6"` to Cargo.toml dependencies.
2. Add a `Watch` CLI subcommand:
   ```bash
   convx watch ~/Downloads --to webp --filter "*.png,*.jpg"
   convx watch ~/Screenshots --to webp
   convx watch ~/Photos --to jpg --preset heic-to-jpg
   ```
3. Use the `notify` crate to watch for new/modified files:
   ```rust
   use notify::{Watcher, RecursiveMode, watcher};
   // On file create/modify event → check if extension matches filter → convert
   ```
4. Debounce events (500ms default, configurable with `--debounce`).
5. Print each conversion as it happens. Ctrl+C to stop.

---

### 4.4 — `convx info` Command

**Addition to:** `convx-core/src/main.rs`

**Implementation:** Add an `Info` subcommand that probes a file and shows detailed metadata:
```bash
convx info video.mp4
```
Output:
```
File:       video.mp4
Format:     MP4 (Video)
Size:       45.2 MB
Duration:   2:34
Resolution: 1920x1080
FPS:        30
Video:      H.264 (libx264)
Audio:      AAC, 48000 Hz, stereo
Converts to: mov, webm, avi, mkv, wmv, flv, m4v, gif
```

Use `ffprobe -v quiet -print_format json -show_format -show_streams` for video/audio metadata.
Use `vips header` or similar for image metadata.

This is useful both for users and as an MCP tool.

---

## TIER 5: Quality of Life & Polish

---

### 5.1 — Dynamic Formats Command Using Enum

**File:** `convx-core/src/main.rs`

Replace the hardcoded formats output with a dynamic listing. After implementing `Format::all_by_category()` from fix 1.3, update the handler:

```rust
Commands::Formats { from, category } => {
    if let Some(from_ext) = from {
        let format = Format::from_extension(&from_ext)
            .expect("Unknown format");
        let targets = format.convertible_targets();
        println!("{} can convert to:", from_ext);
        for t in targets {
            println!("  {}", t.extension());
        }
    } else {
        for cat in &[FormatCategory::Image, FormatCategory::Video, FormatCategory::Audio] {
            let fmts = Format::all_by_category(*cat);
            let names: Vec<&str> = fmts.iter().map(|f| f.extension()).collect();
            println!("  {:?}: {}", cat, names.join(", "));
        }
    }
}
```

Add optional flags:
```bash
convx formats                     # list all
convx formats --from png          # what can PNG convert to?
convx formats --category video    # list video formats only
```

---

### 5.2 — Add `--json` Output Flag to CLI

**File:** `convx-core/src/main.rs`

Add a global `--json` flag that switches all output to JSON. This is critical for scripting and integration:
```bash
convx convert image.png --to webp --json
```
Output:
```json
{
  "status": "completed",
  "input": "image.png",
  "output": "image.webp",
  "input_size": 1234567,
  "output_size": 345678,
  "space_saved_percent": -72.0,
  "duration_ms": 234
}
```

The `ConversionResult` struct already derives `Serialize`, so this is straightforward:
```rust
if cli.json {
    println!("{}", serde_json::to_string_pretty(&result)?);
} else {
    // existing human-readable output
}
```

---

### 5.3 — CI Pipeline with Tests

**File:** `.github/workflows/ci.yml` (new)

Create a CI workflow that:
1. Installs FFmpeg and libvips on the runner
2. Generates test fixtures
3. Runs `cargo test`
4. Runs `cargo clippy`
5. Runs `cargo fmt --check`

```yaml
name: CI
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt
      - name: Install system deps
        run: |
          sudo apt-get update
          sudo apt-get install -y ffmpeg libvips-tools
      - name: Generate test fixtures
        working-directory: convx-core
        run: |
          mkdir -p tests/fixtures
          ffmpeg -f lavfi -i testsrc=duration=1:size=640x480:rate=1 -frames:v 1 tests/fixtures/sample.png -y
          ffmpeg -f lavfi -i testsrc=duration=3:size=320x240:rate=30 -f lavfi -i sine=frequency=440:duration=3 -c:v libx264 -c:a aac tests/fixtures/sample.mp4 -y
          ffmpeg -f lavfi -i sine=frequency=440:duration=3 tests/fixtures/sample.wav -y
      - name: Check formatting
        working-directory: convx-core
        run: cargo fmt --check
      - name: Clippy
        working-directory: convx-core
        run: cargo clippy -- -D warnings
      - name: Test
        working-directory: convx-core
        run: cargo test
```

---

### 5.4 — Improve the README

Update the README to:
1. Lead with the three-surface pitch: CLI + Desktop + MCP
2. Add a "Quick Convert" section showing the 5 most common conversions
3. Add an MCP configuration section showing how to add ConvX to Claude Desktop / Cursor / etc.
4. Add a presets section (once built)
5. Add badges for CI status, crate version, license
6. Remove the roadmap that reveals how much isn't built yet — replace with a compact "Coming Soon" section

---

## TIER 6: Structural Cleanup

---

### 6.1 — Workspace Cargo.toml

**Problem:** The repo has two separate Cargo projects (`convx-core` and `convx-app/src-tauri`) with a path dependency between them but no workspace root.

**Fix:** Add a root `Cargo.toml` workspace:
```toml
[workspace]
members = ["convx-core", "convx-app/src-tauri"]
resolver = "2"
```
This enables `cargo build --workspace`, `cargo test --workspace`, shared target directories, and consistent dependency versions.

---

### 6.2 — Decouple Document Formats from Active Code

**File:** `convx-core/src/types/format.rs`

**Problem:** The `Format` enum includes `Pdf`, `Docx`, `Txt`, `Md`, `Html` but no converter handles documents. `from_extension` will detect these formats, `convertible_targets()` will return an empty vec for them, and a user trying to convert a PDF will get a confusing `UnsupportedConversion` error.

**Fix — choose one:**
- **Option A:** Remove document variants from the enum until document conversion is implemented. Keep them in a comment or feature-gated behind `#[cfg(feature = "documents")]`.
- **Option B (preferred):** Keep them in the enum but add a clear error path. In `ConvxEngine::convert()`, if the input is a document format, return a specific error like `ConvxError::DocumentConversionNotYetSupported` with a message like "Document conversion coming soon. Track progress at github.com/...".

---

## Summary of All Changes

| # | File(s) | Change | Priority |
|---|---------|--------|----------|
| 1.1 | `format.rs` | Remove dead `Jpeg` variant | Critical |
| 1.2 | `engine.rs`, callers | Make `new()` infallible or add real validation | Critical |
| 1.3 | `format.rs`, `main.rs`, `commands.rs` | Dynamic format listing, single source of truth | Critical |
| 1.4 | `commands.rs` | Path validation for Tauri commands | Critical |
| 1.5 | `video.rs`, `audio.rs` | Fix FFmpeg flag ordering | Critical |
| 2.1 | Converters, Tauri commands | Real progress reporting + cancellation | High |
| 2.2 | `converters/mod.rs`, all converters | Add `Converter` trait | High |
| 2.3 | `tests/`, new test files | Comprehensive unit + integration tests | High |
| 2.4 | `error.rs`, converters | Human-readable FFmpeg error extraction | High |
| 3.1 | New `mcp/` module or crate | Full MCP server | **Highest** |
| 4.1 | `main.rs`, Cargo.toml | Batch conversion with glob + parallelism | High |
| 4.2 | New preset files, `main.rs` | Presets system with 10+ built-ins | Medium |
| 4.3 | New `watch.rs`, `main.rs` | Watch mode for auto-conversion | Medium |
| 4.4 | `main.rs` | `convx info` command using ffprobe | Medium |
| 5.1 | `main.rs` | Dynamic `convx formats` with flags | Low |
| 5.2 | `main.rs` | Global `--json` output flag | Low |
| 5.3 | `.github/workflows/ci.yml` | CI pipeline with tests | Medium |
| 5.4 | `README.md` | Rewrite with MCP focus | Medium |
| 6.1 | Root `Cargo.toml` | Workspace setup | Low |
| 6.2 | `format.rs`, `engine.rs` | Handle document format dead-end gracefully | Low |

---

## Execution Order Recommendation

The recommended order balances quick wins, risk reduction, and strategic value:

1. **Tier 1 bugs** (1.1–1.5) — 1-2 hours total. Clean foundation.
2. **MCP Server** (3.1) — 1-2 days. Highest strategic value.
3. **Batch conversion** (4.1) — Half day. Most requested power-user feature.
4. **Converter trait** (2.2) — 1-2 hours. Architectural cleanup that enables everything else.
5. **Test expansion** (2.3) + **CI** (5.3) — Half day. Safety net for all future work.
6. **Real progress + cancellation** (2.1) — 1 day. Desktop app polish.
7. **Presets** (4.2) — Half day. High-value, marketable feature.
8. **Error messages** (2.4) — 1-2 hours. UX polish.
9. **Remaining Tier 5-6 items** — As time allows.
