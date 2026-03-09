use crate::converters::{extract_tool_error, Converter};
use crate::types::{
    error::ConvxError,
    format::{Format, FormatCategory},
    options::ConversionOptions,
    result::{ConversionResult, ConversionStatus},
};
use crate::utils::{DependencyChecker, FfprobeInfo};
use chrono::Utc;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use crate::utils::deps::silent_command;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use uuid::Uuid;

pub struct AudioConverter;

impl AudioConverter {
    fn quality_to_bitrate_kbps(quality: u8) -> u16 {
        let q = quality.clamp(1, 100) as f32;
        // Map 1..100 to ~96..320 kbps
        let bitrate = 96.0 + ((q - 1.0) / 99.0) * 224.0;
        bitrate.round().clamp(96.0, 320.0) as u16
    }

    fn build_args(input: &Path, output: &Path, options: &ConversionOptions) -> Vec<String> {
        let mut args: Vec<String> = Vec::new();

        // Overwrite behavior
        if options.overwrite {
            args.push("-y".to_string());
        } else {
            args.push("-n".to_string());
        }

        args.extend(["-i".to_string(), input.to_string_lossy().to_string()]);

        // Codec based on output format
        let codec = match options.output_format {
            Format::Mp3 => "libmp3lame",
            Format::Aac | Format::M4a => "aac",
            Format::Opus | Format::Ogg => "libopus",
            Format::Flac => "flac",
            Format::Wav => "pcm_s16le",
            Format::Aiff => "pcm_s16be",
            Format::Wma => "wmav2",
            Format::Ac3 => "ac3",
            _ => "copy",
        };
        args.extend(["-c:a".to_string(), codec.to_string()]);

        // Strip video track when extracting audio from video files
        if Format::detect(input)
            .map(|fmt| matches!(fmt.category(), FormatCategory::Video))
            .unwrap_or(false)
        {
            args.push("-vn".to_string());
        }

        // Bitrate (prefer explicit audio bitrate, otherwise derive from generic quality)
        let explicit_bitrate = options
            .audio
            .as_ref()
            .and_then(|audio| audio.bitrate.clone());

        if let Some(bitrate) = explicit_bitrate {
            args.extend(["-b:a".to_string(), bitrate]);
        } else {
            match options.output_format {
                Format::Mp3
                | Format::Aac
                | Format::M4a
                | Format::Opus
                | Format::Ogg
                | Format::Wma
                | Format::Aiff
                | Format::Ac3 => {
                    if let Some(q) = options.quality {
                        let mut kbps = Self::quality_to_bitrate_kbps(q);
                        if matches!(options.output_format, Format::Opus | Format::Ogg) {
                            kbps = kbps.clamp(32, 256);
                        }
                        args.extend(["-b:a".to_string(), format!("{}k", kbps)]);
                    }
                }
                _ => {}
            }
        }

        args.push(output.to_string_lossy().to_string());
        args
    }
}

impl AudioConverter {
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

        let args = Self::build_args(input, output, options);

        let ffmpeg = DependencyChecker::ffmpeg_executable().ok_or(ConvxError::FfmpegNotFound)?;

        let status = silent_command(ffmpeg)
            .args(&args)
            .output()
            .map_err(|_| ConvxError::FfmpegNotFound)?;

