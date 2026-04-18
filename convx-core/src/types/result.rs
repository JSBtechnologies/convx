use super::format::Format;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionResult {
    pub id: Uuid,
    pub status: ConversionStatus,
    pub input_path: PathBuf,
    pub output_path: Option<PathBuf>,
    pub input_format: Format,
    pub output_format: Format,
    pub input_size: u64,
    pub output_size: Option<u64>,
    pub space_saved: Option<i64>,
    pub duration_ms: u64,
    pub error: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConversionStatus {
    Completed,
    Failed,
}
