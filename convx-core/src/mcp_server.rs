use crate::{
    presets, ConversionOptions, ConvxEngine, DocumentOptions, FfprobeInfo, Format, ImageOptions,
    VideoOptions,
};
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug)]
enum MessageFraming {
    JsonLines,
    ContentLength,
}

#[derive(Debug, Clone, Serialize)]
struct ToolDefinition {
    name: &'static str,
    description: &'static str,
    #[serde(rename = "inputSchema")]
    input_schema: Value,
}

#[derive(Debug, Deserialize)]
struct ConvertFileParams {
    input_path: Option<String>,
    output_format: Option<String>,
    output_path: Option<String>,
    quality: Option<u8>,
    max_size: Option<u64>,
    width: Option<u32>,
    fps: Option<f32>,
    page_start: Option<u32>,
    page_end: Option<u32>,
    overwrite: Option<bool>,
    preset: Option<String>,
    /// Base64-encoded file content (alternative to input_path).
    input_content: Option<String>,
    /// Original filename for format detection when using input_content.
    input_filename: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ConversionTargetsParams {
    input_format: String,
}

#[derive(Debug, Deserialize)]
struct CanConvertParams {
    from: String,
    to: String,
}

#[derive(Debug, Deserialize)]
struct FileInfoParams {
    path: String,
}

#[derive(Debug, Deserialize)]
struct BatchConvertParams {
    input_paths: Option<Vec<String>>,
    output_format: Option<String>,
    output_directory: Option<String>,
    quality: Option<u8>,
    max_size: Option<u64>,
    page_start: Option<u32>,
    page_end: Option<u32>,
    overwrite: Option<bool>,
    preset: Option<String>,
    /// Array of base64-encoded file contents (alternative to input_paths).
    input_contents: Option<Vec<String>>,
    /// Original filenames for format detection when using input_contents.
    input_filenames: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct PresetParams {
    name: String,
}

pub fn run_stdio_server() -> anyhow::Result<()> {
    let engine = ConvxEngine::new();
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = stdout.lock();
    let mut framing: Option<MessageFraming> = None;

    while let Some((request, detected_framing)) = read_message(&mut reader)? {
        if framing.is_none() {
            framing = Some(detected_framing);
        }

        if let Some(response) = handle_request(&engine, request) {
            write_message(
                &mut writer,
                &response,
                framing.unwrap_or(MessageFraming::JsonLines),
            )?;
        }
    }

    Ok(())
}

fn handle_request(engine: &ConvxEngine, request: Value) -> Option<Value> {
    let method = request.get("method").and_then(Value::as_str)?.to_string();
    let id = request.get("id").cloned();

    // Notification (no id) => no response
    id.as_ref()?;

    let id = id.expect("id checked above");

    let response = match method.as_str() {
        "initialize" => ok(
            id,
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": {
                    "name": "convx",
                    "version": env!("CARGO_PKG_VERSION"),
                }
            }),
        ),
        "tools/list" => ok(id, json!({ "tools": tool_definitions() })),
        "tools/call" => {
            let params = request.get("params").cloned().unwrap_or_else(|| json!({}));
            match call_tool(engine, params) {
                Ok(data) => ok(
                    id,
                    json!({
                        "content": [{ "type": "text", "text": serde_json::to_string_pretty(&data).unwrap_or_else(|_| "{}".to_string()) }],
                        "structuredContent": data,
                        "isError": false
                    }),
                ),
                Err(err) => ok(
                    id,
                    json!({
                        "content": [{ "type": "text", "text": err.clone() }],
                        "isError": true,
                        "structuredContent": { "error": err }
                    }),
                ),
            }
        }
        _ => err(id, -32601, format!("Method not found: {}", method)),
    };

    Some(response)
}

