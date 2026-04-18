use crate::converters::{extract_tool_error, Converter};
use crate::types::{
    error::ConvxError,
    format::{Format, FormatCategory},
    options::ConversionOptions,
    result::{ConversionResult, ConversionStatus},
};
use crate::utils::deps::silent_command;
use crate::utils::DependencyChecker;
use chrono::Utc;
use std::path::Path;
use uuid::Uuid;

pub struct ImageConverter;

impl ImageConverter {
    fn quality_to_png_compression(quality: u8) -> u8 {
        let q = quality.clamp(1, 100) as f32;
        // Map 1..100 quality to compression 9..1 (higher quality => less aggressive compression)
        let compression = 9.0 - ((q - 1.0) / 99.0) * 8.0;
        compression.round().clamp(1.0, 9.0) as u8
    }

    fn convert_to_ico_with_ffmpeg(input: &Path, output: &Path) -> Result<(), ConvxError> {
        let ffmpeg = DependencyChecker::ffmpeg_executable().ok_or(ConvxError::FfmpegNotFound)?;

        let args = vec![
            "-i".to_string(),
            input.to_string_lossy().to_string(),
            "-vf".to_string(),
            "scale=256:256:force_original_aspect_ratio=decrease,pad=256:256:(ow-iw)/2:(oh-ih)/2:color=0x00000000".to_string(),
            "-frames:v".to_string(),
            "1".to_string(),
            output.to_string_lossy().to_string(),
        ];

        let status = silent_command(ffmpeg)
            .args(&args)
            .output()
            .map_err(|_| ConvxError::FfmpegNotFound)?;

        if !status.status.success() {
            let stderr = String::from_utf8_lossy(&status.stderr).to_string();
            tracing::debug!(stderr = %stderr, "ffmpeg ico conversion failed");
            return Err(ConvxError::ConversionFailed {
                reason: extract_tool_error(&stderr),
            });
        }

        Ok(())
    }

    fn convert_svg_to_png_with_vips(input: &Path, output_png: &Path) -> Result<(), ConvxError> {
        let vips = DependencyChecker::vips_executable().ok_or(ConvxError::VipsNotFound)?;

        let status = silent_command(vips)
            .arg("copy")
            .arg(input)
            .arg(output_png)
            .output()
            .map_err(|_| ConvxError::VipsNotFound)?;

        if !status.status.success() {
            let stderr = String::from_utf8_lossy(&status.stderr).to_string();
            tracing::debug!(stderr = %stderr, "vips svg->png conversion failed");
            return Err(ConvxError::ConversionFailed {
                reason: extract_tool_error(&stderr),
            });
        }

        Ok(())
    }

    fn build_vips_save_suffix(options: &ConversionOptions) -> String {
        let image_opts = options.image.as_ref();
        let mut parts: Vec<String> = Vec::new();

        if let Some(q) = options.quality {
            match options.output_format {
                Format::Jpg | Format::WebP | Format::Heic | Format::Heif | Format::Avif => {
                    parts.push(format!("Q={}", q));
                }
                Format::Png => {
                    let compression = Self::quality_to_png_compression(q);
                    parts.push(format!("compression={}", compression));
                }
                _ => {}
            }
        }

        // On Windows, bundled libheif often lacks HEVC encoder; use AV1 instead
        if cfg!(target_os = "windows")
            && matches!(options.output_format, Format::Heic | Format::Heif)
        {
            parts.push("compression=av1".to_string());
        }

        if image_opts.map(|o| o.strip_metadata).unwrap_or(false) {
            parts.push("strip".to_string());
        }

        if parts.is_empty() {
            String::new()
        } else {
            format!("[{}]", parts.join(","))
        }
    }
}

