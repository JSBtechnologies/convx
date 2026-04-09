use crate::types::preset::Preset;
use crate::{AudioOptions, ConversionOptions, ConvxError, Format, ImageOptions, VideoOptions};

pub fn built_in_presets() -> Vec<Preset> {
    vec![
        Preset {
            name: "discord",
            description: "Discord upload-friendly video (target: <8MB, best effort)",
            output_format: Format::Mp4,
            quality: Some(55),
            max_file_size: Some(8 * 1024 * 1024),
            video: Some(VideoOptions {
                crf: Some(28),
                ..Default::default()
            }),
            audio: None,
            image: None,
        },
        Preset {
            name: "discord-nitro",
            description: "Discord Nitro video quality preset (target: <50MB, best effort)",
            output_format: Format::Mp4,
            quality: Some(75),
            max_file_size: Some(50 * 1024 * 1024),
            video: Some(VideoOptions {
                crf: Some(23),
                ..Default::default()
            }),
            audio: None,
            image: None,
        },
        Preset {
            name: "twitter-image",
            description: "Twitter-optimized image export",
            output_format: Format::Jpg,
            quality: Some(85),
            max_file_size: None,
            video: None,
            audio: None,
            image: Some(ImageOptions {
                width: Some(1200),
                ..Default::default()
            }),
        },
        Preset {
            name: "twitter-gif",
            description: "Twitter-friendly GIF export",
            output_format: Format::Gif,
            quality: Some(70),
            max_file_size: None,
            video: Some(VideoOptions {
                width: Some(480),
                fps: Some(15.0),
                ..Default::default()
            }),
            audio: None,
            image: None,
        },
        Preset {
            name: "instagram-story",
            description: "Instagram Story vertical video preset",
            output_format: Format::Mp4,
            quality: Some(80),
            max_file_size: None,
            video: Some(VideoOptions {
                width: Some(1080),
                height: Some(1920),
                fps: Some(30.0),
                ..Default::default()
            }),
            audio: None,
            image: None,
        },
        Preset {
            name: "web-image",
            description: "General web image preset (WebP)",
            output_format: Format::WebP,
            quality: Some(80),
            max_file_size: None,
            video: None,
            audio: None,
            image: Some(ImageOptions {
                strip_metadata: true,
                ..Default::default()
            }),
        },
        Preset {
            name: "email-friendly",
            description: "Small email attachment image preset",
            output_format: Format::Jpg,
            quality: Some(75),
            max_file_size: Some(1024 * 1024),
            video: None,
            audio: None,
            image: Some(ImageOptions {
                width: Some(1200),
                strip_metadata: true,
                ..Default::default()
            }),
        },
        Preset {
            name: "heic-to-jpg",
            description: "Convert HEIC/HEIF photos to JPG",
            output_format: Format::Jpg,
            quality: Some(90),
            max_file_size: None,
            video: None,
            audio: None,
            image: None,
        },
        Preset {
            name: "archive-lossless",
            description: "Lossless archival image preset (PNG)",
            output_format: Format::Png,
            quality: Some(100),
            max_file_size: None,
            video: None,
            audio: None,
            image: None,
        },
        Preset {
            name: "extract-audio",
            description: "Extract audio track to MP3",
            output_format: Format::Mp3,
            quality: None,
            max_file_size: None,
            video: None,
            audio: Some(AudioOptions {
                bitrate: Some("192k".to_string()),
                ..Default::default()
            }),
            image: None,
        },
        Preset {
            name: "pdf-to-images",
            description: "Export PDF pages as PNG images",
            output_format: Format::Png,
            quality: None,
            max_file_size: None,
            video: None,
            audio: None,
            image: None,
        },
        Preset {
            name: "markdown-to-pdf",
            description: "Convert Markdown documents to PDF",
            output_format: Format::Pdf,
            quality: None,
            max_file_size: None,
            video: None,
            audio: None,
            image: None,
        },
        Preset {
            name: "json-to-csv",
            description: "Convert JSON array data to CSV",
            output_format: Format::Csv,
            quality: None,
            max_file_size: None,
            video: None,
            audio: None,
            image: None,
        },
        Preset {
            name: "epub-to-pdf",
            description: "Convert EPUB ebooks to PDF",
            output_format: Format::Pdf,
            quality: None,
            max_file_size: None,
            video: None,
            audio: None,
            image: None,
        },
        Preset {
            name: "parquet-to-csv",
            description: "Convert Parquet data files to CSV",
            output_format: Format::Csv,
            quality: None,
            max_file_size: None,
            video: None,
            audio: None,
            image: None,
        },
        Preset {
            name: "csv-to-parquet",
            description: "Convert CSV data to Parquet columnar format",
            output_format: Format::Parquet,
            quality: None,
            max_file_size: None,
            video: None,
            audio: None,
            image: None,
        },
        Preset {
            name: "jsonl-to-csv",
            description: "Convert JSON Lines training data to CSV",
            output_format: Format::Csv,
            quality: None,
            max_file_size: None,
            video: None,
            audio: None,
            image: None,
        },
        Preset {
            name: "data-to-pdf",
            description: "Render tabular data as a styled PDF table",
            output_format: Format::Pdf,
            quality: None,
            max_file_size: None,
            video: None,
            audio: None,
            image: None,
        },
    ]
}

