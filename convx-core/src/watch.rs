use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::presets;
use crate::types::preset::Preset;
use crate::{ConversionOptions, ConvxEngine, Format, ImageOptions, VideoOptions};

pub struct WatchRunOptions {
    pub directory: PathBuf,
    pub output_format: Format,
    pub quality: Option<u8>,
    pub max_size: Option<u64>,
    pub width: Option<u32>,
    pub fps: Option<f32>,
    pub overwrite: bool,
    pub filter: Option<String>,
    pub debounce_ms: u64,
    pub preset: Option<Preset>,
    pub json_output: bool,
}

pub fn run_watch(engine: &ConvxEngine, opts: WatchRunOptions) -> anyhow::Result<()> {
    if !opts.directory.exists() {
        anyhow::bail!(
            "Watch directory does not exist: {}",
            opts.directory.display()
        );
    }
    if !opts.directory.is_dir() {
        anyhow::bail!(
            "Watch path is not a directory: {}",
            opts.directory.display()
        );
    }

    let filters = parse_filter_extensions(opts.filter.as_deref());
    let debounce = Duration::from_millis(opts.debounce_ms.max(50));

    if !opts.json_output {
        println!(
            "Watching {} for changes -> {} (debounce {}ms)",
            opts.directory.display(),
            opts.output_format.extension(),
            debounce.as_millis()
        );
        if let Some(ref f) = opts.filter {
            println!("Filter: {}", f);
        }
        println!("Press Ctrl+C to stop.");
    }

    let (tx, rx) = mpsc::channel();
    let mut watcher: RecommendedWatcher = notify::recommended_watcher(move |res| {
        let _ = tx.send(res);
    })?;

    watcher.watch(&opts.directory, RecursiveMode::Recursive)?;

    let mut seen_at: HashMap<PathBuf, Instant> = HashMap::new();
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = Arc::clone(&shutdown);

    ctrlc::set_handler(move || {
        shutdown_clone.store(true, Ordering::SeqCst);
    })?;

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

        let event = match evt {
            Ok(event) => event,
            Err(err) => {
                if !opts.json_output {
                    eprintln!("watch error: {}", err);
                }
                continue;
            }
        };

        if !is_create_or_modify(&event) {
            continue;
        }

        for path in event.paths {
            if !path.is_file() {
                continue;
            }

            let now = Instant::now();
            let last = seen_at.get(&path).copied();
            if let Some(last) = last {
                if now.duration_since(last) < debounce {
                    continue;
                }
            }
            seen_at.insert(path.clone(), now);

            let Some(ext) = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_lowercase())
            else {
                continue;
            };

            if ext == opts.output_format.extension() {
                continue;
            }

            if !matches_filter(&ext, filters.as_deref()) {
                continue;
            }

            let output = derive_output_path(&path, opts.output_format);

            let base_options = ConversionOptions {
                output_format: opts.output_format,
                quality: opts.quality,
                max_file_size: opts.max_size,
                image: opts.width.map(|w| ImageOptions {
                    width: Some(w),
                    ..Default::default()
                }),
                video: if opts.width.is_some() || opts.fps.is_some() {
                    Some(VideoOptions {
                        width: opts.width,
                        fps: opts.fps,
                        ..Default::default()
                    })
                } else {
                    None
                },
                overwrite: opts.overwrite,
                ..Default::default()
            };

            let options = presets::resolve_options(base_options, opts.preset.as_ref());

            match engine.convert(&path, &output, options) {
                Ok(result) => {
                    if opts.json_output {
                        let payload = serde_json::json!({
                            "status": "completed",
                            "input": path,
                            "output": output,
                            "input_size": result.input_size,
                            "output_size": result.output_size,
                            "duration_ms": result.duration_ms,
                        });
                        println!("{}", serde_json::to_string(&payload)?);
                    } else {
                        println!("✓ {} → {}", path.display(), output.display());
                    }
                }
                Err(err) => {
                    if opts.json_output {
                        let payload = serde_json::json!({
                            "status": "failed",
                            "input": path,
                            "output": output,
                            "error": err.to_string(),
                        });
                        println!("{}", serde_json::to_string(&payload)?);
                    } else {
                        eprintln!("✗ {} → {} :: {}", path.display(), output.display(), err);
                    }
                }
            }
        }
    }

    Ok(())
}

fn is_create_or_modify(event: &Event) -> bool {
    matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_))
}

fn parse_filter_extensions(filter: Option<&str>) -> Option<Vec<String>> {
    let filter = filter?;

    let exts = filter
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(normalize_extension)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();

    if exts.is_empty() {
        None
    } else {
        Some(exts)
    }
}

fn normalize_extension(raw: &str) -> String {
    let cleaned = raw
        .trim()
        .trim_start_matches('*')
        .trim_start_matches('.')
        .trim();

    if cleaned.contains('.') {
        cleaned.rsplit('.').next().unwrap_or("").to_lowercase()
    } else {
        cleaned.to_lowercase()
    }
}

fn matches_filter(ext: &str, filters: Option<&[String]>) -> bool {
    let Some(filters) = filters else {
        return true;
    };
    filters.iter().any(|f| f == ext)
}

fn derive_output_path(input: &Path, output_format: Format) -> PathBuf {
    let parent = input
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");

    let mut output = parent.join(format!("{}.{}", stem, output_format.extension()));
    if output == input {
        output = parent.join(format!("{}-converted.{}", stem, output_format.extension()));
    }
    output
}