impl ImageConverter {
    pub fn convert(
        &self,
        input: &Path,
        output: &Path,
        options: &ConversionOptions,
    ) -> Result<ConversionResult, ConvxError> {
        let start = std::time::Instant::now();
        let input_size = std::fs::metadata(input)
            .map_err(|e| ConvxError::FileReadError {
                path: input.to_path_buf(),
                reason: e.to_string(),
            })?
            .len();

        let input_format = Format::detect(input).unwrap_or(Format::Png);

        if options.output_format == Format::Ico {
            if input_format == Format::Svg {
                let temp_png =
                    std::env::temp_dir().join(format!("convx-svg-ico-{}.png", Uuid::new_v4()));
                Self::convert_svg_to_png_with_vips(input, &temp_png)?;
                let result = Self::convert_to_ico_with_ffmpeg(&temp_png, output);
                let _ = std::fs::remove_file(&temp_png);
                result?;
            } else {
                Self::convert_to_ico_with_ffmpeg(input, output)?;
            }
        } else {
            let vips = DependencyChecker::vips_executable().ok_or(ConvxError::VipsNotFound)?;
            let image_opts = options.image.as_ref();
            let needs_resize = image_opts
                .map(|o| o.width.is_some() || o.height.is_some())
                .unwrap_or(false);

            let output_str = format!(
                "{}{}",
                output.display(),
                Self::build_vips_save_suffix(options)
            );

            if needs_resize {
                let mut cmd = silent_command(&vips);
                cmd.arg("thumbnail");
                cmd.arg(input);
                cmd.arg(&output_str);

                let width = image_opts.and_then(|o| o.width).unwrap_or(9999);
                cmd.arg(width.to_string());

                if let Some(h) = image_opts.and_then(|o| o.height) {
                    cmd.arg("--height");
                    cmd.arg(h.to_string());
                }

                let status = cmd.output().map_err(|_| ConvxError::VipsNotFound)?;

                if !status.status.success() {
                    let stderr = String::from_utf8_lossy(&status.stderr).to_string();
                    tracing::debug!(stderr = %stderr, "vips thumbnail failed");
                    return Err(ConvxError::ConversionFailed {
                        reason: extract_tool_error(&stderr),
                    });
                }
            } else {
                let mut cmd = silent_command(&vips);
                cmd.arg("copy");
                cmd.arg(input);
                cmd.arg(&output_str);

                let status = cmd.output().map_err(|_| ConvxError::VipsNotFound)?;

                if !status.status.success() {
                    let stderr = String::from_utf8_lossy(&status.stderr).to_string();
                    tracing::debug!(stderr = %stderr, "vips copy failed");
                    return Err(ConvxError::ConversionFailed {
                        reason: extract_tool_error(&stderr),
                    });
                }
            }
        }

        let output_size = std::fs::metadata(output)
            .map_err(|e| ConvxError::FileWriteError {
                path: output.to_path_buf(),
                reason: e.to_string(),
            })?
            .len();

        Ok(ConversionResult {
            id: Uuid::new_v4(),
            status: ConversionStatus::Completed,
            input_path: input.to_path_buf(),
            output_path: Some(output.to_path_buf()),
            input_format,
            output_format: options.output_format,
            input_size,
            output_size: Some(output_size),
            space_saved: Some(input_size as i64 - output_size as i64),
            duration_ms: start.elapsed().as_millis() as u64,
            error: None,
            timestamp: Utc::now(),
        })
    }

    pub fn can_convert(&self, from: Format, to: Format) -> bool {
        matches!(from.category(), FormatCategory::Image)
            && matches!(to.category(), FormatCategory::Image)
            && to != Format::Svg
    }
}

impl Converter for ImageConverter {
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
}

#[cfg(test)]
mod tests {
    use super::ImageConverter;

    #[test]
    fn png_compression_quality_boundaries() {
        assert_eq!(ImageConverter::quality_to_png_compression(1), 9);
        assert_eq!(ImageConverter::quality_to_png_compression(100), 1);
    }

    #[test]
    fn png_compression_is_monotonic() {
        let mut prev = ImageConverter::quality_to_png_compression(1);
        for q in 2..=100 {
            let current = ImageConverter::quality_to_png_compression(q);
            assert!(
                current <= prev,
                "compression should decrease as quality increases: q={}, prev={}, current={}",
                q,
                prev,
                current
            );
            prev = current;
        }
    }

    #[test]
    fn png_compression_mid_range() {
        let mid = ImageConverter::quality_to_png_compression(50);
        assert!(
            (4..=6).contains(&mid),
            "mid quality should give moderate compression, got {}",
            mid
        );
    }

    #[test]
    fn png_compression_clamps_extreme_values() {
        // Values below 1 or above 100 should be clamped
        assert_eq!(ImageConverter::quality_to_png_compression(0), 9);
        assert_eq!(ImageConverter::quality_to_png_compression(255), 1);
    }

    #[test]
    fn can_convert_image_to_image() {
        let converter = ImageConverter;
        use crate::types::format::Format;
        assert!(converter.can_convert(Format::Png, Format::Jpg));
        assert!(converter.can_convert(Format::Jpg, Format::WebP));
        assert!(converter.can_convert(Format::Svg, Format::Png));
    }

    #[test]
    fn cannot_convert_to_svg() {
        let converter = ImageConverter;
        use crate::types::format::Format;
        assert!(!converter.can_convert(Format::Png, Format::Svg));
        assert!(!converter.can_convert(Format::Jpg, Format::Svg));
    }

    #[test]
    fn cannot_convert_across_categories() {
        let converter = ImageConverter;
        use crate::types::format::Format;
        assert!(!converter.can_convert(Format::Png, Format::Mp4));
        assert!(!converter.can_convert(Format::Mp4, Format::Png));
        assert!(!converter.can_convert(Format::Png, Format::Mp3));
    }
}