pub fn get_preset(name: &str) -> Result<Preset, ConvxError> {
    built_in_presets()
        .into_iter()
        .find(|p| p.name.eq_ignore_ascii_case(name))
        .ok_or_else(|| ConvxError::UnknownPreset {
            preset: name.to_string(),
        })
}

pub fn resolve_options(base: ConversionOptions, preset: Option<&Preset>) -> ConversionOptions {
    let Some(preset) = preset else {
        return base;
    };

    let mut merged = base;

    if merged.quality.is_none() {
        merged.quality = preset.quality;
    }

    if merged.max_file_size.is_none() {
        merged.max_file_size = preset.max_file_size;
    }

    merged.image = merge_image_options(merged.image, preset.image.clone());
    merged.video = merge_video_options(merged.video, preset.video.clone());
    merged.audio = merge_audio_options(merged.audio, preset.audio.clone());

    merged
}

fn merge_image_options(
    cli: Option<ImageOptions>,
    preset: Option<ImageOptions>,
) -> Option<ImageOptions> {
    match (cli, preset) {
        (Some(mut cli), Some(preset)) => {
            if cli.width.is_none() {
                cli.width = preset.width;
            }
            if cli.height.is_none() {
                cli.height = preset.height;
            }
            cli.strip_metadata = cli.strip_metadata || preset.strip_metadata;
            Some(cli)
        }
        (Some(cli), None) => Some(cli),
        (None, Some(preset)) => Some(preset),
        (None, None) => None,
    }
}

fn merge_video_options(
    cli: Option<VideoOptions>,
    preset: Option<VideoOptions>,
) -> Option<VideoOptions> {
    match (cli, preset) {
        (Some(mut cli), Some(preset)) => {
            if cli.width.is_none() {
                cli.width = preset.width;
            }
            if cli.height.is_none() {
                cli.height = preset.height;
            }
            if cli.fps.is_none() {
                cli.fps = preset.fps;
            }
            if cli.crf.is_none() {
                cli.crf = preset.crf;
            }
            cli.no_audio = cli.no_audio || preset.no_audio;
            Some(cli)
        }
        (Some(cli), None) => Some(cli),
        (None, Some(preset)) => Some(preset),
        (None, None) => None,
    }
}