fn call_tool(engine: &ConvxEngine, params: Value) -> Result<Value, String> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| "Missing tool name".to_string())?;
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));

    match name {
        "usage-guide" => Ok(json!({
            "description": "Important guidelines for using ConvX MCP tools effectively",
            "messages": [
                {
                    "role": "user",
                    "content": {
                        "type": "text",
                        "text": "# ConvX MCP Server Usage Guide\n\n## IMPORTANT: Always read tool descriptions\nBefore calling any tool, read its schema and description carefully. They define required fields, optional fields, and output semantics.\n\n## Core workflow\n1. Use get_supported_formats to inspect categories\n2. Use get_conversion_targets for a specific source format\n3. Use can_convert before running user-visible actions\n4. Use convert_file for one file, batch_convert for many\n5. Use get_file_info to inspect metadata and likely outcomes\n\n## Format scope\n- Conversion-capable formats include image/video/audio/document/data/ebook\n- Not every pair in a category is necessarily recommended for quality\n\n## Presets\n- Use list_presets first, then get_preset for details\n- Presets may include quality/resize/audio settings\n- max_file_size is applied as a best-effort target\n\n## Target-size conversions\n- Use max_size (bytes) in convert_file/batch_convert for upload limits\n- ConvX applies iterative tuning (quality/bitrate/resize) to meet the limit\n- If a target cannot be met, conversion fails with a clear error\n\n## File path behavior\n- Paths are local filesystem paths\n- If output_path is omitted, ConvX derives one from input + output_format\n- If derived output would equal input, ConvX uses a -converted suffix\n\n## Uploaded file support\n- For files uploaded in chat (not on disk), use input_content (base64) + input_filename instead of input_path\n- The file is saved to a temp directory and converted from there\n- output_path can be set to control where the converted file is saved\n- batch_convert supports input_contents + input_filenames arrays for multiple uploaded files\n\n## Error handling\n- Missing dependencies are reported by check_dependencies\n- Unsupported conversions return explicit errors\n- Existing output paths can fail when overwrite=false\n\n## Best practices\n1. Check dependencies at session start\n2. Validate targets before conversion\n3. Prefer presets for common workflows\n4. For batch operations, report both successes and failures"
                    }
                }
            ]
        })),
        "convert_file" => {
            let p: ConvertFileParams =
                serde_json::from_value(arguments).map_err(|e| format!("Invalid params: {}", e))?;

            let input = match (&p.input_path, &p.input_content) {
                (Some(path), _) => PathBuf::from(path),
                (None, Some(content)) => {
                    let filename = p.input_filename.as_deref()
                        .ok_or("input_filename is required when using input_content")?;
                    save_base64_to_temp(content, filename)?
                }
                (None, None) => return Err("Either input_path or input_content is required".to_string()),
            };
            let preset = p
                .preset
                .as_deref()
                .map(presets::get_preset)
                .transpose()
                .map_err(|e| e.to_string())?;

            let output_format = p
                .output_format
                .as_deref()
                .map(parse_format)
                .transpose()?
                .or_else(|| preset.as_ref().map(|p| p.output_format))
                .ok_or_else(|| "Either output_format or preset is required".to_string())?;

            let output = p
                .output_path
                .map(PathBuf::from)
                .unwrap_or_else(|| default_output_path(&input, output_format));

            let options = ConversionOptions {
                output_format,
                quality: p.quality,
                max_file_size: p.max_size,
                document: if p.page_start.is_some() || p.page_end.is_some() {
                    Some(DocumentOptions {
                        page_start: p.page_start,
                        page_end: p.page_end,
                    })
                } else {
                    None
                },
                image: if p.width.is_some() {
                    Some(ImageOptions {
                        width: p.width,
                        ..Default::default()
                    })
                } else {
                    None
                },
                video: if p.width.is_some() || p.fps.is_some() {
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

            let options = presets::resolve_options(options, preset.as_ref());

            let result = engine
                .convert(&input, &output, options)
                .map_err(|e| e.to_string())?;
            serde_json::to_value(result).map_err(|e| e.to_string())
        }
        "get_supported_formats" => {
            let images: Vec<String> = crate::Format::all_by_category(crate::FormatCategory::Image)
                .into_iter()
                .map(|f| f.extension().to_string())
                .collect();
            let videos: Vec<String> = crate::Format::all_by_category(crate::FormatCategory::Video)
                .into_iter()
                .map(|f| f.extension().to_string())
                .collect();
            let audio: Vec<String> = crate::Format::all_by_category(crate::FormatCategory::Audio)
                .into_iter()
                .map(|f| f.extension().to_string())
                .collect();
            let document: Vec<String> =
                crate::Format::all_by_category(crate::FormatCategory::Document)
                    .into_iter()
                    .map(|f| f.extension().to_string())
                    .collect();
            let data: Vec<String> = crate::Format::all_by_category(crate::FormatCategory::Data)
                .into_iter()
                .map(|f| f.extension().to_string())
                .collect();
            let ebook: Vec<String> = crate::Format::all_by_category(crate::FormatCategory::Ebook)
                .into_iter()
                .map(|f| f.extension().to_string())
                .collect();

            Ok(json!({
                "image": images,
                "video": videos,
                "audio": audio,
                "document": document,
                "data": data,
                "ebook": ebook,
            }))
        }
        "get_conversion_targets" => {
            let p: ConversionTargetsParams =
                serde_json::from_value(arguments).map_err(|e| format!("Invalid params: {}", e))?;
            let format = parse_format(&p.input_format)?;
            let targets: Vec<String> = format
                .convertible_targets()
                .into_iter()
                .map(|f| f.extension().to_string())
                .collect();
            Ok(json!({ "input_format": format.extension(), "targets": targets }))
        }
        "can_convert" => {
            let p: CanConvertParams =
                serde_json::from_value(arguments).map_err(|e| format!("Invalid params: {}", e))?;
            let from = parse_format(&p.from)?;
            let to = parse_format(&p.to)?;
            Ok(json!({
                "from": from.extension(),
                "to": to.extension(),
                "can_convert": engine.can_convert(from, to),
            }))
        }
        "get_file_info" => {
            let p: FileInfoParams =
                serde_json::from_value(arguments).map_err(|e| format!("Invalid params: {}", e))?;
            let path = PathBuf::from(p.path);
            let metadata = std::fs::metadata(&path).map_err(|e| e.to_string())?;
            let format = Format::detect(&path);
            let targets: Vec<String> = format
                .map(|f| {
                    f.convertible_targets()
                        .into_iter()
                        .map(|t| t.extension().to_string())
                        .collect()
                })
                .unwrap_or_default();

            let probe = FfprobeInfo::probe(&path);
            let duration_seconds = probe.as_ref().and_then(|p| p.duration_seconds());
            let (width, height) = probe
                .as_ref()
                .map(|p| p.dimensions())
                .unwrap_or((None, None));

            let is_image = matches!(
                format.map(|f| f.category()),
                Some(crate::FormatCategory::Image)
            );

            let fps = if is_image {
                None
            } else {
                probe.as_ref().and_then(|p| p.fps())
            };
            let video_codec = if is_image {
                None
            } else {
                probe.as_ref().and_then(|p| p.video_codec())
            };

            let audio_codec = probe.as_ref().and_then(|p| p.audio_codec());
            let audio_sample_rate = probe.as_ref().and_then(|p| p.audio_sample_rate());
            let audio_channels = probe.as_ref().and_then(|p| p.audio_channels());

            Ok(json!({
                "path": path,
                "name": path.file_name().and_then(|v| v.to_str()).unwrap_or_default(),
                "size": metadata.len(),
                "format": format.map(|f| f.extension()),
                "conversion_targets": targets,
                "duration_seconds": duration_seconds,
                "width": width,
                "height": height,
                "fps": fps,
                "video_codec": video_codec,
                "audio_codec": audio_codec,
                "audio_sample_rate": audio_sample_rate,
                "audio_channels": audio_channels
            }))
        }
        "batch_convert" => {
            let p: BatchConvertParams =
                serde_json::from_value(arguments).map_err(|e| format!("Invalid params: {}", e))?;

            // Resolve inputs from either paths or base64 contents
            let input_paths: Vec<PathBuf> = match (&p.input_paths, &p.input_contents) {
                (Some(paths), _) if !paths.is_empty() => {
                    paths.iter().map(PathBuf::from).collect()
                }
                (_, Some(contents)) if !contents.is_empty() => {
                    let filenames = p.input_filenames.as_ref()
                        .ok_or("input_filenames is required when using input_contents")?;
                    if filenames.len() != contents.len() {
                        return Err("input_contents and input_filenames must have the same length".to_string());
                    }
                    let mut paths = Vec::new();
                    for (content, filename) in contents.iter().zip(filenames.iter()) {
                        paths.push(save_base64_to_temp(content, filename)?);
                    }
                    paths
                }
                _ => return Err("Either input_paths or input_contents must be provided and non-empty".to_string()),
            };

            let preset = p
                .preset
                .as_deref()
                .map(presets::get_preset)
                .transpose()
                .map_err(|e| e.to_string())?;

            let output_format = p
                .output_format
                .as_deref()
                .map(parse_format)
                .transpose()?
                .or_else(|| preset.as_ref().map(|p| p.output_format))
                .ok_or_else(|| "Either output_format or preset is required".to_string())?;

            let output_dir = p.output_directory.map(PathBuf::from);
            let mut converted = Vec::new();
            let mut failed = Vec::new();

            for input in input_paths {
                let output = match &output_dir {
                    Some(dir) => {
                        let stem = input
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("output");
                        dir.join(format!("{}.{}", stem, output_format.extension()))
                    }
                    None => default_output_path(&input, output_format),
                };

                let options = ConversionOptions {
                    output_format,
                    quality: p.quality,
                    max_file_size: p.max_size,
                    document: if p.page_start.is_some() || p.page_end.is_some() {
                        Some(DocumentOptions {
                            page_start: p.page_start,
                            page_end: p.page_end,
                        })
                    } else {
                        None
                    },
                    overwrite: p.overwrite.unwrap_or(false),
                    ..Default::default()
                };

                let options = presets::resolve_options(options, preset.as_ref());

                match engine.convert(&input, &output, options) {
                    Ok(result) => converted.push(json!({
                        "input": input,
                        "output": output,
                        "duration_ms": result.duration_ms,
                    })),
                    Err(e) => failed.push(json!({
                        "input": input,
                        "output": output,
                        "error": e.to_string(),
                    })),
                }
            }

            Ok(json!({
                "output_format": output_format.extension(),
                "converted_count": converted.len(),
                "failed_count": failed.len(),
                "converted": converted,
                "failed": failed,
            }))
        }
        "list_presets" => {
            let presets = presets::built_in_presets();
            let list = serde_json::to_value(presets).map_err(|e| e.to_string())?;
            Ok(json!({ "presets": list }))
        }
        "get_preset" => {
            let p: PresetParams =
                serde_json::from_value(arguments).map_err(|e| format!("Invalid params: {}", e))?;
            let preset = presets::get_preset(&p.name).map_err(|e| e.to_string())?;
            serde_json::to_value(preset).map_err(|e| e.to_string())
        }
        _ => Err(format!("Unknown tool: {}", name)),
    }
}

fn tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "usage-guide",
            description: "ConvX usage guide. Call this first to learn best practices for file conversion workflows.",
            input_schema: json!({ "type": "object", "properties": {} }),
        },
        ToolDefinition {
            name: "convert_file",
            description: "Convert an image, video, audio, document, data, or ebook file to another format. Supports 54+ formats including PNG, JPG, HEIC, WebP, AVIF, GIF, MP4, MOV, WebM, MP3, WAV, FLAC, PDF, DOCX, CSV, Parquet, EPUB, and more. Use this when a user asks to convert, resize, compress, or change the format of any file. For uploaded/attached files, pass base64-encoded content via input_content + input_filename. For files already on disk, use input_path. All processing is local and private.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "input_path": { "type": "string", "description": "Filesystem path to the input file" },
                    "output_format": { "type": "string", "description": "Optional when preset is provided" },
                    "output_path": { "type": "string" },
                    "quality": { "type": "integer", "minimum": 1, "maximum": 100 },
                    "max_size": { "type": "integer", "minimum": 1, "description": "Target maximum output size in bytes (best effort)" },
                    "width": { "type": "integer" },
                    "fps": { "type": "number" },
                    "page_start": { "type": "integer", "minimum": 1, "description": "For PDF->image exports: first page (1-based)" },
                    "page_end": { "type": "integer", "minimum": 1, "description": "For PDF->image exports: last page (1-based)" },
                    "overwrite": { "type": "boolean" },
                    "preset": { "type": "string", "description": "Optional built-in preset name" },
                    "input_content": { "type": "string", "description": "Base64-encoded file content (alternative to input_path, for uploaded files)" },
                    "input_filename": { "type": "string", "description": "Original filename with extension, required when using input_content (e.g. 'photo.png')" }
                }
            }),
        },
        ToolDefinition {
            name: "get_supported_formats",
            description: "List all 54+ supported file formats grouped by category (image, video, audio, document, data, ebook). Call this to discover what formats ConvX can handle.",
            input_schema: json!({ "type": "object", "properties": {} }),
        },
        ToolDefinition {
            name: "get_conversion_targets",
            description: "Get all output formats a given input format can be converted to. Use this before converting to verify the target format is supported.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "input_format": { "type": "string" }
                },
                "required": ["input_format"]
            }),
        },
        ToolDefinition {
            name: "can_convert",
            description: "Quick check if a specific format-to-format conversion is supported (e.g. PNG to WebP, MP4 to GIF, PDF to DOCX).",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "from": { "type": "string" },
                    "to": { "type": "string" }
                },
                "required": ["from", "to"]
            }),
        },
        ToolDefinition {
            name: "get_file_info",
            description: "Inspect a file's format, size, dimensions, duration, codecs, and available conversion targets. Useful before converting to understand the source file.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" }
                },
                "required": ["path"]
            }),
        },
        ToolDefinition {
            name: "batch_convert",
            description: "Convert multiple files at once to the same target format. Use for bulk operations like 'convert all these PNGs to WebP'. Supports both filesystem paths (input_paths) and uploaded file content (input_contents + input_filenames as base64).",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "input_paths": { "type": "array", "items": { "type": "string" }, "description": "Filesystem paths to input files" },
                    "output_format": { "type": "string", "description": "Optional when preset is provided" },
                    "output_directory": { "type": "string" },
                    "quality": { "type": "integer", "minimum": 1, "maximum": 100 },
                    "max_size": { "type": "integer", "minimum": 1, "description": "Target maximum output size in bytes (best effort)" },
                    "page_start": { "type": "integer", "minimum": 1, "description": "For PDF->image exports: first page (1-based)" },
                    "page_end": { "type": "integer", "minimum": 1, "description": "For PDF->image exports: last page (1-based)" },
                    "overwrite": { "type": "boolean" },
                    "preset": { "type": "string", "description": "Optional built-in preset name" },
                    "input_contents": { "type": "array", "items": { "type": "string" }, "description": "Base64-encoded file contents (alternative to input_paths)" },
                    "input_filenames": { "type": "array", "items": { "type": "string" }, "description": "Original filenames with extensions, required when using input_contents" }
                }
            }),
        },
        ToolDefinition {
            name: "list_presets",
            description: "List built-in conversion presets for common workflows (e.g. Discord-optimized, Twitter-ready, HEIC-to-JPG, Parquet-to-CSV). Presets bundle format + quality + size settings.",
            input_schema: json!({ "type": "object", "properties": {} }),
        },
        ToolDefinition {
            name: "get_preset",
            description: "Get full details (format, quality, size limits, options) for a specific built-in preset by name.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string" }
                },
                "required": ["name"]
            }),
        },
    ]
}

