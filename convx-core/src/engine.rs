use crate::converters::{
    AudioConverter, Converter, DataConverter, DocumentConverter, EbookConverter, ImageConverter,
    VideoConverter,
};
use crate::types::{
    error::ConvxError,
    format::{Format, FormatCategory},
    options::ConversionOptions,
    result::ConversionResult,
};
use std::path::Path;
use std::sync::atomic::AtomicBool;

pub struct ConvxEngine {
    converters: Vec<Box<dyn Converter>>,
}

impl Default for ConvxEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ConvxEngine {
    pub fn new() -> Self {
        Self {
            converters: vec![
                Box::new(ImageConverter),
                Box::new(VideoConverter),
                Box::new(AudioConverter),
                Box::new(DocumentConverter),
                Box::new(DataConverter),
                Box::new(EbookConverter),
            ],
        }
    }

    pub fn convert(
        &self,
        input: &Path,
        output: &Path,
        options: ConversionOptions,
    ) -> Result<ConversionResult, ConvxError> {
        let (input_format, output_format) = self.validate_request(input, output, &options)?;

        let converter = self
            .converters
            .iter()
            .find(|c| c.can_convert(input_format, output_format))
            .ok_or(ConvxError::UnsupportedConversion {
                from: input_format,
                to: output_format,
            })?;

        if options.max_file_size.is_none() {
            return converter.convert(input, output, &options);
        }

        let mut tuned = options.clone();
        let target = tuned.max_file_size.unwrap_or(u64::MAX);
        let mut last_size = 0_u64;
        let max_attempts = 8;

        for attempt in 0..max_attempts {
            if attempt > 0 {
                tuned.overwrite = true;
            }

            let result = converter.convert(input, output, &tuned)?;
            let size = result.output_size.unwrap_or(0);
            last_size = size;

            if size <= target {
                return Ok(result);
            }

            if !Self::tighten_options_for_size(&mut tuned, output_format) {
                break;
            }
        }

        Err(ConvxError::ConversionFailed {
            reason: format!(
                "Could not meet max file size target (target={}, got={})",
                target, last_size
            ),
        })
    }

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
                    input,
                    output,
                    &options,
                    on_progress,
                    cancel_flag,
                );
            }
        }

        Err(ConvxError::UnsupportedConversion {
            from: input_format,
            to: output_format,
        })
    }

    fn validate_request(
        &self,
        input: &Path,
        output: &Path,
        options: &ConversionOptions,
    ) -> Result<(Format, Format), ConvxError> {
        // Check input exists
        if !input.exists() {
            return Err(ConvxError::FileNotFound {
                path: input.to_path_buf(),
            });
        }

        // Respect overwrite policy
        if output.exists() && !options.overwrite {
            return Err(ConvxError::OutputAlreadyExists {
                path: output.to_path_buf(),
            });
        }

        // Detect input format
        let input_format =
            Format::detect(input).ok_or_else(|| ConvxError::FormatDetectionFailed {
                path: input.to_path_buf(),
            })?;

        let output_format = options.output_format;

        // Validate conversion path before invoking converter commands
        if !self.can_convert(input_format, output_format) {
            return Err(ConvxError::UnsupportedConversion {
                from: input_format,
                to: output_format,
            });
        }

        Ok((input_format, output_format))
    }

    pub fn can_convert(&self, from: Format, to: Format) -> bool {
        self.converters
            .iter()
            .any(|converter| converter.can_convert(from, to))
    }
}

