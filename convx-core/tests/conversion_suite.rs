use convx::{
    AudioOptions, ConversionOptions, ConversionStatus, ConvxEngine, ConvxError, Format,
    VideoOptions,
};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn setup() -> (ConvxEngine, TempDir) {
    let engine = ConvxEngine::new();
    let temp = TempDir::new().expect("Failed to create temp dir");
    (engine, temp)
}

fn fixture_path(name: &str) -> PathBuf {
    let path = Path::new("tests/fixtures").join(name);
    assert!(
        path.exists(),
        "Missing required test fixture: {}. Generate fixtures before running this suite.",
        path.display()
    );
    path
}

#[test]
fn image_png_to_webp_succeeds() {
    let (engine, temp) = setup();
    let input = fixture_path("sample.png");
    let output = temp.path().join("output.webp");

    let options = ConversionOptions {
        output_format: Format::WebP,
        quality: Some(82),
        ..Default::default()
    };

    let result = engine.convert(&input, &output, options).unwrap();
    assert_eq!(result.status, ConversionStatus::Completed);
    assert!(output.exists());
}

#[test]
fn video_mp4_to_gif_succeeds() {
    let (engine, temp) = setup();
    let input = fixture_path("sample.mp4");
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

    let result = engine.convert(&input, &output, options).unwrap();
    assert_eq!(result.status, ConversionStatus::Completed);
    assert!(output.exists());
}

