# convx Test Suite

**Purpose:** Verify the convx engine works correctly  
**Runner:** `cargo test` + integration tests  
**Fixtures:** Sample files for each format

---

## Test Fixtures

### Download Script

Create `tests/fixtures/download.sh`:

```bash
#!/bin/bash
# Download test fixtures for convx

FIXTURES_DIR="$(dirname "$0")"
cd "$FIXTURES_DIR"

echo "Downloading test fixtures..."

# Images
curl -L -o sample.png "https://www.w3.org/Graphics/PNG/nurbcup2si.png"
curl -L -o sample.jpg "https://www.w3.org/People/Raggett/Images/davephoto.jpg"
curl -L -o sample.gif "https://upload.wikimedia.org/wikipedia/commons/2/2c/Rotating_earth_%28large%29.gif"
curl -L -o sample.bmp "https://filesamples.com/samples/image/bmp/sample_640%C3%97426.bmp"
curl -L -o sample.webp "https://www.gstatic.com/webp/gallery/1.webp"

# Video (small samples)
curl -L -o sample.mp4 "https://filesamples.com/samples/video/mp4/sample_640x360.mp4"
curl -L -o sample.webm "https://filesamples.com/samples/video/webm/sample_640x360.webm"

# Audio
curl -L -o sample.mp3 "https://filesamples.com/samples/audio/mp3/sample3.mp3"
curl -L -o sample.wav "https://filesamples.com/samples/audio/wav/sample3.wav"
curl -L -o sample.flac "https://filesamples.com/samples/audio/flac/sample3.flac"

# Documents
curl -L -o sample.txt "https://filesamples.com/samples/document/txt/sample3.txt"

echo "Done. Fixtures downloaded to $FIXTURES_DIR"
ls -la
```

### Alternative: Generate fixtures programmatically

```rust
// tests/fixtures/generate.rs

use std::process::Command;

pub fn generate_test_image(path: &str, width: u32, height: u32) {
    // Use ImageMagick to create a test image
    Command::new("convert")
        .args([
            "-size", &format!("{}x{}", width, height),
            "xc:blue",
            "-fill", "white",
            "-draw", "circle 50,50 50,1",
            path
        ])
        .status()
        .expect("Failed to generate test image");
}

pub fn generate_test_video(path: &str, duration_secs: u32) {
    // Use FFmpeg to create a test video
    Command::new("ffmpeg")
        .args([
            "-f", "lavfi",
            "-i", &format!("testsrc=duration={}:size=320x240:rate=30", duration_secs),
            "-f", "lavfi",
            "-i", &format!("sine=frequency=440:duration={}", duration_secs),
            "-c:v", "libx264",
            "-c:a", "aac",
            "-y",
            path
        ])
        .status()
        .expect("Failed to generate test video");
}

pub fn generate_test_audio(path: &str, duration_secs: u32) {
    Command::new("ffmpeg")
        .args([
            "-f", "lavfi",
            "-i", &format!("sine=frequency=440:duration={}", duration_secs),
            "-y",
            path
        ])
        .status()
        .expect("Failed to generate test audio");
}
```

---

## Unit Tests

### Format Detection Tests

**File:** `tests/unit/format_test.rs`