impl ConvxEngine {
    fn tighten_options_for_size(options: &mut ConversionOptions, output_format: Format) -> bool {
        let mut changed = false;

        if let Some(q) = options.quality {
            let next = q.saturating_sub(10).max(20);
            if next < q {
                options.quality = Some(next);
                changed = true;
            }
        } else {
            options.quality = Some(70);
            changed = true;
        }

        match output_format.category() {
            FormatCategory::Video => {
                let video = options.video.get_or_insert_with(Default::default);
                match video.crf {
                    Some(current) if current < 42 => {
                        video.crf = Some((current + 4).min(42));
                        changed = true;
                    }
                    None => {
                        video.crf = Some(28);
                        changed = true;
                    }
                    _ => {}
                }

                if let Some(w) = video.width {
                    let next_w = ((w as f32) * 0.85).round() as u32;
                    if next_w >= 320 && next_w < w {
                        video.width = Some(next_w);
                        changed = true;
                    }
                } else {
                    video.width = Some(1280);
                    changed = true;
                }
            }
            FormatCategory::Image => {
                let image = options.image.get_or_insert_with(Default::default);
                if let Some(w) = image.width {
                    let next_w = ((w as f32) * 0.85).round() as u32;
                    if next_w >= 320 && next_w < w {
                        image.width = Some(next_w);
                        changed = true;
                    }
                } else {
                    image.width = Some(1600);
                    changed = true;
                }
            }
            FormatCategory::Audio => {
                let audio = options.audio.get_or_insert_with(Default::default);
                if let Some(ref bitrate) = audio.bitrate {
                    if let Some(next) = Self::reduce_kbps_string(bitrate) {
                        if next != *bitrate {
                            audio.bitrate = Some(next);
                            changed = true;
                        }
                    }
                } else {
                    audio.bitrate = Some("192k".to_string());
                    changed = true;
                }
            }
            FormatCategory::Document | FormatCategory::Data | FormatCategory::Ebook => {}
        }

        changed
    }

    fn reduce_kbps_string(value: &str) -> Option<String> {
        let trimmed = value.trim().to_ascii_lowercase();
        let numeric = trimmed.strip_suffix('k')?;
        let current = numeric.parse::<u16>().ok()?;
        let next = current.saturating_sub(24).max(64);
        Some(format!("{}k", next))
    }
}

#[cfg(test)]
mod tests {
    use super::ConvxEngine;
    use crate::types::{error::ConvxError, format::Format, options::ConversionOptions};
    use tempfile::TempDir;

    #[test]
    fn can_convert_expected_paths() {
        let engine = ConvxEngine::new();

        assert!(engine.can_convert(Format::Png, Format::WebP));
        assert!(engine.can_convert(Format::Mp4, Format::Webm));
        assert!(engine.can_convert(Format::Mp4, Format::Gif));
        assert!(engine.can_convert(Format::Mp4, Format::Mp3));
        assert!(engine.can_convert(Format::Mp3, Format::Flac));
    }

    #[test]
    fn can_convert_rejects_invalid_paths() {
        let engine = ConvxEngine::new();

        assert!(!engine.can_convert(Format::Png, Format::Mp4));
        assert!(!engine.can_convert(Format::Mp3, Format::Png));
        assert!(!engine.can_convert(Format::Wav, Format::Webm));
    }

    #[test]
    fn convert_returns_file_not_found_for_missing_input() {
        let engine = ConvxEngine::new();
        let temp = TempDir::new().expect("temp dir");
        let input = temp.path().join("missing.png");
        let output = temp.path().join("out.webp");

        let result = engine.convert(
            &input,
            &output,
            ConversionOptions {
                output_format: Format::WebP,
                ..Default::default()
            },
        );

        assert!(matches!(result, Err(ConvxError::FileNotFound { .. })));
    }

    #[test]
    fn convert_returns_output_already_exists_when_overwrite_disabled() {
        let engine = ConvxEngine::new();
        let temp = TempDir::new().expect("temp dir");
        let input = temp.path().join("input.png");
        let output = temp.path().join("output.webp");

        std::fs::write(&input, b"not-a-real-image").expect("write input");
        std::fs::write(&output, b"already-exists").expect("write output");

        let result = engine.convert(
            &input,
            &output,
            ConversionOptions {
                output_format: Format::WebP,
                overwrite: false,
                ..Default::default()
            },
        );

        assert!(matches!(
            result,
            Err(ConvxError::OutputAlreadyExists { .. })
        ));
    }

    #[test]
    fn can_convert_document_data_and_ebook_paths() {
        let engine = ConvxEngine::new();

        assert!(engine.can_convert(Format::Pdf, Format::Docx));
        assert!(engine.can_convert(Format::Json, Format::Yaml));
        assert!(engine.can_convert(Format::Mobi, Format::Epub));
        assert!(!engine.can_convert(Format::Epub, Format::Mobi));
    }

