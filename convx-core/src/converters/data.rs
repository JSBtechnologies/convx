use crate::converters::{extract_tool_error, Converter};
use crate::types::{
    error::ConvxError,
    format::Format,
    options::ConversionOptions,
    result::{ConversionResult, ConversionStatus},
};
use crate::utils::deps::silent_command;
use crate::utils::DependencyChecker;
use chrono::Utc;
use csv::{ReaderBuilder, WriterBuilder};
use quick_xml::de::from_str as from_xml_str;
use serde_json::Value;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use uuid::Uuid;

pub struct DataConverter;

/// An XML DOM node used for clean HTML rendering.
struct XmlNode {
    tag: String,
    attrs: Vec<(String, String)>,
    text: String,
    children: Vec<XmlNode>,
}

const STRUCTURED_HTML_HEAD: &str = "<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n\
<style>\n\
body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; padding: 24px; background: #fff; color: #333; }\n\
table { border-collapse: collapse; font-size: 14px; margin-bottom: 16px; }\n\
th { background-color: #f5f5f5; font-weight: 600; text-align: left; padding: 6px 12px; border: 1px solid #ddd; }\n\
td { padding: 6px 12px; border: 1px solid #ddd; }\n\
.footer { margin-top: 16px; font-size: 12px; color: #999; }\n\
</style>\n</head>\n<body>\n";

const XML_HTML_HEAD: &str = "<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n\
<style>\n\
body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; margin: 0; padding: 32px; background: #fff; color: #1a1a1a; line-height: 1.5; }\n\
h1 { font-size: 22px; color: #111; border-bottom: 2px solid #333; padding-bottom: 6px; margin: 0 0 16px 0; }\n\
h2 { font-size: 17px; color: #333; border-bottom: 1px solid #ccc; padding-bottom: 4px; margin: 20px 0 10px 0; }\n\
h3 { font-size: 14px; color: #555; margin: 14px 0 6px 0; }\n\
.attr-bar { margin-bottom: 10px; }\n\
.attr { display: inline-block; background: #f0f0f0; border: 1px solid #ddd; border-radius: 4px; padding: 2px 8px; margin: 2px 4px 2px 0; font-size: 12px; color: #555; }\n\
.attr-key { font-weight: 600; color: #333; }\n\
.attr-inline { background: #f5f5f5; border-radius: 3px; padding: 1px 5px; font-size: 11px; color: #777; margin-left: 6px; }\n\
table.props { border-collapse: collapse; margin-bottom: 12px; }\n\
table.props th { background: #fafafa; font-weight: 600; text-align: left; padding: 5px 14px; border: 1px solid #e0e0e0; color: #444; font-size: 13px; white-space: nowrap; }\n\
table.props td { padding: 5px 14px; border: 1px solid #e0e0e0; font-size: 13px; }\n\
table.data-table { border-collapse: collapse; width: 100%; margin-bottom: 16px; }\n\
table.data-table th { background: #f5f5f5; font-weight: 600; text-align: left; padding: 8px 12px; border: 1px solid #ddd; font-size: 13px; color: #333; }\n\
table.data-table td { padding: 8px 12px; border: 1px solid #ddd; font-size: 13px; }\n\
table.data-table tr:nth-child(even) { background: #fafafa; }\n\
.sub-section { padding: 8px 16px !important; background: #fafafa; }\n\
.section { margin-left: 12px; margin-bottom: 8px; }\n\
ul { margin: 4px 0; padding-left: 20px; }\n\
li { font-size: 13px; margin-bottom: 2px; }\n\
p { margin: 4px 0; font-size: 13px; }\n\
.footer { margin-top: 24px; font-size: 11px; color: #aaa; border-top: 1px solid #eee; padding-top: 8px; }\n\
</style>\n</head>\n<body>\n";

impl DataConverter {
    fn can_convert_pair(from: Format, to: Format) -> bool {
        matches!(
            (from, to),
            // Existing
            (Format::Csv, Format::Xlsx)
                | (Format::Xlsx, Format::Csv)
                | (Format::Json, Format::Csv)
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
                // Data -> Document
                | (Format::Json, Format::Html)
                | (Format::Csv, Format::Html)
                | (Format::Xml, Format::Html)
                | (Format::Yaml, Format::Html)
                | (Format::Tsv, Format::Html)
                | (Format::Jsonl, Format::Html)
                | (Format::Json, Format::Pdf)
                | (Format::Csv, Format::Pdf)
                | (Format::Xml, Format::Pdf)
                | (Format::Yaml, Format::Pdf)
                | (Format::Tsv, Format::Pdf)
                | (Format::Jsonl, Format::Pdf)
                | (Format::Json, Format::Md)
                | (Format::Csv, Format::Md)
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
                // SQLite
                | (Format::Sqlite, Format::Csv)
                | (Format::Sqlite, Format::Json)
                // NPY/NPZ
                | (Format::Npy, Format::Csv)
                | (Format::Npz, Format::Csv)
                // HDF5
                | (Format::Hdf5, Format::Csv)
                | (Format::Hdf5, Format::Json)
        )
    }

    // ─── Existing helpers ────────────────────────────────────────────

    fn run_python_pandas(
        input: &Path,
        output: &Path,
        from: Format,
        to: Format,
    ) -> Result<(), ConvxError> {
        let py = DependencyChecker::convx_python().ok_or(ConvxError::ConversionFailed {
            reason: "python3 not found. Required for CSV/XLSX conversion.".to_string(),
        })?;

        if !DependencyChecker::python_has_module("pandas")
            || !DependencyChecker::python_has_module("openpyxl")
        {
            return Err(ConvxError::ConversionFailed {
                reason: "Missing Python modules pandas/openpyxl for CSV/XLSX conversion"
                    .to_string(),
            });
        }

        let script = match (from, to) {
            (Format::Csv, Format::Xlsx) => {
                "import pandas as pd; import sys; df=pd.read_csv(sys.argv[1]); df.to_excel(sys.argv[2], index=False)"
            }
            (Format::Xlsx, Format::Csv) => {
                "import pandas as pd; import sys; df=pd.read_excel(sys.argv[1]); df.to_csv(sys.argv[2], index=False)"
            }
            _ => return Err(ConvxError::UnsupportedConversion { from, to }),
        };

        let out = silent_command(py)
            .args(["-c", script])
            .arg(input)
            .arg(output)
            .output()
            .map_err(|e| ConvxError::ConversionFailed {
                reason: format!("Failed to execute python3: {}", e),
            })?;

        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            return Err(ConvxError::ConversionFailed {
                reason: extract_tool_error(&stderr),
            });
        }

