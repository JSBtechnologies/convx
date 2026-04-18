use crate::{
    presets, ConversionOptions, ConvxEngine, DependencyChecker, FfprobeInfo, Format,
    FormatCategory, ImageOptions, VideoOptions,
};
use clap::{Parser, Subcommand, ValueEnum};
use rayon::prelude::*;
use serde::Serialize;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CliFormatCategory {
    Image,
    Video,
    Audio,
    Document,
    Data,
    Ebook,
}

impl From<CliFormatCategory> for FormatCategory {
    fn from(value: CliFormatCategory) -> Self {
        match value {
            CliFormatCategory::Image => FormatCategory::Image,
            CliFormatCategory::Video => FormatCategory::Video,
            CliFormatCategory::Audio => FormatCategory::Audio,
            CliFormatCategory::Document => FormatCategory::Document,
            CliFormatCategory::Data => FormatCategory::Data,
            CliFormatCategory::Ebook => FormatCategory::Ebook,
        }
    }
}

#[derive(Parser)]
#[command(name = "convx")]
#[command(about = "Local-first file conversion. Your files never leave your machine.")]
#[command(version)]
struct Cli {
    /// Emit machine-readable JSON output
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Serialize)]
struct JsonConversionRow {
    input: String,
    output: Option<String>,
    status: String,
    input_size: Option<u64>,
    output_size: Option<u64>,
    duration_ms: Option<u64>,
    error: Option<String>,
}

#[derive(Serialize)]
struct JsonBatchSummary {
    converted: usize,
    failed: usize,
    duration_ms: u64,
    total_input_size: u64,
    total_output_size: u64,
    size_delta_percent: f64,
    rows: Vec<JsonConversionRow>,
}

#[derive(Serialize)]
struct JsonInfoOutput {
    file: String,
    format: String,
    category: String,
    size: u64,
    duration_seconds: Option<f64>,
    width: Option<u32>,
    height: Option<u32>,
    fps: Option<f64>,
    video_codec: Option<String>,
    audio_codec: Option<String>,
    audio_sample_rate: Option<u32>,
    audio_channels: Option<u32>,
    converts_to: Vec<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert a file
    Convert {
        /// Input file(s) or glob pattern(s)
        #[arg(required = true)]
        input: Vec<PathBuf>,

        /// Output file (single-input only)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output directory for converted files
        #[arg(short = 'd', long)]
        output_dir: Option<PathBuf>,

        /// Output format
        #[arg(short, long)]
        to: Option<String>,

        /// Quality (0-100)
        #[arg(short, long)]
        quality: Option<u8>,

        /// Target maximum output size in bytes (best effort)
        #[arg(long)]
        max_size: Option<u64>,

        /// FPS for GIF output
        #[arg(long)]
        fps: Option<f32>,

        /// Width
        #[arg(short, long)]
        width: Option<u32>,

        /// Overwrite output file if it already exists
        #[arg(long)]
        overwrite: bool,

        /// Number of parallel jobs for batch conversion
        #[arg(short = 'j', long, default_value_t = 1)]
        jobs: usize,

        /// Apply a built-in preset
        #[arg(short = 'p', long)]
        preset: Option<String>,
    },

    /// List and inspect built-in presets
    Presets {
        #[command(subcommand)]
        command: PresetCommand,
    },

    /// List supported formats
    Formats {
        /// Show targets for a specific input format
        #[arg(long)]
        from: Option<String>,

        /// Show only formats in a single category
        #[arg(long, value_enum)]
        category: Option<CliFormatCategory>,
    },

    /// Check system dependencies
    Check,

    /// Show detailed file metadata
    Info {
        /// File path to inspect
        path: PathBuf,
    },

    /// Watch a directory and auto-convert matching files
    Watch {
        /// Directory to watch
        directory: PathBuf,

        /// Output format
        #[arg(short, long)]
        to: Option<String>,

        /// Filter input patterns (comma-separated), e.g. "*.png,*.jpg"
        #[arg(long)]
        filter: Option<String>,

        /// Debounce window in milliseconds
        #[arg(long, default_value_t = 500)]
        debounce: u64,

        /// Width override
        #[arg(short, long)]
        width: Option<u32>,

        /// FPS override
        #[arg(long)]
        fps: Option<f32>,

        /// Quality override
        #[arg(short, long)]
        quality: Option<u8>,

        /// Target maximum output size in bytes (best effort)
        #[arg(long)]
        max_size: Option<u64>,

        /// Overwrite output file if it already exists
        #[arg(long)]
        overwrite: bool,

        /// Apply a built-in preset
        #[arg(short = 'p', long)]
        preset: Option<String>,
    },

