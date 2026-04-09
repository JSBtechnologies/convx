use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum Format {
    // Images
    #[default]
    Png,
    #[serde(rename = "jpg", alias = "jpeg")]
    Jpg,
    WebP,
    Gif,
    Bmp,
    Tiff,
    Ico,
    Svg,
    Heic,
    Heif,
    Avif,
    // Video
    Mp4,
    Mov,
    Webm,
    Avi,
    Mkv,
    Wmv,
    Flv,
    M4v,
    Mpeg,
    Ts,
    // Audio
    Mp3,
    Wav,
    Flac,
    M4a,
    Aac,
    Ogg,
    Wma,
    Aiff,
    Opus,
    Ac3,
    // Documents
    Doc,
    Pdf,
    Docx,
    Pptx,
    Xlsx,
    Txt,
    Md,
    Html,
    // Data
    Csv,
    Json,
    Yaml,
    Xml,
    Parquet,
    #[serde(alias = "ndjson")]
    Jsonl,
    Tsv,
    #[serde(alias = "feather")]
    Arrow,
    #[serde(alias = "db")]
    Sqlite,
    Npy,
    Npz,
    #[serde(rename = "hdf5", alias = "h5")]
    Hdf5,
    // eBooks
    Epub,
    Mobi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatCategory {
    Image,
    Video,
    Audio,
    Document,
    Data,
    Ebook,
}

impl Format {
    pub fn all() -> &'static [Format] {
        const ALL_FORMATS: &[Format] = &[
            // Images
            Format::Png,
            Format::Jpg,
            Format::WebP,
            Format::Gif,
            Format::Bmp,
            Format::Tiff,
            Format::Ico,
            Format::Svg,
            Format::Heic,
            Format::Heif,
            Format::Avif,
            // Video
            Format::Mp4,
            Format::Mov,
            Format::Webm,
            Format::Avi,
            Format::Mkv,
            Format::Wmv,
            Format::Flv,
            Format::M4v,
            Format::Mpeg,
            Format::Ts,
            // Audio
            Format::Mp3,
            Format::Wav,
            Format::Flac,
            Format::M4a,
            Format::Aac,
            Format::Ogg,
            Format::Wma,
            Format::Aiff,
            Format::Opus,
            Format::Ac3,
            // Documents
            Format::Doc,
            Format::Pdf,
            Format::Docx,
            Format::Pptx,
            Format::Xlsx,
            Format::Txt,
            Format::Md,
            Format::Html,
            // Data
            Format::Csv,
            Format::Json,
            Format::Yaml,
            Format::Xml,
            Format::Parquet,
            Format::Jsonl,
            Format::Tsv,
            Format::Arrow,
            Format::Sqlite,
            Format::Npy,
            Format::Npz,
            Format::Hdf5,
            // eBooks
            Format::Epub,
            Format::Mobi,
        ];

        ALL_FORMATS
    }

    pub fn all_by_category(category: FormatCategory) -> Vec<Format> {
        Self::all()
            .iter()
            .copied()
            .filter(|format| format.category() == category)
            .collect()
    }

    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "png" => Some(Self::Png),
            "jpg" | "jpeg" => Some(Self::Jpg),
            "webp" => Some(Self::WebP),
            "gif" => Some(Self::Gif),
            "bmp" => Some(Self::Bmp),
            "tiff" | "tif" => Some(Self::Tiff),
            "ico" => Some(Self::Ico),
            "svg" => Some(Self::Svg),
            "heic" => Some(Self::Heic),
            "heif" => Some(Self::Heif),
            "avif" => Some(Self::Avif),
            "mp4" => Some(Self::Mp4),
            "mov" => Some(Self::Mov),
            "webm" => Some(Self::Webm),
            "avi" => Some(Self::Avi),
            "mkv" => Some(Self::Mkv),
            "wmv" => Some(Self::Wmv),
            "flv" => Some(Self::Flv),
            "m4v" => Some(Self::M4v),
            "mpeg" | "mpg" => Some(Self::Mpeg),
            "ts" => Some(Self::Ts),
            "mp3" => Some(Self::Mp3),
            "wav" => Some(Self::Wav),
            "flac" => Some(Self::Flac),
            "m4a" => Some(Self::M4a),
            "aac" => Some(Self::Aac),
            "ogg" => Some(Self::Ogg),
            "wma" => Some(Self::Wma),
            "aiff" | "aif" => Some(Self::Aiff),
            "opus" => Some(Self::Opus),
            "ac3" => Some(Self::Ac3),
            "doc" => Some(Self::Doc),
            "pdf" => Some(Self::Pdf),
            "docx" => Some(Self::Docx),
            "pptx" => Some(Self::Pptx),
            "xlsx" => Some(Self::Xlsx),
            "txt" => Some(Self::Txt),
            "md" | "markdown" => Some(Self::Md),
            "html" | "htm" => Some(Self::Html),
            "csv" => Some(Self::Csv),
            "json" => Some(Self::Json),
            "yaml" | "yml" => Some(Self::Yaml),
            "xml" => Some(Self::Xml),
            "parquet" => Some(Self::Parquet),
            "jsonl" | "ndjson" => Some(Self::Jsonl),
            "tsv" => Some(Self::Tsv),
            "arrow" | "feather" => Some(Self::Arrow),
            "sqlite" | "db" => Some(Self::Sqlite),
            "npy" => Some(Self::Npy),
            "npz" => Some(Self::Npz),
            "h5" | "hdf5" => Some(Self::Hdf5),
            "epub" => Some(Self::Epub),
            "mobi" => Some(Self::Mobi),
            _ => None,
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpg => "jpg",
            Self::WebP => "webp",
            Self::Gif => "gif",
            Self::Bmp => "bmp",
            Self::Tiff => "tiff",
            Self::Ico => "ico",
            Self::Svg => "svg",
            Self::Heic => "heic",
            Self::Heif => "heif",
            Self::Avif => "avif",
            Self::Mp4 => "mp4",
            Self::Mov => "mov",
            Self::Webm => "webm",
            Self::Avi => "avi",
            Self::Mkv => "mkv",
            Self::Wmv => "wmv",
            Self::Flv => "flv",
            Self::M4v => "m4v",
            Self::Mpeg => "mpeg",
            Self::Ts => "ts",
            Self::Mp3 => "mp3",
            Self::Wav => "wav",
            Self::Flac => "flac",
            Self::M4a => "m4a",
            Self::Aac => "aac",
            Self::Ogg => "ogg",
            Self::Wma => "wma",
            Self::Aiff => "aiff",
            Self::Opus => "opus",
            Self::Ac3 => "ac3",
            Self::Doc => "doc",
            Self::Pdf => "pdf",
            Self::Docx => "docx",
            Self::Pptx => "pptx",
            Self::Xlsx => "xlsx",
            Self::Txt => "txt",
            Self::Md => "md",
            Self::Html => "html",
            Self::Csv => "csv",
            Self::Json => "json",
            Self::Yaml => "yaml",
            Self::Xml => "xml",
            Self::Parquet => "parquet",
            Self::Jsonl => "jsonl",
            Self::Tsv => "tsv",
            Self::Arrow => "arrow",
            Self::Sqlite => "sqlite",
            Self::Npy => "npy",
            Self::Npz => "npz",
            Self::Hdf5 => "h5",
            Self::Epub => "epub",
            Self::Mobi => "mobi",
        }
    }

    pub fn category(&self) -> FormatCategory {
        match self {
            Self::Png
            | Self::Jpg
            | Self::WebP
            | Self::Gif
            | Self::Bmp
            | Self::Tiff
            | Self::Ico
            | Self::Svg
            | Self::Heic
            | Self::Heif
            | Self::Avif => FormatCategory::Image,

            Self::Mp4
            | Self::Mov
            | Self::Webm
            | Self::Avi
            | Self::Mkv
            | Self::Wmv
            | Self::Flv
            | Self::M4v
            | Self::Mpeg
            | Self::Ts => FormatCategory::Video,

            Self::Mp3
            | Self::Wav
            | Self::Flac
            | Self::M4a
            | Self::Aac
            | Self::Ogg
            | Self::Wma
            | Self::Aiff
            | Self::Opus
            | Self::Ac3 => FormatCategory::Audio,

            Self::Doc
            | Self::Pdf
            | Self::Docx
            | Self::Pptx
            | Self::Xlsx
            | Self::Txt
            | Self::Md
            | Self::Html => FormatCategory::Document,

            Self::Csv
            | Self::Json
            | Self::Yaml
            | Self::Xml
            | Self::Parquet
            | Self::Jsonl
            | Self::Tsv
            | Self::Arrow
            | Self::Sqlite
            | Self::Npy
            | Self::Npz
            | Self::Hdf5 => FormatCategory::Data,

            Self::Epub | Self::Mobi => FormatCategory::Ebook,
        }
    }

    pub fn detect(path: &Path) -> Option<Self> {
        path.extension()
            .and_then(|e| e.to_str())
            .and_then(Self::from_extension)
    }

    /// Returns all formats this format can be converted to
    pub fn convertible_targets(&self) -> Vec<Format> {
        Self::all()
            .iter()
            .filter(|&&target| {
                if target == *self {
                    return false;
                }
                match (self.category(), target.category()) {
                    (FormatCategory::Image, FormatCategory::Image) => {
                        // Raster -> SVG is not currently supported by the engine.
                        // (SVG -> raster conversions are supported.)
                        !matches!(target, Format::Svg)
                    }
                    (FormatCategory::Video, FormatCategory::Video) => true,
                    (FormatCategory::Video, FormatCategory::Image) if target == Format::Gif => true,
                    (FormatCategory::Video, FormatCategory::Audio) => true,
                    (FormatCategory::Audio, FormatCategory::Audio) => true,
                    (FormatCategory::Document, FormatCategory::Document) => {
                        matches!(
                            (*self, target),
                            (Format::Pdf, Format::Docx)
                                | (Format::Docx, Format::Pdf)
                                | (Format::Pdf, Format::Pptx)
                                | (Format::Pptx, Format::Pdf)
                                | (Format::Pdf, Format::Xlsx)
                                | (Format::Xlsx, Format::Pdf)
                                | (Format::Md, Format::Pdf)
                                | (Format::Md, Format::Html)
                                | (Format::Doc, Format::Pdf)
                                | (Format::Doc, Format::Docx)
                                | (Format::Txt, Format::Pdf)
                                | (Format::Txt, Format::Docx)
                                | (Format::Txt, Format::Html)
                                | (Format::Html, Format::Pdf)
                                | (Format::Html, Format::Docx)
                        )
                    }
                    (FormatCategory::Document, FormatCategory::Data)
                        if matches!(*self, Format::Xlsx) && matches!(target, Format::Csv) =>
                    {
                        true
                    }
                    (FormatCategory::Document, FormatCategory::Image)
                        if matches!(*self, Format::Pdf)
                            && matches!(target, Format::Png | Format::Jpg) =>
                    {
                        true
                    }
                    (FormatCategory::Image, FormatCategory::Document)
                        if matches!(target, Format::Pdf) =>
                    {
                        true
                    }
                    (FormatCategory::Data, FormatCategory::Data) => {
                        matches!(
                            (*self, target),
                            // Existing
                            (Format::Json, Format::Csv)
                                | (Format::Csv, Format::Json)
                                | (Format::Json, Format::Yaml)
                                | (Format::Yaml, Format::Json)
                                | (Format::Xml, Format::Json)
                                | (Format::Json, Format::Xml)
                                // TSV
                                | (Format::Tsv, Format::Csv)
                                | (Format::Csv, Format::Tsv)
                                // JSONL
                                | (Format::Jsonl, Format::Json)
                                | (Format::Json, Format::Jsonl)
                                | (Format::Jsonl, Format::Csv)
                                | (Format::Csv, Format::Jsonl)
                                // Parquet
                                | (Format::Parquet, Format::Csv)
                                | (Format::Csv, Format::Parquet)
                                | (Format::Parquet, Format::Json)
                                | (Format::Json, Format::Parquet)
                                // Arrow
                                | (Format::Arrow, Format::Csv)
                                | (Format::Csv, Format::Arrow)
                                | (Format::Arrow, Format::Json)
                                | (Format::Json, Format::Arrow)
                                // SQLite (one-way)
                                | (Format::Sqlite, Format::Csv)
                                | (Format::Sqlite, Format::Json)
                                // NPY/NPZ (one-way)
                                | (Format::Npy, Format::Csv)
                                | (Format::Npz, Format::Csv)
                                // HDF5 (one-way)
                                | (Format::Hdf5, Format::Csv)
                                | (Format::Hdf5, Format::Json)
                        )
                    }
                    (FormatCategory::Data, FormatCategory::Document) => {
                        // CSV -> XLSX (existing)
                        if matches!(*self, Format::Csv) && matches!(target, Format::Xlsx) {
                            return true;
                        }
                        // Data -> HTML, PDF (tabular text formats only)
                        if matches!(target, Format::Html | Format::Pdf)
                            && matches!(
                                *self,
                                Format::Json
                                    | Format::Csv
                                    | Format::Xml
                                    | Format::Yaml
                                    | Format::Tsv
                                    | Format::Jsonl
                            )
                        {
                            return true;
                        }
                        // Data -> Markdown (JSON and CSV only)
                        if matches!(target, Format::Md)
                            && matches!(*self, Format::Json | Format::Csv)
                        {
                            return true;
                        }
                        false
                    }
                    (FormatCategory::Ebook, FormatCategory::Ebook) => {
                        matches!((*self, target), (Format::Mobi, Format::Epub))
                    }
                    (FormatCategory::Ebook, FormatCategory::Document)
                        if matches!(*self, Format::Epub) && matches!(target, Format::Pdf) =>
                    {
                        true
                    }
                    _ => false,
                }
            })
            .copied()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{Format, FormatCategory};

    #[test]
    fn from_extension_supports_aliases_and_known_edges() {
        assert_eq!(Format::from_extension("jpeg"), Some(Format::Jpg));
        assert_eq!(Format::from_extension("tif"), Some(Format::Tiff));
        assert_eq!(Format::from_extension("aif"), Some(Format::Aiff));
        assert_eq!(Format::from_extension("htm"), Some(Format::Html));
        assert_eq!(Format::from_extension("markdown"), Some(Format::Md));
    }

    #[test]
    fn from_extension_unknown_values_are_none() {
        assert_eq!(Format::from_extension("xyz"), None);
        assert_eq!(Format::from_extension(""), None);
        assert_eq!(Format::from_extension("UNKNOWN"), None);
    }

    #[test]
    fn category_is_correct_for_representative_formats() {
        assert_eq!(Format::Png.category(), FormatCategory::Image);
        assert_eq!(Format::Mp4.category(), FormatCategory::Video);
        assert_eq!(Format::Mp3.category(), FormatCategory::Audio);
        assert_eq!(Format::Pdf.category(), FormatCategory::Document);
        assert_eq!(Format::Json.category(), FormatCategory::Data);
        assert_eq!(Format::Epub.category(), FormatCategory::Ebook);
    }

    #[test]
    fn extension_round_trip_for_all_formats() {
        for format in Format::all() {
            assert_eq!(Format::from_extension(format.extension()), Some(*format));
        }
    }

    #[test]
    fn convertible_targets_never_include_self() {
        for format in Format::all() {
            assert!(!format.convertible_targets().contains(format));
        }
    }

    #[test]
    fn image_targets_do_not_include_svg_for_raster_input() {
        let targets = Format::Png.convertible_targets();
        assert!(!targets.contains(&Format::Svg));
    }

    #[test]
    fn video_targets_include_gif() {
        let targets = Format::Mp4.convertible_targets();
        assert!(targets.contains(&Format::Gif));
    }

    #[test]
    fn video_targets_include_audio_formats() {
        let targets = Format::Mp4.convertible_targets();
        assert!(targets.contains(&Format::Mp3));
        assert!(targets.contains(&Format::Wav));
        assert!(targets.contains(&Format::Flac));
    }

    #[test]
    fn audio_targets_only_include_audio() {
        let targets = Format::Mp3.convertible_targets();
        assert!(targets
            .iter()
            .all(|f| f.category() == FormatCategory::Audio));
    }

    #[test]
    fn doc_converts_to_pdf_and_docx() {
        let targets = Format::Doc.convertible_targets();
        assert!(targets.contains(&Format::Pdf));
        assert!(targets.contains(&Format::Docx));
    }

    #[test]
    fn txt_converts_to_pdf_docx_html() {
        let targets = Format::Txt.convertible_targets();
        assert!(targets.contains(&Format::Pdf));
        assert!(targets.contains(&Format::Docx));
        assert!(targets.contains(&Format::Html));
    }

    #[test]
    fn html_converts_to_pdf_and_docx() {
        let targets = Format::Html.convertible_targets();
        assert!(targets.contains(&Format::Pdf));
        assert!(targets.contains(&Format::Docx));
    }

    #[test]
    fn image_formats_convert_to_pdf() {
        for format in &[
            Format::Png,
            Format::Jpg,
            Format::WebP,
            Format::Bmp,
            Format::Tiff,
            Format::Gif,
        ] {
            let targets = format.convertible_targets();
            assert!(
                targets.contains(&Format::Pdf),
                "{:?} should convert to PDF",
                format
            );
        }
    }

    #[test]
    fn serde_deserializes_jpeg_alias_to_jpg() {
        let parsed: Format = serde_json::from_str("\"jpeg\"").expect("format should parse");
        assert_eq!(parsed, Format::Jpg);
    }

    #[test]
    fn all_formats_count() {
        // If a new format is added to the enum, this test reminds us to update tests
        assert_eq!(Format::all().len(), 53);
    }

    #[test]
    fn all_by_category_sums_to_total() {
        let categories = [
            FormatCategory::Image,
            FormatCategory::Video,
            FormatCategory::Audio,
            FormatCategory::Document,
            FormatCategory::Data,
            FormatCategory::Ebook,
        ];
        let sum: usize = categories
            .iter()
            .map(|c| Format::all_by_category(*c).len())
            .sum();
        assert_eq!(sum, Format::all().len());
    }

    #[test]
    fn from_extension_is_case_insensitive() {
        assert_eq!(Format::from_extension("PNG"), Some(Format::Png));
        assert_eq!(Format::from_extension("Jpg"), Some(Format::Jpg));
        assert_eq!(Format::from_extension("WEBP"), Some(Format::WebP));
        assert_eq!(Format::from_extension("Mp4"), Some(Format::Mp4));
    }

    #[test]
    fn detect_with_various_paths() {
        use std::path::Path;
        assert_eq!(Format::detect(Path::new("/foo/bar.png")), Some(Format::Png));
        assert_eq!(Format::detect(Path::new("file.MP4")), Some(Format::Mp4));
        assert_eq!(Format::detect(Path::new("no-ext")), None);
        assert_eq!(Format::detect(Path::new("archive.tar.gz")), None);
    }

    #[test]
    fn convertible_targets_are_all_valid_formats() {
        for format in Format::all() {
            for target in format.convertible_targets() {
                assert!(
                    Format::all().contains(&target),
                    "{:?} target {:?} not in Format::all()",
                    format,
                    target
                );
            }
        }
    }

    #[test]
    fn svg_to_raster_is_supported_but_not_reverse() {
        // SVG -> raster should work (SVG is Image category, targets exclude Svg)
        let svg_targets = Format::Svg.convertible_targets();
        assert!(svg_targets.contains(&Format::Png));
        assert!(svg_targets.contains(&Format::Jpg));
        assert!(!svg_targets.contains(&Format::Svg));

        // Raster -> SVG is NOT supported
        assert!(!Format::Png.convertible_targets().contains(&Format::Svg));
        assert!(!Format::Jpg.convertible_targets().contains(&Format::Svg));
    }

    #[test]
    fn all_format_extensions_are_lowercase() {
        for format in Format::all() {
            let ext = format.extension();
            assert_eq!(
                ext,
                ext.to_lowercase(),
                "{:?} extension {:?} is not lowercase",
                format,
                ext
            );
        }
    }

    #[test]
    fn serde_roundtrip_all_formats() {
        for format in Format::all() {
            let json = serde_json::to_string(format).expect("serialize");
            let parsed: Format = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(*format, parsed, "roundtrip failed for {:?}", format);
        }
    }
}