```rust
use convx::{Format, FormatCategory};
use std::path::Path;

#[test]
fn test_format_from_extension() {
    assert_eq!(Format::from_extension("png"), Some(Format::Png));
    assert_eq!(Format::from_extension("PNG"), Some(Format::Png));
    assert_eq!(Format::from_extension("jpg"), Some(Format::Jpg));
    assert_eq!(Format::from_extension("jpeg"), Some(Format::Jpg));
    assert_eq!(Format::from_extension("mp4"), Some(Format::Mp4));
    assert_eq!(Format::from_extension("unknown"), None);
}

#[test]
fn test_format_detect_from_path() {
    assert_eq!(Format::detect(Path::new("image.png")), Some(Format::Png));
    assert_eq!(Format::detect(Path::new("video.mp4")), Some(Format::Mp4));
    assert_eq!(Format::detect(Path::new("audio.mp3")), Some(Format::Mp3));
    assert_eq!(Format::detect(Path::new("no_extension")), None);
}

#[test]
fn test_format_extension() {
    assert_eq!(Format::Png.extension(), "png");
    assert_eq!(Format::Jpg.extension(), "jpg");
    assert_eq!(Format::Mp4.extension(), "mp4");
}

#[test]
fn test_format_category() {
    assert_eq!(Format::Png.category(), FormatCategory::Image);
    assert_eq!(Format::Jpg.category(), FormatCategory::Image);
    assert_eq!(Format::WebP.category(), FormatCategory::Image);
    
    assert_eq!(Format::Mp4.category(), FormatCategory::Video);
    assert_eq!(Format::Webm.category(), FormatCategory::Video);
    
    assert_eq!(Format::Mp3.category(), FormatCategory::Audio);
    assert_eq!(Format::Wav.category(), FormatCategory::Audio);
    
    assert_eq!(Format::Pdf.category(), FormatCategory::Document);
    assert_eq!(Format::Docx.category(), FormatCategory::Document);
}

#[test]
fn test_all_formats_have_extensions() {
    let formats = [
        Format::Png, Format::Jpg, Format::WebP, Format::Gif,
        Format::Mp4, Format::Webm, Format::Mkv,
        Format::Mp3, Format::Wav, Format::Flac,
        Format::Pdf, Format::Txt, Format::Md,
    ];
    
    for format in formats {
        assert!(!format.extension().is_empty(), "{:?} has no extension", format);
    }
}
```

### Options Tests

**File:** `tests/unit/options_test.rs`

```rust
use convx::{ConversionOptions, ImageOptions, VideoOptions, Format};

#[test]
fn test_default_options() {
    let options = ConversionOptions::default();
    assert!(!options.overwrite);
    assert!(!options.preserve_metadata);
}

#[test]
fn test_image_options_defaults() {
    let options = ImageOptions::default();
    assert!(options.width.is_none());
    assert!(options.height.is_none());
    assert!(!options.strip_metadata);
}

#[test]
fn test_options_serialization() {
    let options = ConversionOptions {
        output_format: Format::WebP,
        quality: Some(80),
        ..Default::default()
    };
    
    let json = serde_json::to_string(&options).unwrap();
    let parsed: ConversionOptions = serde_json::from_str(&json).unwrap();
    
    assert_eq!(parsed.output_format, Format::WebP);
    assert_eq!(parsed.quality, Some(80));
}
```

### Error Tests

**File:** `tests/unit/error_test.rs`

```rust
use convx::{ConvxError, Format};

#[test]
fn test_error_display() {
    let error = ConvxError::FileNotFound {
        path: "/tmp/missing.png".into(),
    };
    assert!(error.to_string().contains("missing.png"));
}

#[test]
fn test_error_exit_codes() {
    assert_eq!(ConvxError::FileNotFound { path: "".into() }.exit_code(), 1);
    assert_eq!(ConvxError::PermissionDenied { path: "".into() }.exit_code(), 2);
    assert_eq!(ConvxError::UnsupportedConversion {
        from: Format::Png,
        to: Format::Mp4,
    }.exit_code(), 3);
}

#[test]
fn test_ffmpeg_not_found_error() {
    let error = ConvxError::FfmpegNotFound;
    assert!(error.to_string().contains("FFmpeg"));
}
```

---

## Integration Tests

### Image Conversion Tests

**File:** `tests/integration/image_test.rs`