    /// Run MCP server on stdin/stdout
    Mcp,

    /// Show version
    Version,
}

#[derive(Subcommand)]
enum PresetCommand {
    /// List available presets
    List,
    /// Show a preset definition
    Show {
        /// Preset name
        name: String,
    },
}

pub fn cli_main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let json_output = cli.json;
    let engine = ConvxEngine::new();

    match cli.command {
        Commands::Convert {
            input,
            output,
            output_dir,
            to,
            quality,
            max_size,
            fps,
            width,
            overwrite,
            jobs,
            preset,
        } => {
            // Check dependencies before converting
            if let Err(msg) = DependencyChecker::check_all() {
                eprintln!("{}", msg);
                std::process::exit(1);
            }

            let inputs = expand_inputs(input)?;
            if inputs.is_empty() {
                anyhow::bail!("No input files found.");
            }
            let is_batch = inputs.len() > 1;

            if is_batch && output.is_some() {
                anyhow::bail!("`output` is only supported for single-file conversion. Use --output-dir for batch mode.");
            }

            if let Some(ref dir) = output_dir {
                if !dir.exists() {
                    std::fs::create_dir_all(dir)?;
                }
                if !dir.is_dir() {
                    anyhow::bail!("--output-dir is not a directory: {}", dir.display());
                }
            }

            let preset = preset.as_deref().map(presets::get_preset).transpose()?;

            // Determine output format
            let output_format = to
                .as_deref()
                .and_then(Format::from_extension)
                .or_else(|| output.as_ref().and_then(|p| Format::detect(p)))
                .or_else(|| preset.as_ref().map(|p| p.output_format))
                .ok_or_else(|| {
                    anyhow::anyhow!("Could not determine output format. Use --to flag.")
                })?;

            if is_batch && to.is_none() && preset.is_none() {
                anyhow::bail!("Batch conversion requires --to <format> or --preset <name>.");
            }

            let started = Instant::now();

            let run_one = |input_path: PathBuf| {
                let output_path = derive_output_path(
                    &input_path,
                    output.as_ref(),
                    output_dir.as_ref(),
                    output_format,
                    is_batch,
                );

                let output_path = match output_path {
                    Ok(path) => path,
                    Err(e) => return (input_path, None, Err(e.to_string())),
                };

                let options = ConversionOptions {
                    output_format,
                    quality,
                    max_file_size: max_size,
                    image: width.map(|w| ImageOptions {
                        width: Some(w),
                        ..Default::default()
                    }),
                    video: if fps.is_some() || width.is_some() {
                        Some(VideoOptions {
                            fps,
                            width,
                            ..Default::default()
                        })
                    } else {
                        None
                    },
                    overwrite,
                    ..Default::default()
                };

                let options = presets::resolve_options(options, preset.as_ref());

                let result = engine
                    .convert(&input_path, &output_path, options)
                    .map_err(|e| e.to_string());

                (input_path, Some(output_path), result)
            };

            let attempts: Vec<(
                PathBuf,
                Option<PathBuf>,
                Result<crate::ConversionResult, String>,
            )> = if jobs > 1 && inputs.len() > 1 {
                let pool = rayon::ThreadPoolBuilder::new().num_threads(jobs).build()?;
                pool.install(|| inputs.into_par_iter().map(run_one).collect())
            } else {
                inputs.into_iter().map(run_one).collect()
            };

            let mut success = 0usize;
            let mut failed = 0usize;
            let mut total_input_size = 0u64;
            let mut total_output_size = 0u64;

            let mut json_rows: Vec<JsonConversionRow> = Vec::new();

            for (input_path, output_path, result) in attempts {
                match result {
                    Ok(result) => {
                        success += 1;
                        total_input_size += result.input_size;
                        total_output_size += result.output_size.unwrap_or(0);

                        json_rows.push(JsonConversionRow {
                            input: input_path.display().to_string(),
                            output: output_path.as_ref().map(|p| p.display().to_string()),
                            status: "completed".to_string(),
                            input_size: Some(result.input_size),
                            output_size: result.output_size,
                            duration_ms: Some(result.duration_ms),
                            error: None,
                        });

                        if !json_output && is_batch {
                            if let Some(out) = output_path {
                                println!("✓ {} → {}", input_path.display(), out.display());
                            }
                        } else if !json_output {
                            if let Some(out) = output_path {
                                println!(
                                    "✓ Converted: {} → {}",
                                    input_path.display(),
                                    out.display()
                                );
                                println!(
                                    "  Size: {} → {} ({:+.1}%)",
                                    format_size(result.input_size),
                                    format_size(result.output_size.unwrap_or(0)),
                                    if result.input_size > 0 {
                                        ((result.output_size.unwrap_or(0) as f64
                                            / result.input_size as f64)
                                            - 1.0)
                                            * 100.0
                                    } else {
                                        0.0
                                    }
                                );
                                println!("  Time: {}ms", result.duration_ms);
                            }
                        }
                    }
                    Err(err) => {
                        failed += 1;

                        json_rows.push(JsonConversionRow {
                            input: input_path.display().to_string(),
                            output: output_path.as_ref().map(|p| p.display().to_string()),
                            status: "failed".to_string(),
                            input_size: None,
                            output_size: None,
                            duration_ms: None,
                            error: Some(err.clone()),
                        });

                        if !json_output {
                            if let Some(out) = output_path {
                                eprintln!(
                                    "✗ {} → {} :: {}",
                                    input_path.display(),
                                    out.display(),
                                    err
                                );
                            } else {
                                eprintln!("✗ {} :: {}", input_path.display(), err);
                            }
                        }
                    }
                }
            }

            if json_output {
                if is_batch {
                    let payload = JsonBatchSummary {
                        converted: success,
                        failed,
                        duration_ms: started.elapsed().as_millis() as u64,
                        total_input_size,
                        total_output_size,
                        size_delta_percent: percent_smaller(total_input_size, total_output_size),
                        rows: json_rows,
                    };
                    println!("{}", serde_json::to_string_pretty(&payload)?);
                } else {
                    let row = json_rows
                        .into_iter()
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("No conversion result produced"))?;
                    println!("{}", serde_json::to_string_pretty(&row)?);
                }
            } else if is_batch {
                let elapsed = started.elapsed().as_secs_f64();
                println!("\n✓ Converted {} files in {:.1}s", success, elapsed);
                println!(
                    "  Total: {} → {} ({})",
                    format_size(total_input_size),
                    format_size(total_output_size),
                    size_delta_label(total_input_size, total_output_size)
                );
                println!("  Failed: {}", failed);
            }

            if failed > 0 {
                std::process::exit(1);
            }
        }

        Commands::Presets { command } => match command {
            PresetCommand::List => {
                let all = presets::built_in_presets();
                if json_output {
                    println!("{}", serde_json::to_string_pretty(&all)?);
                } else {
                    println!("Available presets:");
                    for p in all {
                        println!("  {:<18} {}", p.name, p.description);
                    }
                }
            }
            PresetCommand::Show { name } => {
                let preset = presets::get_preset(&name)?;
                if json_output {
                    println!("{}", serde_json::to_string_pretty(&preset)?);
                } else {
                    println!("Preset:      {}", preset.name);
                    println!("Description: {}", preset.description);
                    println!("Output:      {}", preset.output_format.extension());
                    if let Some(q) = preset.quality {
                        println!("Quality:     {}", q);
                    }
                    if let Some(max) = preset.max_file_size {
                        println!("Max size:    {}", format_size(max));
                    }
                }
            }
        },

        Commands::Formats { from, category } => {
            if let Some(from_ext) = from {
                let from_format = Format::from_extension(&from_ext)
                    .expect("Unknown format. Use one of the listed extensions.");
                let targets = from_format.convertible_targets();

                if json_output {
                    let payload = serde_json::json!({
                        "from": from_format.extension(),
                        "targets": targets.into_iter().map(|t| t.extension().to_string()).collect::<Vec<_>>()
                    });
                    println!("{}", serde_json::to_string_pretty(&payload)?);
                } else if targets.is_empty() {
                    println!("{} has no available conversion targets yet.", from_ext);
                } else {
                    println!("{} can convert to:", from_format.extension());
                    for target in targets {
                        println!("  {}", target.extension());
                    }
                }
            } else {
                let categories: Vec<FormatCategory> = match category {
                    Some(selected) => vec![selected.into()],
                    None => vec![
                        FormatCategory::Image,
                        FormatCategory::Video,
                        FormatCategory::Audio,
                        FormatCategory::Document,
                        FormatCategory::Data,
                        FormatCategory::Ebook,
                    ],
                };

                if json_output {
                    let mut map = serde_json::Map::new();
                    for cat in categories {
                        let key = match cat {
                            FormatCategory::Image => "image",
                            FormatCategory::Video => "video",
                            FormatCategory::Audio => "audio",
                            FormatCategory::Document => "document",
                            FormatCategory::Data => "data",
                            FormatCategory::Ebook => "ebook",
                        };

                        let values = Format::all_by_category(cat)
                            .into_iter()
                            .map(|f| f.extension().to_string())
                            .collect::<Vec<_>>();
                        map.insert(key.to_string(), serde_json::json!(values));
                    }
                    println!("{}", serde_json::to_string_pretty(&map)?);
                } else {
                    println!("Supported formats:\n");
                    for cat in categories {
                        let label = match cat {
                            FormatCategory::Image => "Images",
                            FormatCategory::Video => "Video",
                            FormatCategory::Audio => "Audio",
                            FormatCategory::Document => "Documents",
                            FormatCategory::Data => "Data",
                            FormatCategory::Ebook => "eBooks",
                        };

                        let names = Format::all_by_category(cat)
                            .into_iter()
                            .map(|f| f.extension())
                            .collect::<Vec<_>>()
                            .join(", ");

                        println!("  {}: {}", label, names);
                    }
                }
            }
        }

        Commands::Check => match DependencyChecker::check_all() {
            Ok(()) => {
                if json_output {
                    let payload = serde_json::json!({
                        "ok": true,
                        "message": DependencyChecker::get_versions(),
                    });
                    println!("{}", serde_json::to_string_pretty(&payload)?);
                } else {
                    println!("Checking system dependencies...\n");
                    println!("✓ All dependencies are installed!\n");
                    println!("{}", DependencyChecker::get_versions());
                }
            }
            Err(msg) => {
                if json_output {
                    let payload = serde_json::json!({ "ok": false, "message": msg });
                    println!("{}", serde_json::to_string_pretty(&payload)?);
                } else {
                    eprintln!("{}", msg);
                }
                std::process::exit(1);
            }
        },

        Commands::Info { path } => {
            if !path.exists() {
                anyhow::bail!("File not found: {}", path.display());
            }

            let metadata = std::fs::metadata(&path)?;
            let format = Format::detect(&path)
                .ok_or_else(|| anyhow::anyhow!("Could not detect format for {}", path.display()))?;

            let targets = format
                .convertible_targets()
                .into_iter()
                .map(|f| f.extension().to_string())
                .collect::<Vec<_>>();

            let probe = FfprobeInfo::probe(&path);

            let duration_seconds = probe.as_ref().and_then(|p| p.duration_seconds());
            let (width, height) = probe
                .as_ref()
                .map(|p| p.dimensions())
                .unwrap_or((None, None));
            let fps = probe.as_ref().and_then(|p| p.fps());
            let video_codec = probe.as_ref().and_then(|p| p.video_codec());
            let audio_codec = probe.as_ref().and_then(|p| p.audio_codec());
            let audio_sample_rate = probe.as_ref().and_then(|p| p.audio_sample_rate());
            let audio_channels = probe.as_ref().and_then(|p| p.audio_channels());

            let category_label = match format.category() {
                FormatCategory::Image => "image",
                FormatCategory::Video => "video",
                FormatCategory::Audio => "audio",
                FormatCategory::Document => "document",
                FormatCategory::Data => "data",
                FormatCategory::Ebook => "ebook",
            };

            let is_image = matches!(format.category(), FormatCategory::Image);
            let fps = if is_image { None } else { fps };
            let video_codec = if is_image { None } else { video_codec };

            if json_output {
                let payload = JsonInfoOutput {
                    file: path.display().to_string(),
                    format: format.extension().to_string(),
                    category: category_label.to_string(),
                    size: metadata.len(),
                    duration_seconds,
                    width,
                    height,
                    fps,
                    video_codec,
                    audio_codec,
                    audio_sample_rate,
                    audio_channels,
                    converts_to: targets,
                };
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("File:       {}", path.display());
                println!(
                    "Format:     {} ({})",
                    format.extension().to_uppercase(),
                    match format.category() {
                        FormatCategory::Image => "Image",
                        FormatCategory::Video => "Video",
                        FormatCategory::Audio => "Audio",
                        FormatCategory::Document => "Document",
                        FormatCategory::Data => "Data",
                        FormatCategory::Ebook => "eBook",
                    }
                );
                println!("Size:       {}", format_size(metadata.len()));

                if let Some(seconds) = duration_seconds {
                    println!("Duration:   {}", format_duration(seconds));
                }
                if let (Some(w), Some(h)) = (width, height) {
                    println!("Resolution: {}x{}", w, h);
                }
                if let Some(v) = fps {
                    println!("FPS:        {:.2}", v);
                }
                if let Some(codec) = video_codec {
                    println!("Video:      {}", codec);
                }
                if let Some(codec) = audio_codec {
                    let mut parts = vec![codec];
                    if let Some(sr) = audio_sample_rate {
                        parts.push(format!("{} Hz", sr));
                    }
                    if let Some(ch) = audio_channels {
                        parts.push(match ch {
                            1 => "mono".to_string(),
                            2 => "stereo".to_string(),
                            n => format!("{} ch", n),
                        });
                    }
                    println!("Audio:      {}", parts.join(", "));
                }

                if targets.is_empty() {
                    println!("Converts to: (none)");
                } else {
                    println!("Converts to: {}", targets.join(", "));
                }
            }
        }

        Commands::Watch {
            directory,
            to,
            filter,
            debounce,
            width,
            fps,
            quality,
            max_size,
            overwrite,
            preset,
        } => {
            if let Err(msg) = DependencyChecker::check_all() {
                if json_output {
                    let payload = serde_json::json!({ "ok": false, "message": msg });
                    println!("{}", serde_json::to_string_pretty(&payload)?);
                } else {
                    eprintln!("{}", msg);
                }
                std::process::exit(1);
            }

            let preset = preset.as_deref().map(presets::get_preset).transpose()?;

            let output_format = to
                .as_deref()
                .and_then(Format::from_extension)
                .or_else(|| preset.as_ref().map(|p| p.output_format))
                .ok_or_else(|| {
                    anyhow::anyhow!("Could not determine output format. Use --to or --preset.")
                })?;

            crate::watch::run_watch(
                &engine,
                crate::watch::WatchRunOptions {
                    directory,
                    output_format,
                    quality,
                    max_size,
                    width,
                    fps,
                    overwrite,
                    filter,
                    debounce_ms: debounce,
                    preset,
                    json_output,
                },
            )?;
        }

        Commands::Mcp => {
            crate::mcp_server::run_stdio_server()?;
        }

        Commands::Version => {
            if json_output {
                let payload = serde_json::json!({
                    "name": "convx",
                    "version": env!("CARGO_PKG_VERSION")
                });
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("convx {}", env!("CARGO_PKG_VERSION"));
            }
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

fn percent_smaller(input: u64, output: u64) -> f64 {
    if input == 0 {
        return 0.0;
    }
    (1.0 - (output as f64 / input as f64)) * 100.0
}

fn size_delta_label(input: u64, output: u64) -> String {
    let delta = percent_smaller(input, output);
    if delta >= 0.0 {
        format!("{:.1}% smaller", delta)
    } else {
        format!("{:.1}% larger", -delta)
    }
}

fn derive_output_path(
    input: &PathBuf,
    output: Option<&PathBuf>,
    output_dir: Option<&PathBuf>,
    output_format: Format,
    is_batch: bool,
) -> anyhow::Result<PathBuf> {
    if !is_batch {
        if let Some(path) = output {
            return Ok(path.clone());
        }
    }

    let parent = output_dir
        .cloned()
        .or_else(|| input.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));

    let stem = input.file_stem().and_then(|s| s.to_str()).ok_or_else(|| {
        anyhow::anyhow!("Could not derive output file name from {}", input.display())
    })?;

    let mut output = parent.join(format!("{}.{}", stem, output_format.extension()));
    if output == *input {
        output = parent.join(format!("{}-converted.{}", stem, output_format.extension()));
    }

    Ok(output)
}

fn expand_inputs(raw_inputs: Vec<PathBuf>) -> anyhow::Result<Vec<PathBuf>> {
    let mut expanded = Vec::new();

    for input in raw_inputs {
        let pattern = input.to_string_lossy().to_string();
        let is_glob = pattern.contains('*')
            || pattern.contains('?')
            || pattern.contains('[')
            || pattern.contains('{');

        if is_glob {
            let mut matches = Vec::new();
            for path in glob::glob(&pattern)?.flatten() {
                if path.is_file() {
                    matches.push(path);
                }
            }

            if matches.is_empty() {
                anyhow::bail!("No files matched pattern: {}", pattern);
            }

            expanded.extend(matches);
        } else {
            expanded.push(input);
        }
    }

    Ok(expanded)
}

fn format_duration(seconds: f64) -> String {
    let total = seconds.round() as u64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;

    if h > 0 {
        format!("{}:{:02}:{:02}", h, m, s)
    } else {
        format!("{}:{:02}", m, s)
    }
}
