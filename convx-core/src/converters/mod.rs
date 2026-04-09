pub mod audio;
pub mod data;
pub mod document;
pub mod ebook;
pub mod image;
pub mod video;

use crate::types::{
    error::ConvxError, format::Format, options::ConversionOptions, result::ConversionResult,
};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

pub trait Converter: Send + Sync {
    fn can_convert(&self, from: Format, to: Format) -> bool;

    fn convert(
        &self,
        input: &Path,
        output: &Path,
        options: &ConversionOptions,
    ) -> Result<ConversionResult, ConvxError>;

    fn convert_with_progress(
        &self,
        input: &Path,
        output: &Path,
        options: &ConversionOptions,
        on_progress: &mut dyn FnMut(f32),
        cancel_flag: Option<&AtomicBool>,
    ) -> Result<ConversionResult, ConvxError> {
        if let Some(flag) = cancel_flag {
            if flag.load(Ordering::Relaxed) {
                return Err(ConvxError::Cancelled);
            }
        }

        on_progress(0.5);
        let result = self.convert(input, output, options)?;
        on_progress(1.0);
        Ok(result)
    }
}

pub(crate) fn extract_tool_error(stderr: &str) -> String {
    // First try to find a meaningful error line
    if let Some(meaningful) = stderr.lines().rev().find_map(|line| {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }

        let lower = trimmed.to_lowercase();
        let is_meaningful = lower.contains("error")
            || lower.contains("invalid")
            || lower.contains("no such")
            || lower.contains("failed")
            || lower.contains("unsupported")
            || lower.contains("cannot")
            || lower.contains("exception")
            || lower.contains("traceback")
            || lower.contains("warning")
            || lower.contains("abort")
            || lower.contains("denied")
            || lower.contains("not found")
            || lower.contains("missing")
            || lower.contains("permission");

        if is_meaningful {
            Some(trimmed.to_string())
        } else {
            None
        }
    }) {
        return meaningful;
    }

    // Fall back to the last non-empty line of stderr
    if let Some(last_line) = stderr.lines().rev().find(|l| !l.trim().is_empty()) {
        return last_line.trim().to_string();
    }

    // If stderr is completely empty, say so
    "Conversion failed (tool produced no error output)".to_string()
}

pub use audio::AudioConverter;
pub use data::DataConverter;
pub use document::DocumentConverter;
pub use ebook::EbookConverter;
pub use image::ImageConverter;
pub use video::VideoConverter;

#[cfg(test)]
mod tests {
    use super::extract_tool_error;

    #[test]
    fn extract_tool_error_finds_error_line() {
        let stderr = "Processing...\nSome info\nError: file is corrupt\nDone.";
        assert_eq!(extract_tool_error(stderr), "Error: file is corrupt");
    }

    #[test]
    fn extract_tool_error_finds_failed_line() {
        let stderr = "Starting...\nConversion failed due to invalid input\n";
        assert_eq!(
            extract_tool_error(stderr),
            "Conversion failed due to invalid input"
        );
    }

    #[test]
    fn extract_tool_error_falls_back_to_last_line() {
        let stderr = "Line 1\nLine 2\nSome random output";
        assert_eq!(extract_tool_error(stderr), "Some random output");
    }

    #[test]
    fn extract_tool_error_empty_stderr() {
        assert_eq!(
            extract_tool_error(""),
            "Conversion failed (tool produced no error output)"
        );
    }

    #[test]
    fn extract_tool_error_whitespace_only() {
        assert_eq!(
            extract_tool_error("   \n  \n  "),
            "Conversion failed (tool produced no error output)"
        );
    }

    #[test]
    fn extract_tool_error_permission_denied() {
        let stderr = "some info\npermission denied: /etc/shadow\nmore stuff";
        assert_eq!(extract_tool_error(stderr), "permission denied: /etc/shadow");
    }
}