    #[test]
    fn can_convert_new_data_format_paths() {
        let engine = ConvxEngine::new();

        // TSV / JSONL
        assert!(engine.can_convert(Format::Tsv, Format::Csv));
        assert!(engine.can_convert(Format::Csv, Format::Tsv));
        assert!(engine.can_convert(Format::Jsonl, Format::Json));
        assert!(engine.can_convert(Format::Json, Format::Jsonl));
        assert!(engine.can_convert(Format::Jsonl, Format::Csv));

        // ML formats
        assert!(engine.can_convert(Format::Csv, Format::Parquet));
        assert!(engine.can_convert(Format::Parquet, Format::Csv));
        assert!(engine.can_convert(Format::Arrow, Format::Json));
        assert!(engine.can_convert(Format::Sqlite, Format::Csv));
        assert!(engine.can_convert(Format::Npy, Format::Csv));
        assert!(engine.can_convert(Format::Hdf5, Format::Json));

        // Data -> Document
        assert!(engine.can_convert(Format::Csv, Format::Html));
        assert!(engine.can_convert(Format::Json, Format::Html));
        assert!(engine.can_convert(Format::Csv, Format::Pdf));
        assert!(engine.can_convert(Format::Csv, Format::Md));

        // Should NOT work
        assert!(!engine.can_convert(Format::Npy, Format::Json));
        assert!(!engine.can_convert(Format::Csv, Format::Sqlite));
    }

    #[test]
    fn can_convert_cross_category_rejects() {
        let engine = ConvxEngine::new();

        // Image -> Audio: never valid
        assert!(!engine.can_convert(Format::Png, Format::Mp3));
        assert!(!engine.can_convert(Format::Jpg, Format::Wav));

        // Audio -> Image: never valid
        assert!(!engine.can_convert(Format::Mp3, Format::Png));
        assert!(!engine.can_convert(Format::Flac, Format::Jpg));

        // Document -> Video: never valid
        assert!(!engine.can_convert(Format::Pdf, Format::Mp4));

        // Audio -> Video: never valid
        assert!(!engine.can_convert(Format::Mp3, Format::Mp4));
    }

    #[test]
    fn can_convert_svg_to_raster_true() {
        let engine = ConvxEngine::new();
        assert!(engine.can_convert(Format::Svg, Format::Png));
        assert!(engine.can_convert(Format::Svg, Format::Jpg));
        assert!(engine.can_convert(Format::Svg, Format::WebP));
    }

    #[test]
    fn can_convert_raster_to_svg_false() {
        let engine = ConvxEngine::new();
        assert!(!engine.can_convert(Format::Png, Format::Svg));
        assert!(!engine.can_convert(Format::Jpg, Format::Svg));
    }

    #[test]
    fn convert_nonexistent_input() {
        let engine = ConvxEngine::new();
        let temp = TempDir::new().expect("temp dir");
        let result = engine.convert(
            &temp.path().join("does-not-exist.png"),
            &temp.path().join("out.jpg"),
            ConversionOptions {
                output_format: Format::Jpg,
                ..Default::default()
            },
        );
        assert!(matches!(result, Err(ConvxError::FileNotFound { .. })));
    }

    #[test]
    fn default_options_are_sensible() {
        let opts = ConversionOptions::default();
        assert_eq!(opts.output_format, Format::Png); // Default enum variant
        assert!(!opts.overwrite);
        assert!(opts.quality.is_none());
        assert!(opts.max_file_size.is_none());
        assert!(opts.image.is_none());
        assert!(opts.video.is_none());
        assert!(opts.audio.is_none());
    }

    #[test]
    fn engine_has_six_converters() {
        let engine = ConvxEngine::new();
        assert_eq!(engine.converters.len(), 6);
    }

    #[test]
    fn reduce_kbps_string_works() {
        assert_eq!(
            ConvxEngine::reduce_kbps_string("192k"),
            Some("168k".to_string())
        );
        assert_eq!(
            ConvxEngine::reduce_kbps_string("96k"),
            Some("72k".to_string())
        );
        assert_eq!(
            ConvxEngine::reduce_kbps_string("64k"),
            Some("64k".to_string())
        ); // can't go below 64
        assert_eq!(ConvxEngine::reduce_kbps_string("invalid"), None);
        assert_eq!(ConvxEngine::reduce_kbps_string(""), None);
    }
}