```rust
use convx::{ConvxEngine, ConversionOptions, Format, ConversionStatus};
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
    let output = temp.path().join("output.webp");
    
    let options = ConversionOptions {
        output_format: Format::WebP,
        quality: Some(80),
        ..Default::default()
    };
    
    let result = engine.convert(input, &output, options).unwrap();
    
    assert_eq!(result.status, ConversionStatus::Completed);
    assert!(output.exists());
    assert!(result.output_size.unwrap() > 0);
}

#[test]
fn test_png_to_jpg() {
    let (engine, temp) = setup();
    let input = Path::new("tests/fixtures/sample.png");
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
fn test_jpg_to_png() {
    let (engine, temp) = setup();
    let input = Path::new("tests/fixtures/sample.jpg");
    let output = temp.path().join("output.png");
    
    let options = ConversionOptions {
        output_format: Format::Png,
        ..Default::default()
    };
    
    let result = engine.convert(input, &output, options).unwrap();
    
    assert_eq!(result.status, ConversionStatus::Completed);
    assert!(output.exists());
}

#[test]
fn test_webp_to_png() {
    let (engine, temp) = setup();
    let input = Path::new("tests/fixtures/sample.webp");
    let output = temp.path().join("output.png");
    
    let options = ConversionOptions {
        output_format: Format::Png,
        ..Default::default()
    };
    
    let result = engine.convert(input, &output, options).unwrap();
    
    assert_eq!(result.status, ConversionStatus::Completed);
}

#[test]
fn test_image_quality_affects_size() {
    let (engine, temp) = setup();
    let input = Path::new("tests/fixtures/sample.png");
    
    let output_high = temp.path().join("high.webp");
    let output_low = temp.path().join("low.webp");
    
    let result_high = engine.convert(input, &output_high, ConversionOptions {
        output_format: Format::WebP,
        quality: Some(95),
        ..Default::default()
    }).unwrap();
    
    let result_low = engine.convert(input, &output_low, ConversionOptions {
        output_format: Format::WebP,
        quality: Some(50),
        ..Default::default()
    }).unwrap();
    
    // Higher quality should produce larger file
    assert!(result_high.output_size.unwrap() > result_low.output_size.unwrap());
}
```

### Video Conversion Tests

**File:** `tests/integration/video_test.rs`

```rust
use convx::{ConvxEngine, ConversionOptions, VideoOptions, Format, ConversionStatus};
use std::path::Path;
use tempfile::TempDir;

fn setup() -> (ConvxEngine, TempDir) {
    let engine = ConvxEngine::new().expect("Failed to create engine");
    let temp = TempDir::new().expect("Failed to create temp dir");
    (engine, temp)
}

#[test]
fn test_mp4_to_webm() {
    let (engine, temp) = setup();
    let input = Path::new("tests/fixtures/sample.mp4");
    let output = temp.path().join("output.webm");
    
    let options = ConversionOptions {
        output_format: Format::Webm,
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
    let output = temp.path().join("output.gif");
    
    let options = ConversionOptions {
        output_format: Format::Gif,
        video: Some(VideoOptions {
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
fn test_video_resize() {
    let (engine, temp) = setup();
    let input = Path::new("tests/fixtures/sample.mp4");
    let output = temp.path().join("output.mp4");
    
    let options = ConversionOptions {
        output_format: Format::Mp4,
        video: Some(VideoOptions {
            width: Some(320),
            height: Some(240),
            ..Default::default()
        }),
        ..Default::default()
    };
    
    let result = engine.convert(input, &output, options).unwrap();
    
    assert_eq!(result.status, ConversionStatus::Completed);
    // Output should be smaller due to lower resolution
    assert!(result.output_size.unwrap() < result.input_size);
}

#[test]
fn test_video_crf_quality() {
    let (engine, temp) = setup();
    let input = Path::new("tests/fixtures/sample.mp4");
    
    let output_high = temp.path().join("high.mp4");
    let output_low = temp.path().join("low.mp4");
    
    // CRF 18 = high quality
    engine.convert(input, &output_high, ConversionOptions {
        output_format: Format::Mp4,
        video: Some(VideoOptions {
            crf: Some(18),
            ..Default::default()
        }),
        ..Default::default()
    }).unwrap();
    
    // CRF 35 = low quality
    engine.convert(input, &output_low, ConversionOptions {
        output_format: Format::Mp4,
        video: Some(VideoOptions {
            crf: Some(35),
            ..Default::default()
        }),
        ..Default::default()
    }).unwrap();
    
    // Higher CRF = smaller file
    let high_size = std::fs::metadata(&output_high).unwrap().len();
    let low_size = std::fs::metadata(&output_low).unwrap().len();
    assert!(high_size > low_size);
}
```

