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
    assert!(
        path.exists(),
        "Output file does not exist: {}",
        path.display()
    );
    let size = std::fs::metadata(path).expect("metadata").len();
    assert!(size > 0, "Output file is empty: {}", path.display());
}

#[test]
#[ignore]
fn png_to_webp() {
    let engine = ConvxEngine::new();
    let temp = TempDir::new().unwrap();
    let input = fixtures_dir().join("sample.png");
    let output = temp.path().join("output.webp");

    let result = engine
        .convert(
            &input,
            &output,
            ConversionOptions {
                output_format: Format::WebP,
                quality: Some(80),
                ..Default::default()
            },
        )
        .expect("conversion should succeed");

    assert_file_nonempty(&output);
    assert!(result.output_size.unwrap_or(0) > 0);
    assert!(result.duration_ms < 30_000);
}

#[test]
#[ignore]
fn png_to_jpg_with_quality() {
    let engine = ConvxEngine::new();
    let temp = TempDir::new().unwrap();
    let input = fixtures_dir().join("sample.png");
    let output = temp.path().join("output.jpg");

    let result = engine
        .convert(
            &input,
            &output,
            ConversionOptions {
                output_format: Format::Jpg,
                quality: Some(50),
                ..Default::default()
            },
        )
        .expect("conversion should succeed");

    assert_file_nonempty(&output);
    assert!(result.output_size.unwrap_or(0) > 0);
}

#[test]
#[ignore]
fn mp4_to_webm() {
    let engine = ConvxEngine::new();
    let temp = TempDir::new().unwrap();
    let input = fixtures_dir().join("sample.mp4");
    let output = temp.path().join("output.webm");

    engine
        .convert(
            &input,
            &output,
            ConversionOptions {
                output_format: Format::Webm,
                ..Default::default()
            },
        )
        .expect("conversion should succeed");

    assert_file_nonempty(&output);
}

#[test]
#[ignore]
fn mp4_to_gif() {
    let engine = ConvxEngine::new();
    let temp = TempDir::new().unwrap();
    let input = fixtures_dir().join("sample.mp4");
    let output = temp.path().join("output.gif");

    engine
        .convert(
            &input,
            &output,
            ConversionOptions {
                output_format: Format::Gif,
                ..Default::default()
            },
        )
        .expect("conversion should succeed");

    assert_file_nonempty(&output);
}

#[test]
#[ignore]
fn wav_to_mp3() {
    let engine = ConvxEngine::new();
    let temp = TempDir::new().unwrap();
    let input = fixtures_dir().join("sample.wav");
    let output = temp.path().join("output.mp3");

    engine
        .convert(
            &input,
            &output,
            ConversionOptions {
                output_format: Format::Mp3,
                quality: Some(80),
                ..Default::default()
            },
        )
        .expect("conversion should succeed");

    assert_file_nonempty(&output);
}

#[test]
#[ignore]
fn wav_to_flac() {
    let engine = ConvxEngine::new();
    let temp = TempDir::new().unwrap();
    let input = fixtures_dir().join("sample.wav");
    let output = temp.path().join("output.flac");

    engine
        .convert(
            &input,
            &output,
            ConversionOptions {
                output_format: Format::Flac,
                ..Default::default()
            },
        )
        .expect("conversion should succeed");

    assert_file_nonempty(&output);
}

#[test]
#[ignore]
fn mp4_to_mp3_extract_audio() {
    let engine = ConvxEngine::new();
    let temp = TempDir::new().unwrap();
    let input = fixtures_dir().join("sample.mp4");
    let output = temp.path().join("extracted.mp3");

    engine
        .convert(
            &input,
            &output,
            ConversionOptions {
                output_format: Format::Mp3,
                ..Default::default()
            },
        )
        .expect("audio extraction should succeed");

    assert_file_nonempty(&output);
}

#[test]
#[ignore]
fn overwrite_false_rejects_existing_output() {
    let engine = ConvxEngine::new();
    let temp = TempDir::new().unwrap();
    let input = fixtures_dir().join("sample.png");
    let output = temp.path().join("output.webp");

    engine
        .convert(
            &input,
            &output,
            ConversionOptions {
                output_format: Format::WebP,
                overwrite: false,
                ..Default::default()
            },
        )
        .expect("first conversion should succeed");

    let result = engine.convert(
        &input,
        &output,
        ConversionOptions {
            output_format: Format::WebP,
            overwrite: false,
            ..Default::default()
        },
    );

    assert!(result.is_err());
}

#[test]
#[ignore]
fn overwrite_true_replaces_existing_output() {
    let engine = ConvxEngine::new();
    let temp = TempDir::new().unwrap();
    let input = fixtures_dir().join("sample.png");
    let output = temp.path().join("output.webp");

    engine
        .convert(
            &input,
            &output,
            ConversionOptions {
                output_format: Format::WebP,
                overwrite: false,
                ..Default::default()
            },
        )
        .expect("first conversion");

    engine
        .convert(
            &input,
            &output,
            ConversionOptions {
                output_format: Format::WebP,
                overwrite: true,
                ..Default::default()
            },
        )
        .expect("overwrite should succeed");
}
