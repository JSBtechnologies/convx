use crate::converters::{extract_tool_error, Converter};
use crate::types::{
    error::ConvxError,
    format::Format,
    options::ConversionOptions,
    result::{ConversionResult, ConversionStatus},
};
use crate::utils::DependencyChecker;
use chrono::Utc;
use std::fs;
use std::path::Path;
use std::process::Command;
use uuid::Uuid;

pub struct EbookConverter;

impl EbookConverter {
    pub fn can_convert(&self, from: Format, to: Format) -> bool {
        matches!((from, to), (Format::Mobi, Format::Epub))
    }

    pub fn convert(
        &self,
        input: &Path,
        output: &Path,
        options: &ConversionOptions,
    ) -> Result<ConversionResult, ConvxError> {
        let start = std::time::Instant::now();

        let input_size = fs::metadata(input)
            .map_err(|e| ConvxError::FileReadError {
                path: input.to_path_buf(),
                reason: e.to_string(),
            })?
            .len();

        let input_format =
            Format::detect(input).ok_or_else(|| ConvxError::FormatDetectionFailed {
                path: input.to_path_buf(),
            })?;

        if !self.can_convert(input_format, options.output_format) {
            return Err(ConvxError::UnsupportedConversion {
                from: input_format,
                to: options.output_format,
            });
        }

        let python = DependencyChecker::convx_python().ok_or(ConvxError::ConversionFailed {
            reason: "python3 not found. Install Python 3 for ebook conversion.".to_string(),
        })?;

        // Use the mobi pip package to extract MOBI to EPUB.
        // mobi.extract(infile) returns (tempdir, epub_path).
        let script = r#"
import sys, shutil
import mobi

tempdir, epub_path = mobi.extract(sys.argv[1])
try:
    if not epub_path:
        print('ERROR: mobi extraction did not produce an EPUB file', file=sys.stderr)
        sys.exit(1)
    shutil.copy2(epub_path, sys.argv[2])
finally:
    shutil.rmtree(tempdir, ignore_errors=True)
"#
        .to_string();

        let out = Command::new(&python)
            .args(["-c", &script])
            .arg(input)
            .arg(output)
            .output()
            .map_err(|e| ConvxError::ConversionFailed {
                reason: format!("Failed to execute mobi extraction: {}", e),
            })?;

        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            return Err(ConvxError::ConversionFailed {
                reason: extract_tool_error(&stderr),
            });
        }

        let output_size = fs::metadata(output)
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
}

impl Converter for EbookConverter {
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