fn merge_audio_options(
    cli: Option<AudioOptions>,
    preset: Option<AudioOptions>,
) -> Option<AudioOptions> {
    match (cli, preset) {
        (Some(mut cli), Some(preset)) => {
            if cli.bitrate.is_none() {
                cli.bitrate = preset.bitrate;
            }
            if cli.sample_rate.is_none() {
                cli.sample_rate = preset.sample_rate;
            }
            Some(cli)
        }
        (Some(cli), None) => Some(cli),
        (None, Some(preset)) => Some(preset),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn built_in_presets_count() {
        assert_eq!(built_in_presets().len(), 18);
    }

    #[test]
    fn all_presets_have_unique_names() {
        let presets = built_in_presets();
        let names: HashSet<&str> = presets.iter().map(|p| p.name).collect();
        assert_eq!(names.len(), presets.len(), "duplicate preset names found");
    }

    #[test]
    fn all_presets_have_valid_output_format() {
        for preset in built_in_presets() {
            assert!(
                Format::all().contains(&preset.output_format),
                "preset '{}' has invalid output format {:?}",
                preset.name,
                preset.output_format
            );
        }
    }

    #[test]
    fn get_preset_by_name_all_18() {
        for preset in built_in_presets() {
            assert!(
                get_preset(preset.name).is_ok(),
                "get_preset('{}') should succeed",
                preset.name
            );
        }
    }

    #[test]
    fn get_preset_unknown_returns_error() {
        let result = get_preset("nonexistent-preset");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConvxError::UnknownPreset { .. }
        ));
    }

    #[test]
    fn get_preset_is_case_insensitive() {
        assert!(get_preset("DISCORD").is_ok());
        assert!(get_preset("Discord").is_ok());
        assert!(get_preset("discord").is_ok());
    }

    #[test]
    fn preset_descriptions_are_nonempty() {
        for preset in built_in_presets() {
            assert!(
                !preset.description.is_empty(),
                "preset '{}' has empty description",
                preset.name
            );
        }
    }

    #[test]
    fn resolve_options_preset_fills_missing_quality() {
        let base = ConversionOptions::default();
        let preset = get_preset("discord").unwrap();
        let merged = resolve_options(base, Some(&preset));
        assert_eq!(merged.quality, Some(55));
    }

    #[test]
    fn resolve_options_user_overrides_preset() {
        let base = ConversionOptions {
            quality: Some(90),
            ..Default::default()
        };
        let preset = get_preset("discord").unwrap();
        let merged = resolve_options(base, Some(&preset));
        assert_eq!(
            merged.quality,
            Some(90),
            "user quality should take precedence"
        );
    }

    #[test]
    fn resolve_options_preset_sets_max_file_size() {
        let base = ConversionOptions::default();
        let preset = get_preset("discord").unwrap();
        let merged = resolve_options(base, Some(&preset));
        assert_eq!(merged.max_file_size, Some(8 * 1024 * 1024));
    }

    #[test]
    fn resolve_options_no_preset_returns_base() {
        let base = ConversionOptions {
            quality: Some(42),
            ..Default::default()
        };
        let merged = resolve_options(base.clone(), None);
        assert_eq!(merged.quality, base.quality);
    }

    #[test]
    fn discord_preset_has_max_file_size() {
        let preset = get_preset("discord").unwrap();
        assert!(preset.max_file_size.is_some());
        assert!(preset.max_file_size.unwrap() <= 8 * 1024 * 1024);
    }

    #[test]
    fn extract_audio_preset_outputs_mp3() {
        let preset = get_preset("extract-audio").unwrap();
        assert_eq!(preset.output_format, Format::Mp3);
        assert!(preset.audio.is_some());
        assert_eq!(
            preset.audio.as_ref().unwrap().bitrate,
            Some("192k".to_string())
        );
    }

    #[test]
    fn merge_image_options_preset_fills_missing() {
        let cli = Some(ImageOptions {
            width: None,
            height: Some(600),
            strip_metadata: false,
        });
        let preset = Some(ImageOptions {
            width: Some(1200),
            height: Some(800),
            strip_metadata: true,
        });
        let merged = super::merge_image_options(cli, preset).unwrap();
        assert_eq!(merged.width, Some(1200)); // filled from preset
        assert_eq!(merged.height, Some(600)); // kept from cli
        assert!(merged.strip_metadata); // OR'd together
    }

    #[test]
    fn merge_video_options_preset_fills_missing() {
        let cli = Some(VideoOptions {
            crf: Some(20),
            ..Default::default()
        });
        let preset = Some(VideoOptions {
            width: Some(1080),
            height: Some(1920),
            fps: Some(30.0),
            crf: Some(28),
            ..Default::default()
        });
        let merged = super::merge_video_options(cli, preset).unwrap();
        assert_eq!(merged.crf, Some(20)); // cli wins
        assert_eq!(merged.width, Some(1080)); // filled from preset
        assert_eq!(merged.fps, Some(30.0)); // filled from preset
    }
}
