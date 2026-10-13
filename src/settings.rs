use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub config: String,
    pub excel_path: String,
    pub xml_folder_path: String,
    pub quick_mode: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            config: excel_to_xml::get_default_cfg_json(),
            excel_path: String::new(),
            xml_folder_path: String::new(),
            quick_mode: true,
        }
    }
}

pub struct SettingsStore {
    path: PathBuf,
}

impl SettingsStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn default_path() -> Result<PathBuf, String> {
        dirs::config_dir()
            .map(|path| path.join("e2xf/settings.json"))
            .ok_or_else(|| "无法取得用户配置目录".into())
    }

    pub fn load(&self) -> Result<Settings, String> {
        if self.path.exists() {
            let data = fs::read(&self.path).map_err(|error| format!("读取配置失败: {error}"))?;
            return serde_json::from_slice(&data)
                .map_err(|error| format!("保存的配置损坏: {error}"));
        }
        Ok(Settings::default())
    }

    pub fn import_flutter(&self) -> Result<Option<Settings>, String> {
        if self.path.exists() {
            return Ok(None);
        }
        for path in legacy_json_paths() {
            if path.is_file() {
                let json: Value =
                    serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
                        .map_err(|e| e.to_string())?;
                return Ok(Some(from_flutter_json(&json)));
            }
        }
        #[cfg(target_os = "macos")]
        if let Some(home) = dirs::home_dir() {
            for path in [
                home.join(
                    "Library/Containers/com.zole.e2xf/Data/Library/Preferences/com.zole.e2xf.plist",
                ),
                home.join("Library/Preferences/com.zole.e2xf.plist"),
            ] {
                if path.is_file() {
                    let plist = plist::Value::from_file(path).map_err(|e| e.to_string())?;
                    if let Some(dict) = plist.as_dictionary() {
                        let mut json = serde_json::Map::new();
                        for (key, value) in dict {
                            if let Some(text) = value.as_string() {
                                json.insert(key.clone(), Value::String(text.into()));
                            }
                        }
                        return Ok(Some(from_flutter_json(&Value::Object(json))));
                    }
                }
            }
        }
        Ok(None)
    }

    pub fn save(&self, settings: &Settings) -> Result<(), String> {
        let directory = self
            .path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        fs::create_dir_all(directory).map_err(|e| format!("创建配置目录失败: {e}"))?;
        let mut temporary =
            tempfile::NamedTempFile::new_in(directory).map_err(|e| e.to_string())?;
        serde_json::to_writer_pretty(&mut temporary, settings).map_err(|e| e.to_string())?;
        temporary.flush().map_err(|e| e.to_string())?;
        temporary.as_file().sync_all().map_err(|e| e.to_string())?;
        temporary
            .persist(&self.path)
            .map_err(|e| format!("保存配置失败: {e}"))?;
        Ok(())
    }
}

fn legacy_json_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(data) = dirs::data_dir() {
        paths.push(data.join("e2xf/shared_preferences.json"));
        paths.push(data.join("com.zole/e2xf/shared_preferences.json"));
        paths.push(data.join("com.zole.e2xf/shared_preferences.json"));
    }
    paths
}

fn from_flutter_json(json: &Value) -> Settings {
    let string = |key: &str| {
        json.get(format!("flutter.{key}"))
            .or_else(|| json.get(key))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let config = string("defaultCfg");
    Settings {
        config: if config.is_empty() {
            excel_to_xml::get_default_cfg_json()
        } else {
            config
        },
        excel_path: string("excelPath"),
        xml_folder_path: string("xmlFolderPath"),
        quick_mode: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_survive_atomic_replacement_and_unicode_paths() {
        let directory = tempfile::tempdir().unwrap();
        let store = SettingsStore::new(directory.path().join("config/settings.json"));
        let mut settings = Settings {
            excel_path: "翻译/文案.xlsx".into(),
            ..Settings::default()
        };
        store.save(&settings).unwrap();
        settings.quick_mode = false;
        store.save(&settings).unwrap();
        assert_eq!(store.load().unwrap(), settings);
    }

    #[test]
    fn corrupt_settings_are_not_silently_overwritten() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.json");
        fs::write(&path, "broken").unwrap();
        assert!(SettingsStore::new(path.clone()).load().is_err());
        assert_eq!(fs::read_to_string(path).unwrap(), "broken");
    }

    #[test]
    fn reads_legacy_flutter_keys() {
        let settings = from_flutter_json(
            &serde_json::json!({"flutter.defaultCfg": "{}", "flutter.excelPath": "/tmp/翻译.xlsx", "flutter.xmlFolderPath": "/tmp/res"}),
        );
        assert_eq!(settings.config, "{}");
        assert_eq!(settings.excel_path, "/tmp/翻译.xlsx");
        assert_eq!(settings.xml_folder_path, "/tmp/res");
    }
}
