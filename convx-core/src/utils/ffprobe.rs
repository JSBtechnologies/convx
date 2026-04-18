use super::deps::silent_command;
use serde_json::Value;
use std::path::Path;

use crate::utils::DependencyChecker;

pub struct FfprobeInfo {
    raw: Value,
}

impl FfprobeInfo {
    /// Probe a file. Returns None if ffprobe is unavailable or the file can't be probed.
    pub fn probe(path: &Path) -> Option<Self> {
        let ffprobe = DependencyChecker::ffprobe_executable()?;
        let output = silent_command(ffprobe)
            .args([
                "-v",
                "quiet",
                "-print_format",
                "json",
                "-show_format",
                "-show_streams",
            ])
            .arg(path)
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let raw: Value = serde_json::from_slice(&output.stdout).ok()?;
        Some(Self { raw })
    }

    pub fn duration_seconds(&self) -> Option<f64> {
        self.raw
            .get("format")
            .and_then(|f| f.get("duration"))
            .and_then(Value::as_str)
            .and_then(|s| s.parse::<f64>().ok())
            .filter(|d| *d > 0.0)
    }

    pub fn dimensions(&self) -> (Option<u32>, Option<u32>) {
        let stream = self.video_stream().or_else(|| self.first_stream());
        match stream {
            Some(s) => (
                s.get("width")
                    .and_then(Value::as_u64)
                    .and_then(|v| u32::try_from(v).ok()),
                s.get("height")
                    .and_then(Value::as_u64)
                    .and_then(|v| u32::try_from(v).ok()),
            ),
            None => (None, None),
        }
    }

    pub fn fps(&self) -> Option<f64> {
        let video = self.video_stream()?;
        parse_fraction(
            video
                .get("r_frame_rate")
                .and_then(Value::as_str)
                .unwrap_or("0/0"),
        )
    }

    pub fn video_codec(&self) -> Option<String> {
        self.video_stream()
            .and_then(|s| s.get("codec_name"))
            .and_then(Value::as_str)
            .map(ToString::to_string)
    }

    pub fn audio_codec(&self) -> Option<String> {
        self.audio_stream()
            .and_then(|s| s.get("codec_name"))
            .and_then(Value::as_str)
            .map(ToString::to_string)
    }

    pub fn audio_sample_rate(&self) -> Option<u32> {
        self.audio_stream()
            .and_then(|s| s.get("sample_rate"))
            .and_then(Value::as_str)
            .and_then(|s| s.parse::<u32>().ok())
    }

    pub fn audio_channels(&self) -> Option<u32> {
        self.audio_stream()
            .and_then(|s| s.get("channels"))
            .and_then(Value::as_u64)
            .and_then(|v| u32::try_from(v).ok())
    }

    fn streams(&self) -> Option<&Vec<Value>> {
        self.raw.get("streams")?.as_array()
    }

    fn video_stream(&self) -> Option<&Value> {
        self.streams()?
            .iter()
            .find(|s| s.get("codec_type").and_then(Value::as_str) == Some("video"))
    }

    fn audio_stream(&self) -> Option<&Value> {
        self.streams()?
            .iter()
            .find(|s| s.get("codec_type").and_then(Value::as_str) == Some("audio"))
    }

    fn first_stream(&self) -> Option<&Value> {
        self.streams()?.first()
    }
}

fn parse_fraction(value: &str) -> Option<f64> {
    let (num, den) = value.split_once('/')?;
    let num = num.parse::<f64>().ok()?;
    let den = den.parse::<f64>().ok()?;
    if den == 0.0 {
        return None;
    }
    Some(num / den)
}