### Audio Conversion Tests

**File:** `tests/integration/audio_test.rs`

```rust
use convx::{ConvxEngine, ConversionOptions, AudioOptions, Format, ConversionStatus};
use std::path::Path;
use tempfile::TempDir;

fn setup() -> (ConvxEngine, TempDir) {
    let engine = ConvxEngine::new().expect("Failed to create engine");
    let temp = TempDir::new().expect("Failed to create temp dir");
    (engine, temp)
}

#[test]
fn test_mp3_to_wav() {
    let (engine, temp) = setup();
    let input = Path::new("tests/fixtures/sample.mp3");
    let output = temp.path().join("output.wav");
    
    let options = ConversionOptions {
        output_format: Format::Wav,
        ..Default::default()
    };
    
    let result = engine.convert(input, &output, options).unwrap();
    
    assert_eq!(result.status, ConversionStatus::Completed);
    assert!(output.exists());
    // WAV is uncompressed, should be larger
    assert!(result.output_size.unwrap() > result.input_size);
}

#[test]
fn test_wav_to_mp3() {
    let (engine, temp) = setup();
    let input = Path::new("tests/fixtures/sample.wav");
    let output = temp.path().join("output.mp3");
    
    let options = ConversionOptions {
        output_format: Format::Mp3,
        audio: Some(AudioOptions {
            bitrate: Some("192k".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };
    
    let result = engine.convert(input, &output, options).unwrap();
    
    assert_eq!(result.status, ConversionStatus::Completed);
    // MP3 is compressed, should be smaller
    assert!(result.output_size.unwrap() < result.input_size);
}

#[test]
fn test_flac_to_mp3() {
    let (engine, temp) = setup();
    let input = Path::new("tests/fixtures/sample.flac");
    let output = temp.path().join("output.mp3");
    
    let options = ConversionOptions {
        output_format: Format::Mp3,
        ..Default::default()
    };
    
    let result = engine.convert(input, &output, options).unwrap();
    
    assert_eq!(result.status, ConversionStatus::Completed);
}

#[test]
fn test_audio_bitrate_affects_size() {
    let (engine, temp) = setup();
    let input = Path::new("tests/fixtures/sample.wav");
    
    let output_high = temp.path().join("high.mp3");
    let output_low = temp.path().join("low.mp3");
    
    engine.convert(input, &output_high, ConversionOptions {
        output_format: Format::Mp3,
        audio: Some(AudioOptions {
            bitrate: Some("320k".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    }).unwrap();
    
    engine.convert(input, &output_low, ConversionOptions {
        output_format: Format::Mp3,
        audio: Some(AudioOptions {
            bitrate: Some("64k".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    }).unwrap();
    
    let high_size = std::fs::metadata(&output_high).unwrap().len();
    let low_size = std::fs::metadata(&output_low).unwrap().len();
    assert!(high_size > low_size);
}
```

### Error Handling Tests

**File:** `tests/integration/error_test.rs`