#[test]
fn audio_wav_to_mp3_succeeds() {
    let (engine, temp) = setup();
    let input = fixture_path("sample.wav");
    let output = temp.path().join("output.mp3");

    let options = ConversionOptions {
        output_format: Format::Mp3,
        audio: Some(AudioOptions {
            bitrate: Some("192k".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = engine.convert(&input, &output, options).unwrap();
    assert_eq!(result.status, ConversionStatus::Completed);
    assert!(output.exists());
}

#[test]
fn raster_to_svg_is_rejected_as_unsupported() {
    let (engine, temp) = setup();
    let input = fixture_path("sample.png");
    let output = temp.path().join("output.svg");

    let options = ConversionOptions {
        output_format: Format::Svg,
        ..Default::default()
    };

    let result = engine.convert(&input, &output, options);
    assert!(matches!(
        result,
        Err(ConvxError::UnsupportedConversion {
            from: Format::Png,
            to: Format::Svg
        })
    ));
}

#[test]
fn overwrite_policy_is_enforced() {
    let (engine, temp) = setup();
    let input = fixture_path("sample.png");
    let output = temp.path().join("existing.webp");

    std::fs::write(&output, b"already-here").unwrap();

    let no_overwrite = ConversionOptions {
        output_format: Format::WebP,
        overwrite: false,
        ..Default::default()
    };

    let result = engine.convert(&input, &output, no_overwrite);
    assert!(matches!(
        result,
        Err(ConvxError::OutputAlreadyExists { .. })
    ));

    let overwrite = ConversionOptions {
        output_format: Format::WebP,
        overwrite: true,
        ..Default::default()
    };

    let result = engine.convert(&input, &output, overwrite).unwrap();
    assert_eq!(result.status, ConversionStatus::Completed);
}

// ─── Data format conversion tests ────────────────────────────────

#[test]
fn data_tsv_to_csv_succeeds() {
    let (engine, temp) = setup();
    let input = fixture_path("sample.tsv");
    let output = temp.path().join("output.csv");

    let result = engine
        .convert(
            &input,
            &output,
            ConversionOptions {
                output_format: Format::Csv,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(result.status, ConversionStatus::Completed);
    assert!(output.exists());
    let content = std::fs::read_to_string(&output).unwrap();
    assert!(content.contains("Alice"));
    assert!(content.contains(','));
}

#[test]
fn data_csv_to_tsv_succeeds() {
    let (engine, temp) = setup();
    let input = fixture_path("sample.csv");
    let output = temp.path().join("output.tsv");

    let result = engine
        .convert(
            &input,
            &output,
            ConversionOptions {
                output_format: Format::Tsv,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(result.status, ConversionStatus::Completed);
    let content = std::fs::read_to_string(&output).unwrap();
    assert!(content.contains("Alice"));
    assert!(content.contains('\t'));
}

#[test]
fn data_jsonl_to_json_succeeds() {
    let (engine, temp) = setup();
    let input = fixture_path("sample.jsonl");
    let output = temp.path().join("output.json");

    let result = engine
        .convert(
            &input,
            &output,
            ConversionOptions {
                output_format: Format::Json,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(result.status, ConversionStatus::Completed);
    let content = std::fs::read_to_string(&output).unwrap();
    assert!(content.contains("Alice"));
    assert!(content.starts_with('['));
}

#[test]
fn data_json_to_jsonl_succeeds() {
    let (engine, temp) = setup();
    let input = fixture_path("sample.json");
    let output = temp.path().join("output.jsonl");

    let result = engine
        .convert(
            &input,
            &output,
            ConversionOptions {
                output_format: Format::Jsonl,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(result.status, ConversionStatus::Completed);
    let content = std::fs::read_to_string(&output).unwrap();
    let lines: Vec<&str> = content.trim().lines().collect();
    assert_eq!(lines.len(), 3);
    // Each line should be valid JSON
    for line in &lines {
        assert!(serde_json::from_str::<serde_json::Value>(line).is_ok());
    }
}

#[test]
fn data_jsonl_to_csv_succeeds() {
    let (engine, temp) = setup();
    let input = fixture_path("sample.jsonl");
    let output = temp.path().join("output.csv");

    let result = engine
        .convert(
            &input,
            &output,
            ConversionOptions {
                output_format: Format::Csv,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(result.status, ConversionStatus::Completed);
    let content = std::fs::read_to_string(&output).unwrap();
    assert!(content.contains("name"));
    assert!(content.contains("Alice"));
}

#[test]
fn data_csv_to_jsonl_succeeds() {
    let (engine, temp) = setup();
    let input = fixture_path("sample.csv");
    let output = temp.path().join("output.jsonl");

    let result = engine
        .convert(
            &input,
            &output,
            ConversionOptions {
                output_format: Format::Jsonl,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(result.status, ConversionStatus::Completed);
    let content = std::fs::read_to_string(&output).unwrap();
    let lines: Vec<&str> = content.trim().lines().collect();
    assert_eq!(lines.len(), 3);
}

#[test]
fn data_csv_to_html_generates_table() {
    let (engine, temp) = setup();
    let input = fixture_path("sample.csv");
    let output = temp.path().join("output.html");

    let result = engine
        .convert(
            &input,
            &output,
            ConversionOptions {
                output_format: Format::Html,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(result.status, ConversionStatus::Completed);
    let content = std::fs::read_to_string(&output).unwrap();
    assert!(content.contains("<table>"));
    assert!(content.contains("<th>"));
    assert!(content.contains("Alice"));
    assert!(content.contains("Generated by ConvX"));
}

#[test]
fn data_json_to_html_succeeds() {
    let (engine, temp) = setup();
    let input = fixture_path("sample.json");
    let output = temp.path().join("output.html");

    let result = engine
        .convert(
            &input,
            &output,
            ConversionOptions {
                output_format: Format::Html,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(result.status, ConversionStatus::Completed);
    let content = std::fs::read_to_string(&output).unwrap();
    assert!(content.contains("<table>"));
    assert!(content.contains("Bob"));
}

#[test]
fn data_csv_to_markdown_generates_table() {
    let (engine, temp) = setup();
    let input = fixture_path("sample.csv");
    let output = temp.path().join("output.md");

    let result = engine
        .convert(
            &input,
            &output,
            ConversionOptions {
                output_format: Format::Md,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(result.status, ConversionStatus::Completed);
    let content = std::fs::read_to_string(&output).unwrap();
    assert!(content.contains('|'));
    assert!(content.contains("---"));
    assert!(content.contains("Alice"));
}

#[test]
fn data_json_array_to_xml_succeeds() {
    let (engine, temp) = setup();
    let input = fixture_path("sample.json");
    let output = temp.path().join("output.xml");

    let result = engine
        .convert(
            &input,
            &output,
            ConversionOptions {
                output_format: Format::Xml,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(result.status, ConversionStatus::Completed);
    let content = std::fs::read_to_string(&output).unwrap();
    assert!(content.contains("<?xml"));
    assert!(content.contains("Alice"));
}

#[test]
fn data_json_object_to_xml_succeeds() {
    let (_, temp) = setup();
    let engine = ConvxEngine::new();

    // Create a JSON object (non-array) input
    let input = temp.path().join("obj.json");
    std::fs::write(&input, r#"{"name": "Alice", "age": 30}"#).unwrap();
    let output = temp.path().join("output.xml");

    let result = engine
        .convert(
            &input,
            &output,
            ConversionOptions {
                output_format: Format::Xml,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(result.status, ConversionStatus::Completed);
    let content = std::fs::read_to_string(&output).unwrap();
    assert!(content.contains("<?xml"));
    assert!(content.contains("Alice"));
}

#[test]
#[ignore]
fn data_csv_to_pdf_succeeds() {
    let (engine, temp) = setup();
    let input = fixture_path("sample.csv");
    let output = temp.path().join("output.pdf");

    let result = engine
        .convert(
            &input,
            &output,
            ConversionOptions {
                output_format: Format::Pdf,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(result.status, ConversionStatus::Completed);
    assert!(output.exists());
    assert!(std::fs::metadata(&output).unwrap().len() > 0);
}

#[test]
#[ignore]
fn data_parquet_to_csv_succeeds() {
    // This test requires pyarrow and a parquet fixture.
    // Create parquet from CSV first, then convert back.
    let (engine, temp) = setup();
    let csv_input = fixture_path("sample.csv");
    let parquet = temp.path().join("intermediate.parquet");
    let csv_output = temp.path().join("output.csv");

    // CSV -> Parquet
    engine
        .convert(
            &csv_input,
            &parquet,
            ConversionOptions {
                output_format: Format::Parquet,
                ..Default::default()
            },
        )
        .unwrap();

    // Parquet -> CSV
    let result = engine
        .convert(
            &parquet,
            &csv_output,
            ConversionOptions {
                output_format: Format::Csv,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(result.status, ConversionStatus::Completed);
    let content = std::fs::read_to_string(&csv_output).unwrap();
    assert!(content.contains("Alice"));
}
