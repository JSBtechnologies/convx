//! Cross-module public API tests for convx-core.
//! These tests verify behavior spanning multiple modules via the public API.

use convx::{
    ConversionOptions, ConvxEngine, ConvxError, DependencyChecker, Format, FormatCategory,
};

// ── Format validation ────────────────────────────────────────────────

#[test]
fn all_formats_roundtrip_through_extension() {
    for format in Format::all() {
        let ext = format.extension();
        let parsed = Format::from_extension(ext);
        assert_eq!(
            parsed,
            Some(*format),
            "Format {:?} extension '{}' did not round-trip",
            format,
            ext
        );
    }
}

#[test]
fn all_format_extensions_are_lowercase() {
    for format in Format::all() {
        let ext = format.extension();
        assert_eq!(
            ext,
            ext.to_lowercase(),
            "{:?} has non-lowercase extension",
            format
        );
    }
}

#[test]
fn format_serde_roundtrip() {
    for format in Format::all() {
        let json = serde_json::to_string(format).expect("serialize");
        let parsed: Format = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*format, parsed, "serde roundtrip failed for {:?}", format);
    }
}

// ── Conversion matrix ────────────────────────────────────────────────

#[test]
fn conversion_matrix_symmetric_within_image_category() {
    let images = Format::all_by_category(FormatCategory::Image);
    for a in &images {
        for b in &images {
            if *a == *b || *a == Format::Svg || *b == Format::Svg {
                continue;
            }
            // Both directions should be valid for non-SVG image pairs
            assert!(
                a.convertible_targets().contains(b),
                "{:?} -> {:?} should be valid",
                a,
                b
            );
            assert!(
                b.convertible_targets().contains(a),
                "{:?} -> {:?} should be valid (reverse)",
                b,
                a
            );
        }
    }
}

#[test]
fn no_cross_category_false_positives_image_audio() {
    let engine = ConvxEngine::new();
    let images = Format::all_by_category(FormatCategory::Image);
    let audios = Format::all_by_category(FormatCategory::Audio);

    for img in &images {
        for aud in &audios {
            assert!(
                !engine.can_convert(*img, *aud),
                "image {:?} -> audio {:?} should not be convertible",
                img,
                aud
            );
        }
    }
}

#[test]
fn data_format_targets_include_document_outputs() {
    // CSV and JSON should be able to produce HTML and PDF
    assert!(Format::Csv.convertible_targets().contains(&Format::Html));
    assert!(Format::Csv.convertible_targets().contains(&Format::Pdf));
    assert!(Format::Json.convertible_targets().contains(&Format::Html));
    assert!(Format::Json.convertible_targets().contains(&Format::Pdf));

    // CSV and JSON -> Markdown
    assert!(Format::Csv.convertible_targets().contains(&Format::Md));
    assert!(Format::Json.convertible_targets().contains(&Format::Md));
}

#[test]
fn engine_can_convert_matches_convertible_targets() {
    let engine = ConvxEngine::new();

    for format in Format::all() {
        let targets = format.convertible_targets();
        for target in &targets {
            assert!(
                engine.can_convert(*format, *target),
                "engine.can_convert({:?}, {:?}) is false but {:?} lists {:?} as target",
                format,
                target,
                format,
                target
            );
        }
    }
}

// ── Presets + engine ─────────────────────────────────────────────────

#[test]
fn engine_preset_resolve_produces_valid_options() {
    let presets = convx::presets::built_in_presets();

    for preset in &presets {
        let base = ConversionOptions {
            output_format: preset.output_format,
            ..Default::default()
        };
        let resolved = convx::presets::resolve_options(base, Some(preset));
        assert_eq!(
            resolved.output_format, preset.output_format,
            "preset '{}' output format mismatch",
            preset.name
        );
    }
}

// ── DependencyChecker ────────────────────────────────────────────────

#[test]
fn get_versions_returns_nonempty() {
    let versions = DependencyChecker::get_versions();
    // Should at least contain ffmpeg, vips, pandoc, etc. entries
    assert!(
        !versions.is_empty(),
        "get_versions should return at least one tool"
    );
}

#[test]
fn check_all_does_not_panic() {
    // Just verify it doesn't panic — may not find tools in test env
    let _ = DependencyChecker::check_all();
}

// ── Error types ──────────────────────────────────────────────────────

#[test]
fn convx_error_display_is_readable() {
    let err = ConvxError::FileNotFound {
        path: std::path::PathBuf::from("/tmp/test.png"),
    };
    let msg = format!("{}", err);
    assert!(msg.contains("/tmp/test.png"), "error should contain path");
    assert!(
        msg.contains("not found"),
        "error should mention 'not found'"
    );

    let err = ConvxError::UnsupportedConversion {
        from: Format::Png,
        to: Format::Mp4,
    };
    let msg = format!("{}", err);
    assert!(msg.contains("Png"), "error should mention source format");
    assert!(msg.contains("Mp4"), "error should mention target format");
}

#[test]
fn convx_error_from_io_error() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "test");
    let err: ConvxError = io_err.into();
    let msg = format!("{}", err);
    assert!(msg.contains("IO error"));
}

#[test]
fn convx_error_unknown_preset() {
    let err = ConvxError::UnknownPreset {
        preset: "bogus".to_string(),
    };
    let msg = format!("{}", err);
    assert!(msg.contains("bogus"));
}

// ── Category counts ──────────────────────────────────────────────────

#[test]
fn category_counts_are_expected() {
    assert_eq!(Format::all_by_category(FormatCategory::Image).len(), 11);
    assert_eq!(Format::all_by_category(FormatCategory::Video).len(), 10);
    assert_eq!(Format::all_by_category(FormatCategory::Audio).len(), 10);
    assert_eq!(Format::all_by_category(FormatCategory::Document).len(), 8);
    assert_eq!(Format::all_by_category(FormatCategory::Data).len(), 12);
    assert_eq!(Format::all_by_category(FormatCategory::Ebook).len(), 2);
}
