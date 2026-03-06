use convx::{
    AudioOptions, ConversionOptions, ConvxEngine, DependencyChecker, Format, FormatCategory,
    VideoOptions,
};
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn require_ffmpeg() -> String {
    DependencyChecker::ffmpeg_executable().expect(
        "FFmpeg is required for full matrix tests. Install ffmpeg and ensure it is discoverable.",
    )
}

fn require_vips() -> String {
    DependencyChecker::vips_executable().expect(
        "libvips is required for full matrix tests. Install vips/libvips and ensure it is discoverable.",
    )
}

fn run(cmd: &str, args: &[String], context: &str) {
    let out = Command::new(cmd)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("Failed to launch {} for {}: {}", cmd, context, e));

    assert!(
        out.status.success(),
        "{} failed.\nstdout:\n{}\nstderr:\n{}",
        context,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn generate_png(path: &Path) {
    let ffmpeg = require_ffmpeg();
    run(
        &ffmpeg,
        &[
            "-f".into(),
            "lavfi".into(),
            "-i".into(),
            "color=c=0x3366ff:s=640x360:d=1".into(),
            "-frames:v".into(),
            "1".into(),
            "-y".into(),
            path.to_string_lossy().to_string(),
        ],
        "generate PNG fixture",
    );
}

fn generate_mp4(path: &Path) {
    let ffmpeg = require_ffmpeg();
    run(
        &ffmpeg,
        &[
            "-f".into(),
            "lavfi".into(),
            "-i".into(),
            "testsrc2=size=640x360:rate=30".into(),
            "-f".into(),
            "lavfi".into(),
            "-i".into(),
            "sine=frequency=1000:sample_rate=44100:duration=2".into(),
            "-t".into(),
            "2".into(),
            "-c:v".into(),
            "libx264".into(),
            "-pix_fmt".into(),
            "yuv420p".into(),
            "-c:a".into(),
            "aac".into(),
            "-shortest".into(),
            "-y".into(),
            path.to_string_lossy().to_string(),
        ],
        "generate MP4 fixture",
    );
}

fn generate_wav(path: &Path) {
    let ffmpeg = require_ffmpeg();
    run(
        &ffmpeg,
        &[
            "-f".into(),
            "lavfi".into(),
            "-i".into(),
            "sine=frequency=440:duration=2".into(),
            "-c:a".into(),
            "pcm_s16le".into(),
            "-ar".into(),
            "44100".into(),
            "-ac".into(),
            "2".into(),
            "-y".into(),
            path.to_string_lossy().to_string(),
        ],
        "generate WAV fixture",
    );
}

fn convert_and_assert(
    engine: &ConvxEngine,
    input: &Path,
    from: Format,
    to: Format,
    out_dir: &Path,
) {
    assert!(
        engine.can_convert(from, to),
        "Engine should report can_convert({:?}, {:?})",
        from,
        to
    );

    let output = out_dir.join(format!(
        "{}-to-{}.{}",
        from.extension(),
        to.extension(),
        to.extension()
    ));

    let mut options = ConversionOptions {
        output_format: to,
        quality: Some(80),
        overwrite: true,
        ..Default::default()
    };

    if from.category() == FormatCategory::Video && to == Format::Gif {
        options.video = Some(VideoOptions {
            fps: Some(10.0),
            width: Some(320),
            ..Default::default()
        });
    }

    if from.category() == FormatCategory::Audio {
        options.audio = Some(AudioOptions {
            bitrate: Some("192k".to_string()),
            ..Default::default()
        });
    }

    let result = engine.convert(input, &output, options).unwrap_or_else(|e| {
        panic!(
            "Conversion {:?} -> {:?} failed.\ninput: {}\noutput: {}\nerror: {}",
            from,
            to,
            input.display(),
            output.display(),
            e
        )
    });

    assert_eq!(result.output_format, to);
    assert!(
        output.exists(),
        "Expected output file to exist: {}",
        output.display()
    );
}

fn setup() -> (ConvxEngine, TempDir) {
    // Hard requirements for this suite.
    let _ffmpeg = require_ffmpeg();
    let _vips = require_vips();

    let engine = ConvxEngine::new();
    let temp = TempDir::new().expect("Failed to create temp dir");
    (engine, temp)
}

fn run_matrix_for_source(
    engine: &ConvxEngine,
    temp: &TempDir,
    input: &Path,
    from: Format,
) -> usize {
    let targets = from.convertible_targets();
    let count = targets.len();

    for target in targets {
        convert_and_assert(engine, input, from, target, temp.path());
    }

    count
}

fn convert_fixture(engine: &ConvxEngine, input: &Path, output: &Path, output_format: Format) {
    let mut options = ConversionOptions {
        output_format,
        quality: Some(80),
        overwrite: true,
        ..Default::default()
    };

    if input.extension().and_then(|e| e.to_str()) == Some("mp4") && output_format == Format::Gif {
        options.video = Some(VideoOptions {
            fps: Some(10.0),
            width: Some(320),
            ..Default::default()
        });
    }

    if input.extension().and_then(|e| e.to_str()) == Some("wav") {
        options.audio = Some(AudioOptions {
            bitrate: Some("192k".to_string()),
            ..Default::default()
        });
    }

    let _ = engine.convert(input, output, options).unwrap_or_else(|e| {
        panic!(
            "Failed to prepare fixture {:?} at {} from {}: {}",
            output_format,
            output.display(),
            input.display(),
            e
        )
    });
}

fn build_image_sources(
    engine: &ConvxEngine,
    temp: &TempDir,
    base_png: &Path,
) -> Vec<(Format, std::path::PathBuf)> {
    let mut sources = vec![(Format::Png, base_png.to_path_buf())];

    let image_targets = vec![
        Format::Jpg,
        Format::WebP,
        Format::Gif,
        Format::Bmp,
        Format::Tiff,
        Format::Ico,
        Format::Heic,
        Format::Heif,
        Format::Avif,
    ];

    for fmt in image_targets {
        let path = temp.path().join(format!(
            "source-image-{}.{}",
            fmt.extension(),
            fmt.extension()
        ));
        convert_fixture(engine, base_png, &path, fmt);
        sources.push((fmt, path));
    }

    // Prepare a native SVG source file so SVG->raster paths are exercised.
    let svg_path = temp.path().join("source-image-svg.svg");
    fs::write(
        &svg_path,
        r#"<svg xmlns='http://www.w3.org/2000/svg' width='256' height='256'><rect width='256' height='256' fill='#3366ff'/><circle cx='128' cy='128' r='72' fill='#ffffff'/></svg>"#,
    )
    .expect("Failed to write SVG fixture");
    sources.push((Format::Svg, svg_path));

    sources
}

fn build_video_sources(
    engine: &ConvxEngine,
    temp: &TempDir,
    base_mp4: &Path,
) -> Vec<(Format, std::path::PathBuf)> {
    let mut sources = vec![(Format::Mp4, base_mp4.to_path_buf())];

    let video_targets = vec![
        Format::Mov,
        Format::Webm,
        Format::Avi,
        Format::Mkv,
        Format::Wmv,
        Format::Flv,
        Format::M4v,
        Format::Mpeg,
        Format::Ts,
    ];

    for fmt in video_targets {
        let path = temp.path().join(format!(
            "source-video-{}.{}",
            fmt.extension(),
            fmt.extension()
        ));
        convert_fixture(engine, base_mp4, &path, fmt);
        sources.push((fmt, path));
    }

    sources
}

fn build_audio_sources(
    engine: &ConvxEngine,
    temp: &TempDir,
    base_wav: &Path,
) -> Vec<(Format, std::path::PathBuf)> {
    let mut sources = vec![(Format::Wav, base_wav.to_path_buf())];

    let audio_targets = vec![
        Format::Mp3,
        Format::Flac,
        Format::M4a,
        Format::Aac,
        Format::Ogg,
        Format::Wma,
        Format::Aiff,
        Format::Opus,
        Format::Ac3,
    ];

    for fmt in audio_targets {
        let path = temp.path().join(format!(
            "source-audio-{}.{}",
            fmt.extension(),
            fmt.extension()
        ));
        convert_fixture(engine, base_wav, &path, fmt);
        sources.push((fmt, path));
    }

    sources
}

#[test]
fn full_matrix_image_targets() {
    let (engine, temp) = setup();
    let image_input = temp.path().join("sample.png");
    generate_png(&image_input);

    let sources = build_image_sources(&engine, &temp, &image_input);
    let tested: usize = sources
        .iter()
        .map(|(from, path)| run_matrix_for_source(&engine, &temp, path, *from))
        .sum();
    assert!(tested > 0, "No image targets were tested");
}

#[test]
fn full_matrix_video_targets() {
    let (engine, temp) = setup();
    let video_input = temp.path().join("sample.mp4");
    generate_mp4(&video_input);

    let sources = build_video_sources(&engine, &temp, &video_input);
    let tested: usize = sources
        .iter()
        .map(|(from, path)| run_matrix_for_source(&engine, &temp, path, *from))
        .sum();
    assert!(tested > 0, "No video targets were tested");
}

#[test]
fn full_matrix_audio_targets() {
    let (engine, temp) = setup();
    let audio_input = temp.path().join("sample.wav");
    generate_wav(&audio_input);

    let sources = build_audio_sources(&engine, &temp, &audio_input);
    let tested: usize = sources
        .iter()
        .map(|(from, path)| run_matrix_for_source(&engine, &temp, path, *from))
        .sum();
    assert!(tested > 0, "No audio targets were tested");
}