        if !status.status.success() {
            let stderr = String::from_utf8_lossy(&status.stderr).to_string();
            tracing::debug!(stderr = %stderr, "ffmpeg conversion failed");
            return Err(ConvxError::ConversionFailed {
                reason: extract_tool_error(&stderr),
            });
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
            input_format: Format::detect(input).unwrap_or(Format::Mp3),
            output_format: options.output_format,
            input_size,
            output_size: Some(output_size),
            space_saved: Some(input_size as i64 - output_size as i64),
            duration_ms: start.elapsed().as_millis() as u64,
            error: None,
            timestamp: Utc::now(),
        })
    }

    pub fn convert_with_progress(
        &self,
        input: &Path,
        output: &Path,
        options: &ConversionOptions,
        on_progress: &mut dyn FnMut(f32),
        cancel_flag: Option<&AtomicBool>,
    ) -> Result<ConversionResult, ConvxError> {
        let start = std::time::Instant::now();
        let input_size = std::fs::metadata(input)
            .map_err(|e| ConvxError::FileReadError {
                path: input.to_path_buf(),
                reason: e.to_string(),
            })?
            .len();

        let duration_us = FfprobeInfo::probe(input)
            .and_then(|p| p.duration_seconds())
            .map(|d| d * 1_000_000.0);

        let mut args = vec![
            "-v".to_string(),
            "error".to_string(),
            "-progress".to_string(),
            "pipe:1".to_string(),
            "-nostats".to_string(),
        ];
        args.extend(Self::build_args(input, output, options));

        let ffmpeg = DependencyChecker::ffmpeg_executable().ok_or(ConvxError::FfmpegNotFound)?;

        let mut child = silent_command(ffmpeg)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| ConvxError::FfmpegNotFound)?;

        let stderr_reader = child.stderr.take().map(|mut stderr| {
            std::thread::spawn(move || {
                let mut s = String::new();
                let _ = stderr.read_to_string(&mut s);
                s
            })
        });

        if let Some(stdout) = child.stdout.take() {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();

            loop {
                if let Some(flag) = cancel_flag {
                    if flag.load(Ordering::Relaxed) {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err(ConvxError::Cancelled);
                    }
                }

                line.clear();
                let bytes_read = reader.read_line(&mut line)?;
                if bytes_read == 0 {
                    break;
                }

                let trimmed = line.trim();
                if let Some((key, value)) = trimmed.split_once('=') {
                    if key == "out_time_us" || key == "out_time_ms" {
                        if let (Ok(current_us), Some(total_us)) =
                            (value.parse::<f64>(), duration_us)
                        {
                            let pct = (current_us / total_us).clamp(0.0, 1.0) as f32;
                            on_progress(pct);
                        }
                    } else if key == "progress" && value == "end" {
                        on_progress(1.0);
                    }
                }
            }
        }

        let status = child.wait()?;
        let stderr = stderr_reader
            .and_then(|h| h.join().ok())
            .unwrap_or_default();

        if !status.success() {
            tracing::debug!(stderr = %stderr, "ffmpeg conversion failed");
            return Err(ConvxError::ConversionFailed {
                reason: extract_tool_error(&stderr),
            });
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
            input_format: Format::detect(input).unwrap_or(Format::Mp3),
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
        let to_audio = matches!(to.category(), FormatCategory::Audio);
        let from_audio_or_video = matches!(
            from.category(),
            FormatCategory::Audio | FormatCategory::Video
        );
        to_audio && from_audio_or_video
    }
}

impl Converter for AudioConverter {
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

    fn convert_with_progress(
        &self,
        input: &Path,
        output: &Path,
        options: &ConversionOptions,
        on_progress: &mut dyn FnMut(f32),
        cancel_flag: Option<&AtomicBool>,
    ) -> Result<ConversionResult, ConvxError> {
        Self::convert_with_progress(self, input, output, options, on_progress, cancel_flag)
    }
}

#[cfg(test)]
mod tests {
    use super::AudioConverter;
    use crate::types::format::Format;

    #[test]
    fn quality_to_bitrate_boundaries() {
        assert_eq!(AudioConverter::quality_to_bitrate_kbps(1), 96);
        assert_eq!(AudioConverter::quality_to_bitrate_kbps(100), 320);
    }

    #[test]
    fn quality_to_bitrate_is_monotonic() {
        let mut prev = AudioConverter::quality_to_bitrate_kbps(1);
        for q in 2..=100 {
            let current = AudioConverter::quality_to_bitrate_kbps(q);
            assert!(
                current >= prev,
                "bitrate should increase as quality increases: q={}, prev={}, current={}",
                q,
                prev,
                current
            );
            prev = current;
        }
    }

    #[test]
    fn quality_50_reasonable_bitrate() {
        let bitrate = AudioConverter::quality_to_bitrate_kbps(50);
        assert!(
            (180..=220).contains(&bitrate),
            "mid quality should produce ~200kbps, got {}",
            bitrate
        );
    }

    #[test]
    fn all_quality_values_produce_valid_bitrate() {
        for q in 1..=100 {
            let bitrate = AudioConverter::quality_to_bitrate_kbps(q);
            assert!(
                bitrate >= 64,
                "bitrate should be >= 64kbps, q={} gave {}",
                q,
                bitrate
            );
            assert!(
                bitrate <= 320,
                "bitrate should be <= 320kbps, q={} gave {}",
                q,
                bitrate
            );
        }
    }

    #[test]
    fn can_convert_audio_to_audio() {
        let converter = AudioConverter;
        assert!(converter.can_convert(Format::Mp3, Format::Flac));
        assert!(converter.can_convert(Format::Wav, Format::Mp3));
        assert!(converter.can_convert(Format::Aac, Format::Opus));
    }

    #[test]
    fn can_convert_video_to_audio() {
        let converter = AudioConverter;
        assert!(converter.can_convert(Format::Mp4, Format::Mp3));
        assert!(converter.can_convert(Format::Mkv, Format::Flac));
    }

    #[test]
    fn cannot_convert_audio_to_non_audio() {
        let converter = AudioConverter;
        assert!(!converter.can_convert(Format::Mp3, Format::Png));
        assert!(!converter.can_convert(Format::Mp3, Format::Mp4));
        assert!(!converter.can_convert(Format::Png, Format::Mp3));
    }
}