```rust
use convx::{ConvxEngine, ConversionOptions, Format, ConvxError};
use std::path::Path;
use tempfile::TempDir;

#[test]
fn test_file_not_found() {
    let engine = ConvxEngine::new().unwrap();
    let temp = TempDir::new().unwrap();
    
    let result = engine.convert(
        Path::new("/nonexistent/file.png"),
        &temp.path().join("output.webp"),
        ConversionOptions {
            output_format: Format::WebP,
            ..Default::default()
        },
    );
    
    assert!(matches!(result, Err(ConvxError::FileNotFound { .. })));
}

#[test]
fn test_unsupported_conversion() {
    let engine = ConvxEngine::new().unwrap();
    let temp = TempDir::new().unwrap();
    
    // PNG to MP4 is not supported
    let result = engine.convert(
        Path::new("tests/fixtures/sample.png"),
        &temp.path().join("output.mp4"),
        ConversionOptions {
            output_format: Format::Mp4,
            ..Default::default()
        },
    );
    
    assert!(matches!(result, Err(ConvxError::UnsupportedConversion { .. })));
}

#[test]
fn test_invalid_file() {
    let engine = ConvxEngine::new().unwrap();
    let temp = TempDir::new().unwrap();
    
    // Create a file with wrong content
    let fake_png = temp.path().join("fake.png");
    std::fs::write(&fake_png, b"not a png file").unwrap();
    
    let result = engine.convert(
        &fake_png,
        &temp.path().join("output.webp"),
        ConversionOptions {
            output_format: Format::WebP,
            ..Default::default()
        },
    );
    
    assert!(result.is_err());
}

#[test]
fn test_output_dir_not_writable() {
    let engine = ConvxEngine::new().unwrap();
    
    let result = engine.convert(
        Path::new("tests/fixtures/sample.png"),
        Path::new("/root/cannot_write_here.webp"),
        ConversionOptions {
            output_format: Format::WebP,
            ..Default::default()
        },
    );
    
    assert!(result.is_err());
}
```

---

## CLI Tests

**File:** `tests/cli/cli_test.rs`

```rust
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn convx() -> Command {
    Command::cargo_bin("convx").unwrap()
}

#[test]
fn test_help() {
    convx()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Local-first file conversion"));
}

#[test]
fn test_version() {
    convx()
        .arg("version")
        .assert()
        .success()
        .stdout(predicate::str::contains("convx"));
}

#[test]
fn test_formats_list() {
    convx()
        .arg("formats")
        .assert()
        .success()
        .stdout(predicate::str::contains("png"))
        .stdout(predicate::str::contains("mp4"))
        .stdout(predicate::str::contains("mp3"));
}

#[test]
fn test_convert_basic() {
    let temp = TempDir::new().unwrap();
    let output = temp.path().join("output.webp");
    
    convx()
        .args(["convert", "tests/fixtures/sample.png", output.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Converted"));
    
    assert!(output.exists());
}

#[test]
fn test_convert_with_quality() {
    let temp = TempDir::new().unwrap();
    let output = temp.path().join("output.webp");
    
    convx()
        .args([
            "convert",
            "tests/fixtures/sample.png",
            output.to_str().unwrap(),
            "--quality", "50"
        ])
        .assert()
        .success();
    
    assert!(output.exists());
}

#[test]
fn test_convert_with_format_flag() {
    let temp = TempDir::new().unwrap();
    let output = temp.path().join("output.jpg");
    
    convx()
        .args([
            "convert",
            "tests/fixtures/sample.png",
            "--to", "jpg",
            "-o", output.to_str().unwrap()
        ])
        .assert()
        .success();
    
    assert!(output.exists());
}

#[test]
fn test_convert_missing_input() {
    convx()
        .args(["convert", "/nonexistent/file.png", "output.webp"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found").or(predicate::str::contains("No such file")));
}
```

---

## Data Format Conversion Tests

### Test Fixtures (Data)

Create simple data fixtures in `tests/fixtures/`:

```
sample.csv:   name,age,score\nAlice,30,95.5\nBob,25,87.3\nCharlie,35,91.0
sample.tsv:   name\tage\tscore\nAlice\t30\t95.5\nBob\t25\t87.3\nCharlie\t35\t91.0
sample.json:  [{"name":"Alice","age":30,"score":95.5},{"name":"Bob","age":25,"score":87.3},{"name":"Charlie","age":35,"score":91.0}]
sample.jsonl: {"name":"Alice","age":30,"score":95.5}\n{"name":"Bob","age":25,"score":87.3}\n{"name":"Charlie","age":35,"score":91.0}
```

### Data Conversion Tests (Pure Rust — no external deps)

**File:** `tests/conversion_suite.rs` (appended)

