use crate::converters::{extract_tool_error, Converter};
use crate::types::{
    error::ConvxError,
    format::{Format, FormatCategory},
    options::ConversionOptions,
    result::{ConversionResult, ConversionStatus},
};
use crate::utils::DependencyChecker;
use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};
use crate::utils::deps::silent_command;
use uuid::Uuid;

pub struct DocumentConverter;

impl DocumentConverter {
    // ─── Pandoc-based conversions (primary path) ────────────────────────

    /// General Pandoc conversion for non-PDF targets (HTML, DOCX, etc.).
    fn convert_with_pandoc(
        input: &Path,
        output: &Path,
        from_flag: Option<&str>,
        _target: Format,
    ) -> Result<u64, ConvxError> {
        let pandoc =
            DependencyChecker::pandoc_executable().ok_or(ConvxError::ConversionFailed {
                reason: "pandoc not found. Install pandoc for document conversion.".to_string(),
            })?;

        let mut cmd = silent_command(&pandoc);

        if let Some(from) = from_flag {
            cmd.arg(format!("--from={}", from));
        }

        cmd.arg(input).arg("-o").arg(output).arg("--standalone");

        DependencyChecker::set_lib_env(&mut cmd);

        let status = cmd.output().map_err(|e| ConvxError::ConversionFailed {
            reason: format!("Failed to execute pandoc: {}", e),
        })?;

        if !status.status.success() {
            let stderr = String::from_utf8_lossy(&status.stderr).to_string();
            return Err(ConvxError::ConversionFailed {
                reason: extract_tool_error(&stderr),
            });
        }

        let size = fs::metadata(output)
            .map_err(|e| ConvxError::FileWriteError {
                path: output.to_path_buf(),
                reason: e.to_string(),
            })?
            .len();
        Ok(size)
    }

    /// Two-step PDF generation: pandoc → HTML → weasyprint → PDF.
    /// Modern pandoc (3.9+) only accepts whitelisted --pdf-engine names, so
    /// we can't pass a custom wrapper script path. Instead, we convert to HTML
    /// first then use weasyprint directly as a subprocess with correct lib env.
    fn convert_to_pdf_via_weasyprint(
        input: &Path,
        output: &Path,
        from_flag: Option<&str>,
    ) -> Result<u64, ConvxError> {
        let pandoc =
            DependencyChecker::pandoc_executable().ok_or(ConvxError::ConversionFailed {
                reason: "pandoc not found. Install pandoc for document conversion.".to_string(),
            })?;

        let weasyprint =
            DependencyChecker::weasyprint_executable().ok_or(ConvxError::ConversionFailed {
                reason: "weasyprint not found. Install: pip install weasyprint".to_string(),
            })?;

        // Step 1: pandoc → temp HTML
        let temp_html = std::env::temp_dir().join(format!("convx-pandoc-{}.html", Uuid::new_v4()));

        let mut pandoc_cmd = silent_command(&pandoc);
        if let Some(from) = from_flag {
            pandoc_cmd.arg(format!("--from={}", from));
        }
        pandoc_cmd
            .arg(input)
            .arg("-o")
            .arg(&temp_html)
            .arg("--standalone");
        DependencyChecker::set_lib_env(&mut pandoc_cmd);

        let pandoc_out = pandoc_cmd
            .output()
            .map_err(|e| ConvxError::ConversionFailed {
                reason: format!("Failed to execute pandoc: {}", e),
            })?;

        if !pandoc_out.status.success() {
            let stderr = String::from_utf8_lossy(&pandoc_out.stderr).to_string();
            let _ = fs::remove_file(&temp_html);
            return Err(ConvxError::ConversionFailed {
                reason: extract_tool_error(&stderr),
            });
        }

        // Step 2: weasyprint HTML → PDF
        let mut wp_cmd = silent_command(&weasyprint);
        wp_cmd.arg(&temp_html).arg(output);
        DependencyChecker::set_lib_env(&mut wp_cmd);

        let wp_out = wp_cmd.output().map_err(|e| ConvxError::ConversionFailed {
            reason: format!("Failed to execute weasyprint: {}", e),
        })?;

        let _ = fs::remove_file(&temp_html);

        if !wp_out.status.success() {
            let stderr = String::from_utf8_lossy(&wp_out.stderr).to_string();
            return Err(ConvxError::ConversionFailed {
                reason: extract_tool_error(&stderr),
            });
        }

        let size = fs::metadata(output)
            .map_err(|e| ConvxError::FileWriteError {
                path: output.to_path_buf(),
                reason: e.to_string(),
            })?
            .len();
        Ok(size)
    }

