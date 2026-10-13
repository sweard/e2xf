use std::{collections::VecDeque, path::Path};

use excel_to_xml::{ConversionMode, ConversionReport};
use serde_json::Value;

use crate::settings::Settings;

pub const MAX_LOG_LINES: usize = 1000;

#[derive(Default)]
pub struct LogBuffer(VecDeque<String>);

impl LogBuffer {
    pub fn append(&mut self, message: &str) {
        for line in message.lines() {
            if self.0.len() == MAX_LOG_LINES {
                self.0.pop_front();
            }
            self.0.push_back(line.to_owned());
        }
    }

    pub fn text(&self) -> String {
        self.0
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }
}

pub fn validate_config(config: &str, sheets: &[String]) -> Result<Value, String> {
    let json: Value =
        serde_json::from_str(config).map_err(|error| format!("JSON 格式错误: {error}"))?;
    let object = json.as_object().ok_or("配置必须是 JSON 对象")?;
    if object.get("tagName").and_then(Value::as_str).is_none() {
        return Err("tagName 必须是字符串".into());
    }
    for field in ["langMap", "escapeOnly"] {
        if !object.get(field).is_some_and(Value::is_object) {
            return Err(format!("{field} 必须是 JSON 对象"));
        }
    }
    if let Some(sheet) = object.get("sheetName") {
        let name = sheet.as_str().ok_or("sheetName 必须是字符串")?;
        if !name.is_empty() && !sheets.is_empty() && !sheets.iter().any(|sheet| sheet == name) {
            return Err(format!("Excel 中没有对应的工作表: {name}"));
        }
    }
    Ok(json)
}

pub fn apply_sheet_names(settings: &mut Settings, sheets: &[String]) -> Result<(), String> {
    let mut json = validate_config(&settings.config, sheets)?;
    if json
        .get("sheetName")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .is_empty()
    {
        let first = sheets.first().ok_or("Excel 中没有工作表")?;
        json["sheetName"] = Value::String(first.clone());
        settings.config = serde_json::to_string_pretty(&json).map_err(|error| error.to_string())?;
    }
    Ok(())
}

/// Snapshot all input before launching a worker; edits cannot change an in-flight conversion.
pub struct ConversionRequest {
    settings: Settings,
}

impl ConversionRequest {
    pub fn new(settings: &Settings, sheets: &[String]) -> Result<Self, String> {
        validate_config(&settings.config, sheets)?;
        if !Path::new(&settings.excel_path).is_file() {
            return Err("请选择可访问的 Excel 文件".into());
        }
        if !Path::new(&settings.xml_folder_path).is_dir() {
            return Err("请选择可访问的模块文件夹".into());
        }
        Ok(Self {
            settings: settings.clone(),
        })
    }

    pub fn run(self) -> Result<ConversionReport, String> {
        let mode = if self.settings.quick_mode {
            ConversionMode::Fast
        } else {
            ConversionMode::LowMemory
        };
        excel_to_xml::convert(
            &self.settings.config,
            &self.settings.excel_path,
            &self.settings.xml_folder_path,
            mode,
        )
        .map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_is_bounded_even_for_multiline_messages() {
        let mut logs = LogBuffer::default();
        logs.append(
            &(0..1005)
                .map(|i| format!("line-{i}"))
                .collect::<Vec<_>>()
                .join("\n"),
        );
        let text = logs.text();
        assert_eq!(text.lines().count(), MAX_LOG_LINES);
        assert!(text.starts_with("line-5\n"));
        assert!(text.ends_with("line-1004"));
        logs.clear();
        assert_eq!(logs.text(), "");
    }

    #[test]
    fn auto_selects_first_sheet_without_changing_explicit_selection() {
        let mut settings = Settings::default();
        apply_sheet_names(&mut settings, &["翻译".into(), "Sheet2".into()]).unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&settings.config).unwrap()["sheetName"],
            "翻译"
        );
        assert!(apply_sheet_names(&mut settings, &["Sheet2".into()]).is_err());
        assert_eq!(
            serde_json::from_str::<Value>(&settings.config).unwrap()["sheetName"],
            "翻译"
        );
    }

    #[test]
    fn invalid_json_and_missing_paths_are_reported_before_conversion() {
        assert!(validate_config("{", &[]).is_err());
        assert!(validate_config("[]", &[]).is_err());
        assert!(validate_config("{}", &[]).is_err());
        assert!(ConversionRequest::new(&Settings::default(), &[]).is_err());
    }
}