```rust
#[test]
fn data_tsv_to_csv_succeeds() { /* TSV→CSV, assert output contains "Alice" and ',' */ }

#[test]
fn data_csv_to_tsv_succeeds() { /* CSV→TSV, assert output contains "Alice" and '\t' */ }

#[test]
fn data_jsonl_to_json_succeeds() { /* JSONL→JSON, assert output starts with '[' */ }

#[test]
fn data_json_to_jsonl_succeeds() { /* JSON→JSONL, assert 3 lines, each valid JSON */ }

#[test]
fn data_jsonl_to_csv_succeeds() { /* JSONL→CSV, assert output contains "name" and "Alice" */ }

#[test]
fn data_csv_to_jsonl_succeeds() { /* CSV→JSONL, assert 3 lines */ }

#[test]
fn data_csv_to_html_generates_table() { /* CSV→HTML, assert <table>, <th>, "Alice", "Generated by ConvX" */ }

#[test]
fn data_json_to_html_succeeds() { /* JSON→HTML, assert <table>, "Bob" */ }

#[test]
fn data_csv_to_markdown_generates_table() { /* CSV→Markdown, assert '|' and "---" and "Alice" */ }
```

### ML Format Tests (require Python deps — `#[ignore]`)

```rust
#[test]
#[ignore] // requires pandoc + weasyprint
fn data_csv_to_pdf_succeeds() { /* CSV→PDF, assert output exists and size > 0 */ }

#[test]
#[ignore] // requires pyarrow
fn data_parquet_to_csv_succeeds() { /* CSV→Parquet→CSV roundtrip, assert "Alice" preserved */ }
```

---

## Test Matrix

### Must Pass Before Ship

| Test | Command | Expected |
|------|---------|----------|
| PNG → WebP | `convx convert test.png --to webp` | Output exists, smaller size |
| PNG → JPG | `convx convert test.png --to jpg` | Output exists |
| JPG → PNG | `convx convert test.jpg --to png` | Output exists |
| WebP → PNG | `convx convert test.webp --to png` | Output exists |
| MP4 → WebM | `convx convert test.mp4 --to webm` | Output exists |
| MP4 → GIF | `convx convert test.mp4 --to gif` | Output exists, animated |
| MP3 → WAV | `convx convert test.mp3 --to wav` | Output exists |
| WAV → MP3 | `convx convert test.wav --to mp3` | Output exists, smaller size |
| Quality flag | `convx convert test.png --to webp -q 50` | Smaller than -q 90 |
| TSV → CSV | `convx convert test.tsv --to csv` | Output has commas, contains "Alice" |
| JSONL → JSON | `convx convert test.jsonl --to json` | Output starts with `[` |
| CSV → HTML | `convx convert test.csv --to html` | Output contains `<table>` |
| CSV → Markdown | `convx convert test.csv --to md` | Output contains `\|` and `---` |
| Missing file | `convx convert missing.png --to webp` | Error, exit code 1 |
| Bad format | `convx convert test.png --to xyz` | Error message |

### Run All Tests

```bash
# Download fixtures first
./tests/fixtures/download.sh

# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_png_to_webp

# Run integration tests only
cargo test --test integration

# Run CLI tests only
cargo test --test cli
```

---

## CI Configuration

**File:** `.github/workflows/test.yml`

```yaml
name: Test

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    
    steps:
      - uses: actions/checkout@v4
      
      - name: Install dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y ffmpeg libvips-tools pandoc
          pip install pyarrow numpy h5py weasyprint
      
      - name: Install Rust
        uses: dtolnay/rust-action@stable
      
      - name: Download test fixtures
        run: |
          chmod +x tests/fixtures/download.sh
          ./tests/fixtures/download.sh
      
      - name: Run tests
        run: cargo test --verbose
      
      - name: Build release
        run: cargo build --release
      
      - name: Test CLI
        run: |
          ./target/release/convx --help
          ./target/release/convx formats
```

---

## Coverage Requirements

- **Unit tests:** 80%+ line coverage on `src/types/`
- **Integration tests:** All conversion paths in the "Must Pass" matrix
- **CLI tests:** All subcommands have at least one test

Check coverage with:

```bash
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
open tarpaulin-report.html
```