    /// Direct weasyprint HTML → PDF (no pandoc intermediate step needed).
    fn convert_html_to_pdf(input: &Path, output: &Path) -> Result<u64, ConvxError> {
        let weasyprint =
            DependencyChecker::weasyprint_executable().ok_or(ConvxError::ConversionFailed {
                reason: "weasyprint not found. Install: pip install weasyprint".to_string(),
            })?;

        let mut cmd = silent_command(&weasyprint);
        cmd.arg(input).arg(output);
        DependencyChecker::set_lib_env(&mut cmd);

        let out = cmd.output().map_err(|e| ConvxError::ConversionFailed {
            reason: format!("Failed to execute weasyprint: {}", e),
        })?;

        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            return Err(ConvxError::ConversionFailed {
                reason: extract_tool_error(&stderr),
            });
        }

        let size = fs::metadata(output)
            .map_err(|e| ConvxError::FileWriteError {
                path: output.to_path_buf(),
                reason: e.to_string(),
            })?
            .len();
        Ok(size)
    }

    // ─── Python pdf2docx (PDF → DOCX) ──────────────────────────────────

    fn convert_pdf_to_docx(input: &Path, output: &Path) -> Result<u64, ConvxError> {
        // Try pdf2docx first (higher quality), fall back to LibreOffice
        if let Some(python) = DependencyChecker::convx_python() {
            let script = format!(
                "from pdf2docx import Converter; cv = Converter(r'{}'); cv.convert(r'{}'); cv.close()",
                input.display(),
                output.display()
            );

            let mut cmd = silent_command(python);
            cmd.args(["-c", &script]);
            DependencyChecker::set_lib_env(&mut cmd);

            if let Ok(status) = cmd.output() {
                if status.status.success() {
                    let size = fs::metadata(output)
                        .map_err(|e| ConvxError::FileWriteError {
                            path: output.to_path_buf(),
                            reason: e.to_string(),
                        })?
                        .len();
                    return Ok(size);
                }
            }
        }

        // Fallback: LibreOffice
        Self::convert_with_libreoffice(input, output, Format::Docx)
    }

    // ─── LibreOffice fallback (DOC, PPTX, XLSX only) ───────────────────

    fn convert_with_libreoffice(
        input: &Path,
        output: &Path,
        target: Format,
    ) -> Result<u64, ConvxError> {
        let soffice =
            DependencyChecker::libreoffice_executable().ok_or(ConvxError::ConversionFailed {
                reason: "LibreOffice not found. Install libreoffice for DOC/PPTX/XLSX conversion."
                    .to_string(),
            })?;

        let out_dir = output
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));

        let status = silent_command(soffice)
            .args([
                "--headless",
                "--convert-to",
                target.extension(),
                "--outdir",
                &out_dir.to_string_lossy(),
            ])
            .arg(input)
            .output()
            .map_err(|e| ConvxError::ConversionFailed {
                reason: format!("Failed to execute LibreOffice: {}", e),
            })?;

        if !status.status.success() {
            let stderr = String::from_utf8_lossy(&status.stderr).to_string();
            return Err(ConvxError::ConversionFailed {
                reason: extract_tool_error(&stderr),
            });
        }

        let generated = out_dir.join(format!(
            "{}.{}",
            input
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("output"),
            target.extension()
        ));

        if generated != output && generated.exists() {
            fs::rename(&generated, output).map_err(|e| ConvxError::FileWriteError {
                path: output.to_path_buf(),
                reason: e.to_string(),
            })?;
        }

        let size = fs::metadata(output)
            .map_err(|e| ConvxError::FileWriteError {
                path: output.to_path_buf(),
                reason: e.to_string(),
            })?
            .len();

        Ok(size)
    }

    // ─── PDF to images (poppler pdftoppm) ───────────────────────────────

    fn parse_page_range(options: &ConversionOptions) -> (Option<u32>, Option<u32>) {
        options
            .document
            .as_ref()
            .map(|d| (d.page_start, d.page_end))
            .unwrap_or((None, None))
    }

    fn pdf_to_images(
        input: &Path,
        output: &Path,
        options: &ConversionOptions,
    ) -> Result<u64, ConvxError> {
        let pdftoppm =
            DependencyChecker::pdftoppm_executable().ok_or(ConvxError::ConversionFailed {
                reason: "pdftoppm not found. Install poppler for PDF image export.".to_string(),
            })?;

        let out_dir = output
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        fs::create_dir_all(&out_dir).map_err(|e| ConvxError::FileWriteError {
            path: out_dir.clone(),
            reason: e.to_string(),
        })?;

        let stem = output
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("page")
            .to_string();
        let prefix = out_dir.join(&stem);

        let mut cmd = silent_command(pdftoppm);
        cmd.arg(input);

        let (start, end) = Self::parse_page_range(options);
        if let Some(s) = start {
            cmd.arg("-f").arg(s.to_string());
        }
        if let Some(e) = end {
            cmd.arg("-l").arg(e.to_string());
        }

        match options.output_format {
            Format::Png => {
                cmd.arg("-png");
            }
            Format::Jpg => {
                cmd.arg("-jpeg");
            }
            _ => {}
        }

        cmd.arg(prefix.to_string_lossy().to_string());

        let out = cmd.output().map_err(|e| ConvxError::ConversionFailed {
            reason: format!("Failed to execute pdftoppm: {}", e),
        })?;

        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            return Err(ConvxError::ConversionFailed {
                reason: extract_tool_error(&stderr),
            });
        }

        let expected_ext = options.output_format.extension();
        let prefix_name = prefix
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("page")
            .to_string();

        let mut total = 0u64;
        for entry in fs::read_dir(&out_dir).map_err(|e| ConvxError::FileReadError {
            path: out_dir.clone(),
            reason: e.to_string(),
        })? {
            let entry = entry.map_err(|e| ConvxError::FileReadError {
                path: out_dir.clone(),
                reason: e.to_string(),
            })?;
            let p = entry.path();
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("");
            if name.starts_with(&format!("{}-", prefix_name))
                && ext.eq_ignore_ascii_case(expected_ext)
            {
                total += fs::metadata(&p)
                    .map_err(|e| ConvxError::FileReadError {
                        path: p.clone(),
                        reason: e.to_string(),
                    })?
                    .len();
            }
        }

        if total == 0 {
            return Err(ConvxError::ConversionFailed {
                reason: "No page images were generated from PDF".to_string(),
            });
        }

        Ok(total)
    }

    // ─── Image → PDF (pure Rust, no external tools) ────────────────────

    fn convert_image_to_pdf(input: &Path, output: &Path) -> Result<u64, ConvxError> {
        use ::image::GenericImageView;

        let input_format = Format::detect(input).unwrap_or(Format::Png);
        // Formats the `image` crate can't decode — fall back to vips → temp PNG
        let needs_vips = matches!(
            input_format,
            Format::Heic | Format::Heif | Format::Avif | Format::Svg
        );

        let (img, temp_path) = if needs_vips {
            let vips = DependencyChecker::vips_executable().ok_or(ConvxError::VipsNotFound)?;
            let temp_png =
                std::env::temp_dir().join(format!("convx-img2pdf-{}.png", Uuid::new_v4()));

            let status = silent_command(vips)
                .arg("copy")
                .arg(input)
                .arg(&temp_png)
                .output()
                .map_err(|_| ConvxError::VipsNotFound)?;

            if !status.status.success() {
                let stderr = String::from_utf8_lossy(&status.stderr).to_string();
                return Err(ConvxError::ConversionFailed {
                    reason: extract_tool_error(&stderr),
                });
            }

            let img = ::image::open(&temp_png).map_err(|e| ConvxError::ConversionFailed {
                reason: format!("Failed to decode image: {}", e),
            })?;
            (img, Some(temp_png))
        } else {
            let img = ::image::open(input).map_err(|e| ConvxError::ConversionFailed {
                reason: format!("Failed to decode image: {}", e),
            })?;
            (img, None)
        };

        let (img_w, img_h) = img.dimensions();
        let rgb_image = img.to_rgb8();

        // Page sized to image at 150 DPI
        let dpi = 150.0_f32;
        let page_w = printpdf::Mm(img_w as f32 / dpi * 25.4);
        let page_h = printpdf::Mm(img_h as f32 / dpi * 25.4);

        let mut doc = printpdf::PdfDocument::new("Converted by ConvX");

        let image_id = doc.add_image(&printpdf::RawImage {
            pixels: printpdf::RawImageData::U8(rgb_image.into_raw()),
            width: img_w as usize,
            height: img_h as usize,
            data_format: printpdf::RawImageFormat::RGB8,
            tag: Vec::new(),
        });

        let page = printpdf::PdfPage::new(
            page_w,
            page_h,
            vec![printpdf::Op::UseXobject {
                id: image_id,
                transform: printpdf::XObjectTransform {
                    translate_x: Some(printpdf::Pt(0.0)),
                    translate_y: Some(printpdf::Pt(0.0)),
                    dpi: Some(dpi),
                    ..Default::default()
                },
            }],
        );

        doc.with_pages(vec![page]);

        let mut warnings = Vec::new();
        let pdf_bytes = doc.save(&printpdf::PdfSaveOptions::default(), &mut warnings);

        fs::write(output, &pdf_bytes).map_err(|e| ConvxError::FileWriteError {
            path: output.to_path_buf(),
            reason: e.to_string(),
        })?;

        if let Some(temp) = temp_path {
            let _ = fs::remove_file(temp);
        }

        Ok(pdf_bytes.len() as u64)
    }

    // ─── Public interface ───────────────────────────────────────────────

    pub fn can_convert(&self, from: Format, to: Format) -> bool {
        matches!(
            (from, to),
            (Format::Pdf, Format::Docx)
                | (Format::Docx, Format::Pdf)
                | (Format::Pdf, Format::Pptx)
                | (Format::Pptx, Format::Pdf)
                | (Format::Pdf, Format::Xlsx)
                | (Format::Xlsx, Format::Pdf)
                | (Format::Md, Format::Pdf)
                | (Format::Md, Format::Html)
                | (Format::Pdf, Format::Png)
                | (Format::Pdf, Format::Jpg)
                | (Format::Doc, Format::Pdf)
                | (Format::Doc, Format::Docx)
                | (Format::Txt, Format::Pdf)
                | (Format::Txt, Format::Docx)
                | (Format::Txt, Format::Html)
                | (Format::Html, Format::Pdf)
                | (Format::Html, Format::Docx)
                | (Format::Epub, Format::Pdf)
        ) || (from.category() == FormatCategory::Image && to == Format::Pdf)
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

        let output_size = match (input_format, options.output_format) {
            // ── Pandoc → PDF (two-step: pandoc→HTML, weasyprint→PDF) ──
            (Format::Md | Format::Docx | Format::Epub, Format::Pdf) => {
                Self::convert_to_pdf_via_weasyprint(input, output, None)?
            }
            (Format::Txt, Format::Pdf) => Self::convert_to_pdf_via_weasyprint(input, output, None)?,
            (Format::Html, Format::Pdf) => {
                // HTML can go straight to weasyprint without pandoc
                Self::convert_html_to_pdf(input, output)?
            }

            // ── Pandoc non-PDF conversions ───────────────────────────
            (Format::Md, Format::Html) => {
                Self::convert_with_pandoc(input, output, None, options.output_format)?
            }
            (Format::Txt, Format::Html | Format::Docx) => {
                // Pandoc auto-detects .txt — no --from flag needed
                Self::convert_with_pandoc(input, output, None, options.output_format)?
            }
            (Format::Html, Format::Docx) => {
                Self::convert_with_pandoc(input, output, None, options.output_format)?
            }

            // ── Python pdf2docx ─────────────────────────────────────
            (Format::Pdf, Format::Docx) => Self::convert_pdf_to_docx(input, output)?,

            // ── PDF to images (poppler) ─────────────────────────────
            (Format::Pdf, Format::Png | Format::Jpg) => {
                Self::pdf_to_images(input, output, options)?
            }

            // ── Image to PDF (pure Rust) ────────────────────────────
            (f, Format::Pdf) if f.category() == FormatCategory::Image => {
                Self::convert_image_to_pdf(input, output)?
            }

            // ── LibreOffice fallback (DOC→*, PPTX↔PDF, XLSX↔PDF) ──
            _ => Self::convert_with_libreoffice(input, output, options.output_format)?,
        };

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

impl Converter for DocumentConverter {
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
