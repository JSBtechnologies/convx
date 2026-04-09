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

pub struct VideoConverter;

impl VideoConverter {
    fn quality_to_crf(quality: u8) -> u8 {
        let q = quality.clamp(1, 100) as f32;
        // Map 1..100 quality to CRF 35..18 (lower is better quality)
        let crf = 35.0 - ((q - 1.0) / 99.0) * 17.0;
        crf.round().clamp(18.0, 35.0) as u8
    }

    fn quality_to_vp9_crf(quality: u8) -> u8 {
        let q = quality.clamp(1, 100) as f32;
        // VP9 CRF commonly uses roughly 0..63; lower is better.
        let crf = 52.0 - ((q - 1.0) / 99.0) * 34.0;
        crf.round().clamp(18.0, 52.0) as u8
    }

    fn select_video_audio_codecs(output_format: Format) -> (&'static str, &'static str) {
        match output_format {
            Format::Webm => ("libvpx-vp9", "libopus"),
            Format::Avi => ("mpeg4", "mp3"),
            Format::Wmv => ("wmv2", "wmav2"),
            Format::Flv => ("flv", "mp3"),
            Format::Mpeg => ("mpeg2video", "mp2"),
            Format::Ts => ("libx264", "aac"),
            Format::Mp4 | Format::Mov | Format::M4v | Format::Mkv => ("libx264", "aac"),
            _ => ("libx264", "aac"),
        }
    }
}

impl VideoConverter {
    fn build_args(input: &Path, output: &Path, options: &ConversionOptions) -> Vec<String> {
        let mut args: Vec<String> = Vec::new();

        // Overwrite behavior
        if options.overwrite {
            args.push("-y".to_string());
        } else {
            args.push("-n".to_string());
        }

        args.extend(["-i".to_string(), input.to_string_lossy().to_string()]);

        // Handle GIF output specially
        if options.output_format == Format::Gif {
            let fps = options.video.as_ref().and_then(|v| v.fps).unwrap_or(10.0);
            let width = options.video.as_ref().and_then(|v| v.width).unwrap_or(480);

            args.extend([
                "-vf".to_string(),
                format!("fps={},scale={}:-1:flags=lanczos", fps, width),
            ]);
        } else {
            // Container-aware codec selection
            let (video_codec, audio_codec) = Self::select_video_audio_codecs(options.output_format);
            args.extend(["-c:v".to_string(), video_codec.to_string()]);

            // Quality handling per codec family
            if video_codec == "libx264" {
                let crf = options
                    .video
                    .as_ref()
                    .and_then(|v| v.crf)
                    .unwrap_or_else(|| options.quality.map(Self::quality_to_crf).unwrap_or(23));
                args.extend(["-crf".to_string(), crf.to_string()]);
            } else if video_codec == "libvpx-vp9" {
                let vp9_crf = options.quality.map(Self::quality_to_vp9_crf).unwrap_or(32);
                args.extend([
                    "-crf".to_string(),
                    vp9_crf.to_string(),
                    "-b:v".to_string(),
                    "0".to_string(),
                ]);
            }

            // Resolution
            if let Some(ref video) = options.video {
                if let (Some(w), Some(h)) = (video.width, video.height) {
                    args.extend(["-vf".to_string(), format!("scale={}:{}", w, h)]);
                }
            }

            // Audio
            if options.video.as_ref().map(|v| v.no_audio).unwrap_or(false) {
                args.push("-an".to_string());
            } else {
                args.extend(["-c:a".to_string(), audio_codec.to_string()]);
            }
        }

        args.push(output.to_string_lossy().to_string());
        args
    }

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
            input_format: Format::detect(input).unwrap_or(Format::Mp4),
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
            input_format: Format::detect(input).unwrap_or(Format::Mp4),
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
        let from_ok = matches!(from.category(), FormatCategory::Video);
        let to_ok = matches!(to.category(), FormatCategory::Video) || to == Format::Gif;
        from_ok && to_ok
    }
}

impl Converter for VideoConverter {
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
    use super::VideoConverter;
    use crate::types::format::Format;

    #[test]
    fn quality_to_crf_boundaries() {
        assert_eq!(VideoConverter::quality_to_crf(1), 35);
        assert_eq!(VideoConverter::quality_to_crf(100), 18);
        let mid = VideoConverter::quality_to_crf(50);
        assert!((26..=27).contains(&mid));
    }

    #[test]
    fn quality_to_vp9_crf_boundaries() {
        assert_eq!(VideoConverter::quality_to_vp9_crf(1), 52);
        assert_eq!(VideoConverter::quality_to_vp9_crf(100), 18);
    }

    #[test]
    fn codec_selection_matches_output_format() {
        assert_eq!(
            VideoConverter::select_video_audio_codecs(Format::Webm),
            ("libvpx-vp9", "libopus")
        );
        assert_eq!(
            VideoConverter::select_video_audio_codecs(Format::Avi),
            ("mpeg4", "mp3")
        );
        assert_eq!(
            VideoConverter::select_video_audio_codecs(Format::Mp4),
            ("libx264", "aac")
        );
    }

    #[test]
    fn quality_to_crf_is_monotonic() {
        let mut prev = VideoConverter::quality_to_crf(1);
        for q in 2..=100 {
            let current = VideoConverter::quality_to_crf(q);
            assert!(
                current <= prev,
                "CRF should decrease as quality increases: q={}, prev={}, current={}",
                q,
                prev,
                current
            );
            prev = current;
        }
    }

    #[test]
    fn quality_to_vp9_crf_is_monotonic() {
        let mut prev = VideoConverter::quality_to_vp9_crf(1);
        for q in 2..=100 {
            let current = VideoConverter::quality_to_vp9_crf(q);
            assert!(
                current <= prev,
                "VP9 CRF should decrease as quality increases: q={}, prev={}, current={}",
                q,
                prev,
                current
            );
            prev = current;
        }
    }

    #[test]
    fn quality_50_produces_reasonable_crf() {
        let crf = VideoConverter::quality_to_crf(50);
        assert!(
            (20..=30).contains(&crf),
            "mid quality CRF should be in reasonable range, got {}",
            crf
        );
        let vp9_crf = VideoConverter::quality_to_vp9_crf(50);
        assert!(
            (30..=40).contains(&vp9_crf),
            "mid quality VP9 CRF should be in reasonable range, got {}",
            vp9_crf
        );
    }

    #[test]
    fn can_convert_video_to_video() {
        let converter = VideoConverter;
        assert!(converter.can_convert(Format::Mp4, Format::Webm));
        assert!(converter.can_convert(Format::Mkv, Format::Avi));
    }

    #[test]
    fn can_convert_video_to_gif() {
        let converter = VideoConverter;
        assert!(converter.can_convert(Format::Mp4, Format::Gif));
        assert!(converter.can_convert(Format::Webm, Format::Gif));
    }

    #[test]
    fn cannot_convert_non_video() {
        let converter = VideoConverter;
        assert!(!converter.can_convert(Format::Png, Format::Mp4));
        assert!(!converter.can_convert(Format::Mp3, Format::Mp4));
        assert!(!converter.can_convert(Format::Mp4, Format::Png));
    }
}