        Ok(())
    }

    fn json_to_csv(input: &Path, output: &Path) -> Result<(), ConvxError> {
        let text = fs::read_to_string(input).map_err(|e| ConvxError::FileReadError {
            path: input.to_path_buf(),
            reason: e.to_string(),
        })?;

        let value: Value =
            serde_json::from_str(&text).map_err(|e| ConvxError::ConversionFailed {
                reason: format!("Invalid JSON: {}", e),
            })?;

        let arr = value.as_array().ok_or(ConvxError::ConversionFailed {
            reason: "JSON->CSV expects an array of objects".to_string(),
        })?;

        Self::objects_to_csv(arr, output)
    }

    fn objects_to_csv(arr: &[Value], output: &Path) -> Result<(), ConvxError> {
        let mut headers: Vec<String> = Vec::new();
        for row in arr {
            if let Some(obj) = row.as_object() {
                for k in obj.keys() {
                    if !headers.contains(k) {
                        headers.push(k.clone());
                    }
                }
            }
        }

        let mut wtr =
            WriterBuilder::new()
                .from_path(output)
                .map_err(|e| ConvxError::FileWriteError {
                    path: output.to_path_buf(),
                    reason: e.to_string(),
                })?;

        wtr.write_record(&headers)
            .map_err(|e| ConvxError::ConversionFailed {
                reason: e.to_string(),
            })?;

        for row in arr {
            let obj = row.as_object().ok_or(ConvxError::ConversionFailed {
                reason: "Expected only object entries".to_string(),
            })?;
            let record = headers
                .iter()
                .map(|h| obj.get(h).map(Self::json_value_to_csv).unwrap_or_default())
                .collect::<Vec<_>>();
            wtr.write_record(record)
                .map_err(|e| ConvxError::ConversionFailed {
                    reason: e.to_string(),
                })?;
        }

        wtr.flush().map_err(|e| ConvxError::ConversionFailed {
            reason: e.to_string(),
        })?;
        Ok(())
    }

    fn csv_to_json(input: &Path, output: &Path) -> Result<(), ConvxError> {
        let mut rdr =
            ReaderBuilder::new()
                .from_path(input)
                .map_err(|e| ConvxError::FileReadError {
                    path: input.to_path_buf(),
                    reason: e.to_string(),
                })?;

        let headers = rdr
            .headers()
            .map_err(|e| ConvxError::ConversionFailed {
                reason: e.to_string(),
            })?
            .clone();

        let mut rows = Vec::new();
        for rec in rdr.records() {
            let rec = rec.map_err(|e| ConvxError::ConversionFailed {
                reason: e.to_string(),
            })?;
            let mut map = serde_json::Map::new();
            for (h, v) in headers.iter().zip(rec.iter()) {
                map.insert(h.to_string(), Value::String(v.to_string()));
            }
            rows.push(Value::Object(map));
        }

        fs::write(
            output,
            serde_json::to_string_pretty(&rows).map_err(|e| ConvxError::ConversionFailed {
                reason: e.to_string(),
            })?,
        )
        .map_err(|e| ConvxError::FileWriteError {
            path: output.to_path_buf(),
            reason: e.to_string(),
        })
    }

    fn json_to_yaml(input: &Path, output: &Path) -> Result<(), ConvxError> {
        let text = fs::read_to_string(input).map_err(|e| ConvxError::FileReadError {
            path: input.to_path_buf(),
            reason: e.to_string(),
        })?;
        let v: Value = serde_json::from_str(&text).map_err(|e| ConvxError::ConversionFailed {
            reason: format!("Invalid JSON: {}", e),
        })?;
        let y = serde_yaml::to_string(&v).map_err(|e| ConvxError::ConversionFailed {
            reason: e.to_string(),
        })?;
        fs::write(output, y).map_err(|e| ConvxError::FileWriteError {
            path: output.to_path_buf(),
            reason: e.to_string(),
        })
    }

    fn yaml_to_json(input: &Path, output: &Path) -> Result<(), ConvxError> {
        let text = fs::read_to_string(input).map_err(|e| ConvxError::FileReadError {
            path: input.to_path_buf(),
            reason: e.to_string(),
        })?;
        let v: Value = serde_yaml::from_str(&text).map_err(|e| ConvxError::ConversionFailed {
            reason: format!("Invalid YAML: {}", e),
        })?;
        fs::write(
            output,
            serde_json::to_string_pretty(&v).map_err(|e| ConvxError::ConversionFailed {
                reason: e.to_string(),
            })?,
        )
        .map_err(|e| ConvxError::FileWriteError {
            path: output.to_path_buf(),
            reason: e.to_string(),
        })
    }

    fn xml_to_json(input: &Path, output: &Path) -> Result<(), ConvxError> {
        let text = fs::read_to_string(input).map_err(|e| ConvxError::FileReadError {
            path: input.to_path_buf(),
            reason: e.to_string(),
        })?;
        let v: Value = from_xml_str(&text).map_err(|e| ConvxError::ConversionFailed {
            reason: format!("Invalid XML: {}", e),
        })?;
        fs::write(
            output,
            serde_json::to_string_pretty(&v).map_err(|e| ConvxError::ConversionFailed {
                reason: e.to_string(),
            })?,
        )
        .map_err(|e| ConvxError::FileWriteError {
            path: output.to_path_buf(),
            reason: e.to_string(),
        })
    }

    fn json_to_xml(input: &Path, output: &Path) -> Result<(), ConvxError> {
        let text = fs::read_to_string(input).map_err(|e| ConvxError::FileReadError {
            path: input.to_path_buf(),
            reason: e.to_string(),
        })?;
        let v: Value = serde_json::from_str(&text).map_err(|e| ConvxError::ConversionFailed {
            reason: format!("Invalid JSON: {}", e),
        })?;

        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        Self::json_value_to_xml_string(&v, "root", &mut xml, 0);
        xml.push('\n');

        fs::write(output, xml).map_err(|e| ConvxError::FileWriteError {
            path: output.to_path_buf(),
            reason: e.to_string(),
        })
    }

    fn json_value_to_xml_string(value: &Value, tag: &str, out: &mut String, depth: usize) {
        let indent = "  ".repeat(depth);
        match value {
            Value::Null => {
                out.push_str(&format!("{}<{}/>\n", indent, tag));
            }
            Value::Bool(b) => {
                out.push_str(&format!("{}<{}>{}</{}>\n", indent, tag, b, tag));
            }
            Value::Number(n) => {
                out.push_str(&format!("{}<{}>{}</{}>\n", indent, tag, n, tag));
            }
            Value::String(s) => {
                out.push_str(&format!(
                    "{}<{}>{}</{}>\n",
                    indent,
                    tag,
                    Self::xml_escape(s),
                    tag
                ));
            }
            Value::Array(arr) => {
                out.push_str(&format!("{}<{}>\n", indent, tag));
                for item in arr {
                    Self::json_value_to_xml_string(item, "item", out, depth + 1);
                }
                out.push_str(&format!("{}</{}>\n", indent, tag));
            }
            Value::Object(map) => {
                out.push_str(&format!("{}<{}>\n", indent, tag));
                for (key, val) in map {
                    Self::json_value_to_xml_string(val, key, out, depth + 1);
                }
                out.push_str(&format!("{}</{}>\n", indent, tag));
            }
        }
    }

    fn xml_escape(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }

    fn json_value_to_csv(v: &Value) -> String {
        match v {
            Value::Null => String::new(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => n.to_string(),
            Value::String(s) => s.clone(),
            _ => serde_json::to_string(v).unwrap_or_default(),
        }
    }

    // ─── TSV ↔ CSV ──────────────────────────────────────────────────

    fn tsv_to_csv(input: &Path, output: &Path) -> Result<(), ConvxError> {
        let mut rdr = ReaderBuilder::new()
            .delimiter(b'\t')
            .from_path(input)
            .map_err(|e| ConvxError::FileReadError {
                path: input.to_path_buf(),
                reason: e.to_string(),
            })?;

        let mut wtr =
            WriterBuilder::new()
                .from_path(output)
                .map_err(|e| ConvxError::FileWriteError {
                    path: output.to_path_buf(),
                    reason: e.to_string(),
                })?;

        let headers = rdr
            .headers()
            .map_err(|e| ConvxError::ConversionFailed {
                reason: e.to_string(),
            })?
            .clone();
        wtr.write_record(&headers)
            .map_err(|e| ConvxError::ConversionFailed {
                reason: e.to_string(),
            })?;

        for rec in rdr.records() {
            let rec = rec.map_err(|e| ConvxError::ConversionFailed {
                reason: e.to_string(),
            })?;
            wtr.write_record(&rec)
                .map_err(|e| ConvxError::ConversionFailed {
                    reason: e.to_string(),
                })?;
        }

        wtr.flush().map_err(|e| ConvxError::ConversionFailed {
            reason: e.to_string(),
        })?;
        Ok(())
    }

    fn csv_to_tsv(input: &Path, output: &Path) -> Result<(), ConvxError> {
        let mut rdr =
            ReaderBuilder::new()
                .from_path(input)
                .map_err(|e| ConvxError::FileReadError {
                    path: input.to_path_buf(),
                    reason: e.to_string(),
                })?;

        let mut wtr = WriterBuilder::new()
            .delimiter(b'\t')
            .from_path(output)
            .map_err(|e| ConvxError::FileWriteError {
                path: output.to_path_buf(),
                reason: e.to_string(),
            })?;

        let headers = rdr
            .headers()
            .map_err(|e| ConvxError::ConversionFailed {
                reason: e.to_string(),
            })?
            .clone();
        wtr.write_record(&headers)
            .map_err(|e| ConvxError::ConversionFailed {
                reason: e.to_string(),
            })?;

        for rec in rdr.records() {
            let rec = rec.map_err(|e| ConvxError::ConversionFailed {
                reason: e.to_string(),
            })?;
            wtr.write_record(&rec)
                .map_err(|e| ConvxError::ConversionFailed {
                    reason: e.to_string(),
                })?;
        }

        wtr.flush().map_err(|e| ConvxError::ConversionFailed {
            reason: e.to_string(),
        })?;
        Ok(())
    }

    // ─── JSONL ↔ JSON / CSV ─────────────────────────────────────────

    fn parse_jsonl(input: &Path) -> Result<Vec<Value>, ConvxError> {
        let file = fs::File::open(input).map_err(|e| ConvxError::FileReadError {
            path: input.to_path_buf(),
            reason: e.to_string(),
        })?;
        let reader = BufReader::new(file);
        let mut values = Vec::new();
        for line in reader.lines() {
            let line = line.map_err(|e| ConvxError::FileReadError {
                path: input.to_path_buf(),
                reason: e.to_string(),
            })?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let v: Value =
                serde_json::from_str(trimmed).map_err(|e| ConvxError::ConversionFailed {
                    reason: format!("Invalid JSONL line: {}", e),
                })?;
            values.push(v);
        }
        Ok(values)
    }

    fn jsonl_to_json(input: &Path, output: &Path) -> Result<(), ConvxError> {
        let values = Self::parse_jsonl(input)?;
        let arr = Value::Array(values);
        fs::write(
            output,
            serde_json::to_string_pretty(&arr).map_err(|e| ConvxError::ConversionFailed {
                reason: e.to_string(),
            })?,
        )
        .map_err(|e| ConvxError::FileWriteError {
            path: output.to_path_buf(),
            reason: e.to_string(),
        })
    }

    fn json_to_jsonl(input: &Path, output: &Path) -> Result<(), ConvxError> {
        let text = fs::read_to_string(input).map_err(|e| ConvxError::FileReadError {
            path: input.to_path_buf(),
            reason: e.to_string(),
        })?;
        let value: Value =
            serde_json::from_str(&text).map_err(|e| ConvxError::ConversionFailed {
                reason: format!("Invalid JSON: {}", e),
            })?;
        let arr = value.as_array().ok_or(ConvxError::ConversionFailed {
            reason: "JSON->JSONL expects an array".to_string(),
        })?;

        let mut file = fs::File::create(output).map_err(|e| ConvxError::FileWriteError {
            path: output.to_path_buf(),
            reason: e.to_string(),
        })?;
        for item in arr {
            let line = serde_json::to_string(item).map_err(|e| ConvxError::ConversionFailed {
                reason: e.to_string(),
            })?;
            writeln!(file, "{}", line).map_err(|e| ConvxError::FileWriteError {
                path: output.to_path_buf(),
                reason: e.to_string(),
            })?;
        }
        Ok(())
    }

    fn jsonl_to_csv(input: &Path, output: &Path) -> Result<(), ConvxError> {
        let values = Self::parse_jsonl(input)?;
        Self::objects_to_csv(&values, output)
    }

    fn csv_to_jsonl(input: &Path, output: &Path) -> Result<(), ConvxError> {
        let mut rdr =
            ReaderBuilder::new()
                .from_path(input)
                .map_err(|e| ConvxError::FileReadError {
                    path: input.to_path_buf(),
                    reason: e.to_string(),
                })?;

        let headers = rdr
            .headers()
            .map_err(|e| ConvxError::ConversionFailed {
                reason: e.to_string(),
            })?
            .clone();

        let mut file = fs::File::create(output).map_err(|e| ConvxError::FileWriteError {
            path: output.to_path_buf(),
            reason: e.to_string(),
        })?;

        for rec in rdr.records() {
            let rec = rec.map_err(|e| ConvxError::ConversionFailed {
                reason: e.to_string(),
            })?;
            let mut map = serde_json::Map::new();
            for (h, v) in headers.iter().zip(rec.iter()) {
                map.insert(h.to_string(), Value::String(v.to_string()));
            }
            let line = serde_json::to_string(&Value::Object(map)).map_err(|e| {
                ConvxError::ConversionFailed {
                    reason: e.to_string(),
                }
            })?;
            writeln!(file, "{}", line).map_err(|e| ConvxError::FileWriteError {
                path: output.to_path_buf(),
                reason: e.to_string(),
            })?;
        }
        Ok(())
    }

    // ─── Data → Tabular (shared helper) ─────────────────────────────

    /// Normalize any text-based data format into (headers, rows).
    fn data_to_tabular(
        input: &Path,
        from: Format,
    ) -> Result<(Vec<String>, Vec<Vec<String>>), ConvxError> {
        match from {
            Format::Csv => Self::csv_to_tabular(input, b','),
            Format::Tsv => Self::csv_to_tabular(input, b'\t'),
            Format::Json => {
                let text = fs::read_to_string(input).map_err(|e| ConvxError::FileReadError {
                    path: input.to_path_buf(),
                    reason: e.to_string(),
                })?;
                let v: Value =
                    serde_json::from_str(&text).map_err(|e| ConvxError::ConversionFailed {
                        reason: format!("Invalid JSON: {}", e),
                    })?;
                Self::json_value_to_tabular(&v)
            }
            Format::Jsonl => {
                let values = Self::parse_jsonl(input)?;
                Self::json_value_to_tabular(&Value::Array(values))
            }
            Format::Yaml => {
                let text = fs::read_to_string(input).map_err(|e| ConvxError::FileReadError {
                    path: input.to_path_buf(),
                    reason: e.to_string(),
                })?;
                let v: Value =
                    serde_yaml::from_str(&text).map_err(|e| ConvxError::ConversionFailed {
                        reason: format!("Invalid YAML: {}", e),
                    })?;
                Self::json_value_to_tabular(&v)
            }
            Format::Xml => {
                let text = fs::read_to_string(input).map_err(|e| ConvxError::FileReadError {
                    path: input.to_path_buf(),
                    reason: e.to_string(),
                })?;
                let v: Value = from_xml_str(&text).map_err(|e| ConvxError::ConversionFailed {
                    reason: format!("Invalid XML: {}", e),
                })?;
                Self::json_value_to_tabular(&v)
            }
            _ => Err(ConvxError::ConversionFailed {
                reason: format!("Cannot extract tabular data from {:?}", from),
            }),
        }
    }

    fn csv_to_tabular(
        input: &Path,
        delimiter: u8,
    ) -> Result<(Vec<String>, Vec<Vec<String>>), ConvxError> {
        let mut rdr = ReaderBuilder::new()
            .delimiter(delimiter)
            .from_path(input)
            .map_err(|e| ConvxError::FileReadError {
                path: input.to_path_buf(),
                reason: e.to_string(),
            })?;

        let headers: Vec<String> = rdr
            .headers()
            .map_err(|e| ConvxError::ConversionFailed {
                reason: e.to_string(),
            })?
            .iter()
            .map(|s| s.to_string())
            .collect();

        let mut rows = Vec::new();
        for rec in rdr.records() {
            let rec = rec.map_err(|e| ConvxError::ConversionFailed {
                reason: e.to_string(),
            })?;
            rows.push(rec.iter().map(|s| s.to_string()).collect());
        }

        Ok((headers, rows))
    }

    fn json_value_to_tabular(v: &Value) -> Result<(Vec<String>, Vec<Vec<String>>), ConvxError> {
        let arr = Self::extract_array(v).ok_or(ConvxError::ConversionFailed {
            reason: "Could not extract tabular data: expected an array of objects (or an object containing one)".to_string(),
        })?;

        let mut headers: Vec<String> = Vec::new();
        for row in arr {
            if let Some(obj) = row.as_object() {
                for k in obj.keys() {
                    if !headers.contains(k) {
                        headers.push(k.clone());
                    }
                }
            }
        }

        let rows = arr
            .iter()
            .filter_map(|row| {
                row.as_object().map(|obj| {
                    headers
                        .iter()
                        .map(|h| obj.get(h).map(Self::json_value_to_csv).unwrap_or_default())
                        .collect()
                })
            })
            .collect();

        Ok((headers, rows))
    }

    /// Try to find an array of objects in a JSON value.
    /// - If it's already an array, return it directly.
    /// - If it's an object, search its values for the first array of objects
    ///   (handles XML like `<root><items><item>...</item></items></root>`).
    /// - If it's an object with no nested array, wrap it as a single-element array.
    fn extract_array(v: &Value) -> Option<&Vec<Value>> {
        if let Some(arr) = v.as_array() {
            return Some(arr);
        }
        if let Some(obj) = v.as_object() {
            // Look for a child value that is an array of objects
            for val in obj.values() {
                if let Some(arr) = val.as_array() {
                    if arr.iter().any(|item| item.is_object()) {
                        return Some(arr);
                    }
                }
                // Recurse one level into nested objects
                if let Some(inner_obj) = val.as_object() {
                    for inner_val in inner_obj.values() {
                        if let Some(arr) = inner_val.as_array() {
                            if arr.iter().any(|item| item.is_object()) {
                                return Some(arr);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    // ─── Data → HTML ────────────────────────────────────────────────

    fn data_to_html(input: &Path, output: &Path, from: Format) -> Result<(), ConvxError> {
        match Self::data_to_tabular(input, from) {
            Ok((headers, rows)) => Self::tabular_to_html(output, &headers, &rows),
            Err(_) if from == Format::Xml || from == Format::Json || from == Format::Yaml => {
                // Non-tabular structured data: render as key-value tree
                Self::structured_to_html(input, output, from)
            }
            Err(e) => Err(e),
        }
    }

    fn tabular_to_html(
        output: &Path,
        headers: &[String],
        rows: &[Vec<String>],
    ) -> Result<(), ConvxError> {
        let mut html = String::from(
            "<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n\
             <style>\n\
             body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; padding: 24px; background: #fff; }\n\
             h2 { color: #333; margin-bottom: 16px; }\n\
             table { border-collapse: collapse; width: 100%; font-size: 14px; }\n\
             th, td { border: 1px solid #ddd; padding: 8px 12px; text-align: left; }\n\
             th { background-color: #f5f5f5; font-weight: 600; color: #333; }\n\
             tr:nth-child(even) { background-color: #fafafa; }\n\
             tr:hover { background-color: #f0f0f0; }\n\
             .footer { margin-top: 16px; font-size: 12px; color: #999; }\n\
             </style>\n</head>\n<body>\n",
        );

        html.push_str("<table>\n<thead>\n<tr>");
        for h in headers {
            html.push_str("<th>");
            html.push_str(&html_escape(h));
            html.push_str("</th>");
        }
        html.push_str("</tr>\n</thead>\n<tbody>\n");

        for row in rows {
            html.push_str("<tr>");
            for cell in row {
                html.push_str("<td>");
                html.push_str(&html_escape(cell));
                html.push_str("</td>");
            }
            html.push_str("</tr>\n");
        }

        html.push_str("</tbody>\n</table>\n");
        html.push_str(&format!(
            "<p class=\"footer\">{} rows &times; {} columns &mdash; Generated by ConvX</p>\n",
            rows.len(),
            headers.len()
        ));
        html.push_str("</body>\n</html>\n");

        fs::write(output, html).map_err(|e| ConvxError::FileWriteError {
            path: output.to_path_buf(),
            reason: e.to_string(),
        })
    }

    /// Render non-tabular structured data (XML, JSON, YAML) as a formatted HTML document.
    fn structured_to_html(input: &Path, output: &Path, from: Format) -> Result<(), ConvxError> {
        if from == Format::Xml {
            return Self::xml_to_styled_html(input, output);
        }

        let text = fs::read_to_string(input).map_err(|e| ConvxError::FileReadError {
            path: input.to_path_buf(),
            reason: e.to_string(),
        })?;

        let v: Value = match from {
            Format::Yaml => {
                serde_yaml::from_str(&text).map_err(|e| ConvxError::ConversionFailed {
                    reason: format!("Invalid YAML: {}", e),
                })?
            }
            _ => serde_json::from_str(&text).map_err(|e| ConvxError::ConversionFailed {
                reason: format!("Invalid JSON: {}", e),
            })?,
        };

        let mut html = String::from(STRUCTURED_HTML_HEAD);
        Self::render_value_html(&v, &mut html, 0);
        html.push_str("<p class=\"footer\">Generated by ConvX</p>\n</body>\n</html>\n");

        fs::write(output, html).map_err(|e| ConvxError::FileWriteError {
            path: output.to_path_buf(),
            reason: e.to_string(),
        })
    }

    #[allow(clippy::only_used_in_recursion)]
    fn render_value_html(v: &Value, html: &mut String, depth: usize) {
        match v {
            Value::Object(obj) => {
                html.push_str("<table>\n");
                for (k, val) in obj {
                    html.push_str("<tr><th>");
                    html.push_str(&html_escape(k));
                    html.push_str("</th><td>");
                    if val.is_object() || val.is_array() {
                        Self::render_value_html(val, html, depth + 1);
                    } else {
                        html.push_str(&html_escape(&Self::json_value_to_csv(val)));
                    }
                    html.push_str("</td></tr>\n");
                }
                html.push_str("</table>\n");
            }
            Value::Array(arr) => {
                for (i, item) in arr.iter().enumerate() {
                    html.push_str(&format!(
                        "<details open><summary>Item {}</summary>\n",
                        i + 1
                    ));
                    Self::render_value_html(item, html, depth + 1);
                    html.push_str("</details>\n");
                }
            }
            other => {
                html.push_str(&html_escape(&Self::json_value_to_csv(other)));
            }
        }
    }

    // ─── XML-specific clean HTML renderer ────────────────────────────

    /// Parse XML into a tree of XmlNode, then render clean HTML.
    fn xml_to_styled_html(input: &Path, output: &Path) -> Result<(), ConvxError> {
        use quick_xml::events::Event;
        use quick_xml::Reader;

        let text = fs::read_to_string(input).map_err(|e| ConvxError::FileReadError {
            path: input.to_path_buf(),
            reason: e.to_string(),
        })?;

        fn parse_element(
            reader: &mut Reader<&[u8]>,
            tag: String,
            attrs: Vec<(String, String)>,
        ) -> XmlNode {
            let mut node = XmlNode {
                tag,
                attrs,
                text: String::new(),
                children: Vec::new(),
            };

            loop {
                match reader.read_event() {
                    Ok(Event::Start(ref e)) => {
                        let child_tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                        let child_attrs: Vec<(String, String)> = e
                            .attributes()
                            .filter_map(|a| a.ok())
                            .map(|a| {
                                (
                                    String::from_utf8_lossy(a.key.as_ref()).to_string(),
                                    a.unescape_value().unwrap_or_default().to_string(),
                                )
                            })
                            .collect();
                        node.children
                            .push(parse_element(reader, child_tag, child_attrs));
                    }
                    Ok(Event::Empty(ref e)) => {
                        let child_tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                        let child_attrs: Vec<(String, String)> = e
                            .attributes()
                            .filter_map(|a| a.ok())
                            .map(|a| {
                                (
                                    String::from_utf8_lossy(a.key.as_ref()).to_string(),
                                    a.unescape_value().unwrap_or_default().to_string(),
                                )
                            })
                            .collect();
                        node.children.push(XmlNode {
                            tag: child_tag,
                            attrs: child_attrs,
                            text: String::new(),
                            children: Vec::new(),
                        });
                    }
                    Ok(Event::Text(ref e)) => {
                        let txt = e.unescape().unwrap_or_default().trim().to_string();
                        if !txt.is_empty() {
                            node.text.push_str(&txt);
                        }
                    }
                    Ok(Event::End(_)) => break,
                    Ok(Event::Eof) => break,
                    _ => {}
                }
            }

            node
        }

        let mut reader = Reader::from_str(&text);
        let mut root: Option<XmlNode> = None;

        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) => {
                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let attrs: Vec<(String, String)> = e
                        .attributes()
                        .filter_map(|a| a.ok())
                        .map(|a| {
                            (
                                String::from_utf8_lossy(a.key.as_ref()).to_string(),
                                a.unescape_value().unwrap_or_default().to_string(),
                            )
                        })
                        .collect();
                    root = Some(parse_element(&mut reader, tag, attrs));
                    break;
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    return Err(ConvxError::ConversionFailed {
                        reason: format!("Invalid XML: {}", e),
                    })
                }
                _ => {}
            }
        }

        let root = root.ok_or(ConvxError::ConversionFailed {
            reason: "XML document has no root element".to_string(),
        })?;

        let mut html = String::from(XML_HTML_HEAD);
        Self::render_xml_node(&root, &mut html, 0, true);
        html.push_str("<p class=\"footer\">Generated by ConvX</p>\n</body>\n</html>\n");

        fs::write(output, html).map_err(|e| ConvxError::FileWriteError {
            path: output.to_path_buf(),
            reason: e.to_string(),
        })
    }

    fn render_xml_node(node: &XmlNode, html: &mut String, depth: usize, is_root: bool) {
        let heading = if depth == 0 {
            "h1"
        } else if depth == 1 {
            "h2"
        } else {
            "h3"
        };

        // Determine if children are a homogeneous list (all same tag name)
        let child_tags: Vec<&str> = node.children.iter().map(|c| c.tag.as_str()).collect();
        let is_homogeneous_list =
            child_tags.len() > 1 && child_tags.iter().all(|t| *t == child_tags[0]);

        // If this node has only text, it's a leaf — don't render a section
        if node.children.is_empty() && node.attrs.is_empty() {
            // Pure text leaf — rendered inline by parent
            return;
        }

        // Section header with tag name
        if !is_root || !node.attrs.is_empty() || !node.children.is_empty() {
            let display_name = Self::xml_tag_display(&node.tag);
            html.push_str(&format!("<{heading}>{display_name}</{heading}>\n"));
        }

        // Render attributes as a metadata bar
        if !node.attrs.is_empty() {
            html.push_str("<div class=\"attr-bar\">");
            for (k, v) in &node.attrs {
                html.push_str(&format!(
                    "<span class=\"attr\"><span class=\"attr-key\">{}</span> {}</span>",
                    html_escape(k),
                    html_escape(v)
                ));
            }
            html.push_str("</div>\n");
        }

        // Render text content
        if !node.text.is_empty() {
            html.push_str(&format!("<p>{}</p>\n", html_escape(&node.text)));
        }

        // Render children
        if is_homogeneous_list {
            // Render as a table
            Self::render_xml_children_as_table(&node.children, html);
        } else {
            // Render leaf children (text-only or simple value) as a definition list
            let leaf_children: Vec<&XmlNode> = node
                .children
                .iter()
                .filter(|c| c.children.is_empty())
                .collect();
            let complex_children: Vec<&XmlNode> = node
                .children
                .iter()
                .filter(|c| !c.children.is_empty())
                .collect();

            if !leaf_children.is_empty() {
                html.push_str("<table class=\"props\">\n");
                for child in &leaf_children {
                    let display = Self::xml_tag_display(&child.tag);
                    html.push_str(&format!(
                        "<tr><th>{}</th><td>{}",
                        html_escape(&display),
                        html_escape(&child.text)
                    ));
                    // Show child attributes inline
                    if !child.attrs.is_empty() {
                        for (k, v) in &child.attrs {
                            html.push_str(&format!(
                                " <span class=\"attr-inline\">{}: {}</span>",
                                html_escape(k),
                                html_escape(v)
                            ));
                        }
                    }
                    html.push_str("</td></tr>\n");
                }
                html.push_str("</table>\n");
            }

            for child in &complex_children {
                html.push_str("<div class=\"section\">\n");
                Self::render_xml_node(child, html, depth + 1, false);
                html.push_str("</div>\n");
            }
        }
    }

    /// Render a list of similar XML elements as a clean table.
    fn render_xml_children_as_table(children: &[XmlNode], html: &mut String) {
        if children.is_empty() {
            return;
        }

        // If the children are all simple text (like <tag>value</tag>), render as a list
        let all_text_only = children
            .iter()
            .all(|c| c.children.is_empty() && c.attrs.is_empty());
        if all_text_only {
            html.push_str("<ul>\n");
            for child in children {
                html.push_str(&format!("<li>{}</li>\n", html_escape(&child.text)));
            }
            html.push_str("</ul>\n");
            return;
        }

        // Collect all unique field names across all children.
        // Sources: item attributes, leaf sub-elements, and attributes of leaf sub-elements
        // that are self-closing (e.g. <expectedRestock date="..."/>).
        let mut columns: Vec<String> = Vec::new();
        // Also track if any child has text content directly (e.g. <note priority="low">text</note>)
        let any_has_text = children.iter().any(|c| !c.text.is_empty());

        for child in children {
            // Item-level attributes (e.g. sku, status on <item>)
            for (k, _) in &child.attrs {
                if !columns.contains(k) {
                    columns.push(k.clone());
                }
            }
            // Leaf sub-elements: elements with text or with only attributes (self-closing)
            for sub in &child.children {
                if sub.children.is_empty() && !columns.contains(&sub.tag) {
                    columns.push(sub.tag.clone());
                }
            }
        }

        // Add a "Content" column for items with direct text (e.g. <note priority="low">text</note>)
        let text_col = "_text_content_".to_string();
        if any_has_text {
            columns.push(text_col.clone());
        }

        if columns.is_empty() {
            // Fallback: render each child as a section
            for child in children {
                html.push_str("<div class=\"section\">\n");
                Self::render_xml_node(child, html, 2, false);
                html.push_str("</div>\n");
            }
            return;
        }

        html.push_str("<table class=\"data-table\">\n<thead><tr>");
        for col in &columns {
            if *col == text_col {
                // Use the parent's child tag name as column header (e.g. "Note")
                let label = Self::xml_tag_display(&children[0].tag);
                html.push_str(&format!("<th>{}</th>", html_escape(&label)));
            } else {
                html.push_str(&format!(
                    "<th>{}</th>",
                    html_escape(&Self::xml_tag_display(col))
                ));
            }
        }
        html.push_str("</tr></thead>\n<tbody>\n");

        for child in children {
            html.push_str("<tr>");
            for col in &columns {
                if *col == text_col {
                    // Direct text content of the element
                    html.push_str(&format!("<td>{}</td>", html_escape(&child.text)));
                } else if let Some((_, v)) = child.attrs.iter().find(|(k, _)| k == col) {
                    // Item-level attribute
                    html.push_str(&format!("<td>{}</td>", html_escape(v)));
                } else if let Some(sub) = child
                    .children
                    .iter()
                    .find(|c| c.tag == *col && c.children.is_empty())
                {
                    // Leaf sub-element: show text content.
                    // For elements with text, show the text. For self-closing elements
                    // with attributes (e.g. <expectedRestock date="..."/>), show attr values.
                    if !sub.text.is_empty() {
                        let mut cell = html_escape(&sub.text);
                        // Append extra attributes inline (e.g. quantity "37" with unit "pcs")
                        for (ak, av) in &sub.attrs {
                            cell.push_str(&format!(
                                " <span class=\"attr-inline\">{}</span>",
                                html_escape(av)
                            ));
                            let _ = ak; // key already contextual
                        }
                        html.push_str(&format!("<td>{}</td>", cell));
                    } else if !sub.attrs.is_empty() {
                        // Self-closing element: show first attribute value as cell content
                        let val = sub
                            .attrs
                            .iter()
                            .map(|(_, v)| v.as_str())
                            .collect::<Vec<_>>()
                            .join(", ");
                        html.push_str(&format!("<td>{}</td>", html_escape(&val)));
                    } else {
                        html.push_str("<td></td>");
                    }
                } else {
                    html.push_str("<td></td>");
                }
            }
            html.push_str("</tr>\n");

            // If child has complex sub-elements, render them below the row
            let complex_subs: Vec<&XmlNode> = child
                .children
                .iter()
                .filter(|c| !c.children.is_empty())
                .collect();
            if !complex_subs.is_empty() {
                html.push_str(&format!(
                    "<tr><td colspan=\"{}\" class=\"sub-section\">\n",
                    columns.len()
                ));
                for sub in complex_subs {
                    Self::render_xml_node(sub, html, 3, false);
                }
                html.push_str("</td></tr>\n");
            }
        }

        html.push_str("</tbody>\n</table>\n");
    }

    /// Convert XML tag name to a display-friendly label (camelCase → Title Case).
    fn xml_tag_display(tag: &str) -> String {
        let mut result = String::with_capacity(tag.len() + 4);
        let mut prev_was_upper = false;
        for (i, c) in tag.chars().enumerate() {
            if i == 0 {
                result.push(c.to_ascii_uppercase());
            } else if c.is_uppercase() && !prev_was_upper {
                result.push(' ');
                result.push(c);
            } else if c == '_' || c == '-' {
                result.push(' ');
            } else {
                result.push(c);
            }
            prev_was_upper = c.is_uppercase();
        }
        result
    }

    // ─── Data → Markdown ────────────────────────────────────────────

    fn data_to_markdown(input: &Path, output: &Path, from: Format) -> Result<(), ConvxError> {
        let (headers, rows) = Self::data_to_tabular(input, from)?;

        let mut md = String::new();

        // Header row
        md.push('|');
        for h in &headers {
            md.push(' ');
            md.push_str(&h.replace('|', "\\|"));
            md.push_str(" |");
        }
        md.push('\n');

        // Separator
        md.push('|');
        for _ in &headers {
            md.push_str("---|");
        }
        md.push('\n');

        // Data rows
        for row in &rows {
            md.push('|');
            for cell in row {
                md.push(' ');
                md.push_str(&cell.replace('|', "\\|"));
                md.push_str(" |");
            }
            md.push('\n');
        }

        fs::write(output, md).map_err(|e| ConvxError::FileWriteError {
            path: output.to_path_buf(),
            reason: e.to_string(),
        })
    }

    // ─── Data → PDF (via temp HTML + Pandoc/weasyprint) ─────────────

    fn data_to_pdf(input: &Path, output: &Path, from: Format) -> Result<(), ConvxError> {
        let temp_html =
            std::env::temp_dir().join(format!("convx-data2pdf-{}.html", Uuid::new_v4()));

        Self::data_to_html(input, &temp_html, from)?;

        // Use Python to call WeasyPrint API directly instead of the CLI binary.
        // This avoids macOS SIP stripping DYLD_LIBRARY_PATH from child processes
        // launched by GUI apps (Tauri). By setting the env var *inside* the Python
        // process before importing weasyprint, the native libs load correctly.
        let py = DependencyChecker::convx_python().ok_or(ConvxError::ConversionFailed {
            reason: "python3 not found. Required for PDF generation.".to_string(),
        })?;

        let lib_path = DependencyChecker::native_lib_search_path();
        let lib_env_var = DependencyChecker::lib_path_env_var();
        let script = format!(
            r#"
import os, sys
os.environ['{lib_env_var}'] = '{lib_path}' + os.pathsep + os.environ.get('{lib_env_var}', '')
try:
    import weasyprint
except ImportError:
    print("weasyprint not installed. Install: pip install weasyprint", file=sys.stderr)
    sys.exit(1)
weasyprint.HTML(filename=sys.argv[1]).write_pdf(sys.argv[2])
"#,
            lib_env_var = lib_env_var,
            lib_path = lib_path.replace('\'', "\\'"),
        );

        let mut cmd = silent_command(&py);
        cmd.args(["-c", &script]).arg(&temp_html).arg(output);
        DependencyChecker::set_lib_env(&mut cmd);
        let status = cmd.output().map_err(|e| ConvxError::ConversionFailed {
            reason: format!("Failed to execute python3 for PDF generation: {}", e),
        })?;

        let _ = fs::remove_file(&temp_html);

        if !status.status.success() {
            let stderr = String::from_utf8_lossy(&status.stderr).to_string();
            let stdout = String::from_utf8_lossy(&status.stdout).to_string();
            // WeasyPrint prints some warnings to stdout, check both
            let combined = if stderr.trim().is_empty() && !stdout.trim().is_empty() {
                stdout
            } else {
                stderr
            };
            return Err(ConvxError::ConversionFailed {
                reason: extract_tool_error(&combined),
            });
        }

        Ok(())
    }

    // ─── Python: Parquet / Arrow via pyarrow ────────────────────────

    fn run_python_pyarrow(
        input: &Path,
        output: &Path,
        from: Format,
        to: Format,
    ) -> Result<(), ConvxError> {
        let py = DependencyChecker::convx_python().ok_or(ConvxError::ConversionFailed {
            reason: "python3 not found. Required for Parquet/Arrow conversion.".to_string(),
        })?;

        if !DependencyChecker::python_has_module("pyarrow") {
            return Err(ConvxError::ConversionFailed {
                reason: "Missing Python module pyarrow. Install: pip install pyarrow".to_string(),
            });
        }

        let script = match (from, to) {
            (Format::Parquet, Format::Csv) => {
                "import pyarrow.parquet as pq,pyarrow.csv as ac,sys; t=pq.read_table(sys.argv[1]); ac.write_csv(t,sys.argv[2])"
            }
            (Format::Csv, Format::Parquet) => {
                "import pyarrow.csv as ac,pyarrow.parquet as pq,sys; t=ac.read_csv(sys.argv[1]); pq.write_table(t,sys.argv[2])"
            }
            (Format::Parquet, Format::Json) => {
                "import pyarrow.parquet as pq,json,sys; t=pq.read_table(sys.argv[1]); json.dump(t.to_pylist(),open(sys.argv[2],'w'),default=str,indent=2)"
            }
            (Format::Json, Format::Parquet) => {
                "import pyarrow as pa,pyarrow.parquet as pq,json,sys; d=json.load(open(sys.argv[1])); t=pa.Table.from_pylist(d); pq.write_table(t,sys.argv[2])"
            }
            (Format::Arrow, Format::Csv) => {
                "import pyarrow as pa,pyarrow.csv as ac,sys; f=pa.ipc.open_file(sys.argv[1]); t=f.read_all(); ac.write_csv(t,sys.argv[2])"
            }
            (Format::Csv, Format::Arrow) => {
                "import pyarrow as pa,pyarrow.csv as ac,sys; t=ac.read_csv(sys.argv[1]); w=pa.ipc.new_file(sys.argv[2],t.schema); w.write_table(t); w.close()"
            }
            (Format::Arrow, Format::Json) => {
                "import pyarrow as pa,json,sys; f=pa.ipc.open_file(sys.argv[1]); t=f.read_all(); json.dump(t.to_pylist(),open(sys.argv[2],'w'),default=str,indent=2)"
            }
            (Format::Json, Format::Arrow) => {
                "import pyarrow as pa,json,sys; d=json.load(open(sys.argv[1])); t=pa.Table.from_pylist(d); w=pa.ipc.new_file(sys.argv[2],t.schema); w.write_table(t); w.close()"
            }
            _ => return Err(ConvxError::UnsupportedConversion { from, to }),
        };

        Self::run_python_script(&py, script, input, output)
    }

    // ─── Python: SQLite via stdlib sqlite3 ──────────────────────────

    fn run_python_sqlite(
        input: &Path,
        output: &Path,
        _from: Format,
        to: Format,
    ) -> Result<(), ConvxError> {
        let py = DependencyChecker::convx_python().ok_or(ConvxError::ConversionFailed {
            reason: "python3 not found. Required for SQLite conversion.".to_string(),
        })?;

        let script = match to {
            Format::Csv => {
                "import sqlite3,csv,sys\n\
                 conn=sqlite3.connect(sys.argv[1])\n\
                 cur=conn.cursor()\n\
                 tables=[r[0] for r in cur.execute(\"SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'\").fetchall()]\n\
                 if not tables: print('No tables found',file=sys.stderr); sys.exit(1)\n\
                 cur.execute(f'SELECT * FROM \"{tables[0]}\"')\n\
                 cols=[d[0] for d in cur.description]\n\
                 w=csv.writer(open(sys.argv[2],'w',newline=''))\n\
                 w.writerow(cols)\n\
                 w.writerows(cur.fetchall())\n\
                 conn.close()"
            }
            Format::Json => {
                "import sqlite3,json,sys\n\
                 conn=sqlite3.connect(sys.argv[1])\n\
                 cur=conn.cursor()\n\
                 tables=[r[0] for r in cur.execute(\"SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'\").fetchall()]\n\
                 if not tables: print('No tables found',file=sys.stderr); sys.exit(1)\n\
                 cur.execute(f'SELECT * FROM \"{tables[0]}\"')\n\
                 cols=[d[0] for d in cur.description]\n\
                 rows=[dict(zip(cols,r)) for r in cur.fetchall()]\n\
                 json.dump(rows,open(sys.argv[2],'w'),default=str,indent=2)\n\
                 conn.close()"
            }
            _ => {
                return Err(ConvxError::UnsupportedConversion {
                    from: Format::Sqlite,
                    to,
                })
            }
        };

        Self::run_python_script(&py, script, input, output)
    }

    // ─── Python: NPY/NPZ via numpy ─────────────────────────────────

    fn run_python_numpy(
        input: &Path,
        output: &Path,
        from: Format,
        _to: Format,
    ) -> Result<(), ConvxError> {
        let py = DependencyChecker::convx_python().ok_or(ConvxError::ConversionFailed {
            reason: "python3 not found. Required for NPY/NPZ conversion.".to_string(),
        })?;

        if !DependencyChecker::python_has_module("numpy") {
            return Err(ConvxError::ConversionFailed {
                reason: "Missing Python module numpy. Install: pip install numpy".to_string(),
            });
        }

        let script = match from {
            Format::Npy => {
                "import numpy as np,sys; d=np.load(sys.argv[1]); \
                 np.savetxt(sys.argv[2],d.reshape(-1,d.shape[-1]) if d.ndim>1 else d.reshape(-1,1),delimiter=',',fmt='%s')"
            }
            Format::Npz => {
                "import numpy as np,sys; f=np.load(sys.argv[1]); d=f[f.files[0]]; \
                 np.savetxt(sys.argv[2],d.reshape(-1,d.shape[-1]) if d.ndim>1 else d.reshape(-1,1),delimiter=',',fmt='%s')"
            }
            _ => {
                return Err(ConvxError::UnsupportedConversion {
                    from,
                    to: Format::Csv,
                })
            }
        };

        Self::run_python_script(&py, script, input, output)
    }

    // ─── Python: HDF5 via h5py ──────────────────────────────────────

    fn run_python_hdf5(
        input: &Path,
        output: &Path,
        _from: Format,
        to: Format,
    ) -> Result<(), ConvxError> {
        let py = DependencyChecker::convx_python().ok_or(ConvxError::ConversionFailed {
            reason: "python3 not found. Required for HDF5 conversion.".to_string(),
        })?;

        if !DependencyChecker::python_has_module("h5py") {
            return Err(ConvxError::ConversionFailed {
                reason: "Missing Python module h5py. Install: pip install h5py".to_string(),
            });
        }

        let script = match to {
            Format::Csv => {
                "import h5py,numpy as np,sys\n\
f=h5py.File(sys.argv[1],'r')\n\
ds=[]\n\
def v(name,obj):\n  if isinstance(obj,h5py.Dataset): ds.append(name)\n\
f.visititems(v)\n\
if not ds: print('No datasets found',file=sys.stderr); sys.exit(1)\n\
d=np.array(f[ds[0]])\n\
np.savetxt(sys.argv[2],d.reshape(-1,d.shape[-1]) if d.ndim>1 else d.reshape(-1,1),delimiter=',',fmt='%s')\n\
f.close()"
            }
            Format::Json => {
                "import h5py,json,numpy as np,sys\n\
f=h5py.File(sys.argv[1],'r')\n\
ds=[]\n\
def v(name,obj):\n  if isinstance(obj,h5py.Dataset): ds.append(name)\n\
f.visititems(v)\n\
if not ds: print('No datasets found',file=sys.stderr); sys.exit(1)\n\
d=np.array(f[ds[0]])\n\
json.dump(d.tolist(),open(sys.argv[2],'w'),indent=2)\n\
f.close()"
            }
            _ => {
                return Err(ConvxError::UnsupportedConversion {
                    from: Format::Hdf5,
                    to,
                })
            }
        };

        Self::run_python_script(&py, script, input, output)
    }

    // ─── Shared Python runner ───────────────────────────────────────

    fn run_python_script(
        py: &str,
        script: &str,
        input: &Path,
        output: &Path,
    ) -> Result<(), ConvxError> {
        let mut cmd = silent_command(py);
        cmd.args(["-c", script]).arg(input).arg(output);
        // Some Python packages (weasyprint, etc.) need native libs via ctypes
        DependencyChecker::set_lib_env(&mut cmd);
        let out = cmd.output().map_err(|e| ConvxError::ConversionFailed {
            reason: format!("Failed to execute python3: {}", e),
        })?;

        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            return Err(ConvxError::ConversionFailed {
                reason: extract_tool_error(&stderr),
            });
        }

        Ok(())
    }

    // ─── Public interface ───────────────────────────────────────────

    pub fn can_convert(&self, from: Format, to: Format) -> bool {
        Self::can_convert_pair(from, to)
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

        match (input_format, options.output_format) {
            // Existing
            (Format::Csv, Format::Xlsx) | (Format::Xlsx, Format::Csv) => {
                Self::run_python_pandas(input, output, input_format, options.output_format)?
            }
            (Format::Json, Format::Csv) => Self::json_to_csv(input, output)?,
            (Format::Csv, Format::Json) => Self::csv_to_json(input, output)?,
            (Format::Json, Format::Yaml) => Self::json_to_yaml(input, output)?,
            (Format::Yaml, Format::Json) => Self::yaml_to_json(input, output)?,
            (Format::Xml, Format::Json) => Self::xml_to_json(input, output)?,
            (Format::Json, Format::Xml) => Self::json_to_xml(input, output)?,
            // TSV
            (Format::Tsv, Format::Csv) => Self::tsv_to_csv(input, output)?,
            (Format::Csv, Format::Tsv) => Self::csv_to_tsv(input, output)?,
            // JSONL
            (Format::Jsonl, Format::Json) => Self::jsonl_to_json(input, output)?,
            (Format::Json, Format::Jsonl) => Self::json_to_jsonl(input, output)?,
            (Format::Jsonl, Format::Csv) => Self::jsonl_to_csv(input, output)?,
            (Format::Csv, Format::Jsonl) => Self::csv_to_jsonl(input, output)?,
            // Data -> HTML
            (
                f @ (Format::Json
                | Format::Csv
                | Format::Xml
                | Format::Yaml
                | Format::Tsv
                | Format::Jsonl),
                Format::Html,
            ) => Self::data_to_html(input, output, f)?,
            // Data -> PDF
            (
                f @ (Format::Json
                | Format::Csv
                | Format::Xml
                | Format::Yaml
                | Format::Tsv
                | Format::Jsonl),
                Format::Pdf,
            ) => Self::data_to_pdf(input, output, f)?,
            // Data -> Markdown
            (f @ (Format::Json | Format::Csv), Format::Md) => {
                Self::data_to_markdown(input, output, f)?
            }
            // Parquet / Arrow via pyarrow
            (Format::Parquet, Format::Csv)
            | (Format::Csv, Format::Parquet)
            | (Format::Parquet, Format::Json)
            | (Format::Json, Format::Parquet)
            | (Format::Arrow, Format::Csv)
            | (Format::Csv, Format::Arrow)
            | (Format::Arrow, Format::Json)
            | (Format::Json, Format::Arrow) => {
                Self::run_python_pyarrow(input, output, input_format, options.output_format)?
            }
            // SQLite
            (Format::Sqlite, Format::Csv) | (Format::Sqlite, Format::Json) => {
                Self::run_python_sqlite(input, output, input_format, options.output_format)?
            }
            // NPY / NPZ
            (Format::Npy, Format::Csv) | (Format::Npz, Format::Csv) => {
                Self::run_python_numpy(input, output, input_format, options.output_format)?
            }
            // HDF5
            (Format::Hdf5, Format::Csv) | (Format::Hdf5, Format::Json) => {
                Self::run_python_hdf5(input, output, input_format, options.output_format)?
            }
            _ => {
                return Err(ConvxError::UnsupportedConversion {
                    from: input_format,
                    to: options.output_format,
                })
            }
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

impl Converter for DataConverter {
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

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