/// Decode base64 content to a temp file, returning the path.
fn save_base64_to_temp(content: &str, filename: &str) -> Result<PathBuf, String> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(content)
        .map_err(|e| format!("Invalid base64 input_content: {}", e))?;

    let temp_dir = std::env::temp_dir().join("convx-mcp");
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Cannot create temp dir: {}", e))?;

    let path = temp_dir.join(filename);
    std::fs::write(&path, &bytes)
        .map_err(|e| format!("Cannot write temp file: {}", e))?;

    Ok(path)
}

fn parse_format(ext: &str) -> Result<Format, String> {
    Format::from_extension(ext).ok_or_else(|| format!("Unknown format: {}", ext))
}

fn default_output_path(input: &Path, output_format: Format) -> PathBuf {
    let mut output = input.to_path_buf();
    output.set_extension(output_format.extension());
    if output == input {
        let parent = input
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let stem = input
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        output = parent.join(format!("{}-converted.{}", stem, output_format.extension()));
    }
    output
}

fn ok(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    })
}

fn err(id: Value, code: i64, message: String) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message,
        }
    })
}

fn read_message<R: BufRead>(reader: &mut R) -> io::Result<Option<(Value, MessageFraming)>> {
    let mut first_line = String::new();
    loop {
        first_line.clear();
        let read = reader.read_line(&mut first_line)?;
        if read == 0 {
            return Ok(None);
        }
        if !first_line.trim().is_empty() {
            break;
        }
    }

    // JSON lines fallback
    if first_line.trim_start().starts_with('{') {
        let value = serde_json::from_str(first_line.trim())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        return Ok(Some((value, MessageFraming::JsonLines)));
    }

    let mut content_length: Option<usize> = None;

    if first_line.to_lowercase().starts_with("content-length:") {
        content_length = first_line
            .split(':')
            .nth(1)
            .and_then(|v| v.trim().parse::<usize>().ok());
    }

    let mut line = String::new();
    loop {
        line.clear();
        let read = reader.read_line(&mut line)?;
        if read == 0 {
            break;
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
        if line.to_lowercase().starts_with("content-length:") {
            content_length = line
                .split(':')
                .nth(1)
                .and_then(|v| v.trim().parse::<usize>().ok());
        }
    }

    let len = content_length.ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "Missing Content-Length header")
    })?;

    let mut payload = vec![0_u8; len];
    reader.read_exact(&mut payload)?;
    let msg = serde_json::from_slice::<Value>(&payload)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    Ok(Some((msg, MessageFraming::ContentLength)))
}

fn write_message<W: Write>(
    writer: &mut W,
    message: &Value,
    framing: MessageFraming,
) -> io::Result<()> {
    let payload = serde_json::to_vec(message)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

    match framing {
        MessageFraming::ContentLength => {
            write!(writer, "Content-Length: {}\r\n\r\n", payload.len())?;
            writer.write_all(&payload)?;
        }
        MessageFraming::JsonLines => {
            writer.write_all(&payload)?;
            writer.write_all(b"\n")?;
        }
    }

    writer.flush()
}
