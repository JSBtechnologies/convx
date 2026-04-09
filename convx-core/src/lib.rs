#[cfg(feature = "cli")]
pub mod cli;
pub mod converters;
pub mod engine;
pub mod mcp_server;
pub mod presets;
pub mod types;
pub mod utils;
pub mod watch;

pub use engine::ConvxEngine;
pub use types::error::ConvxError;
pub use types::format::{Format, FormatCategory};
pub use types::options::{
    AudioOptions, ConversionOptions, DocumentOptions, ImageOptions, VideoOptions,
};
pub use types::preset::Preset;
pub use types::result::{ConversionResult, ConversionStatus};
pub use utils::silent_command;
pub use utils::DependencyChecker;
pub use utils::FfprobeInfo;
