use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Config {
    pub onboarding_complete: bool,
    pub theme: String,
    pub preview_mode: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            onboarding_complete: false,
            theme: "gitpad-dark".into(),
            preview_mode: "off".into(),
        }
    }
}

fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("config dir: {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("create config dir: {e}"))?;
    Ok(dir.join("config.json"))
}

/// Missing or corrupt file falls back to defaults instead of failing startup.
fn read_config(path: &Path) -> Config {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn write_config(path: &Path, config: &Config) -> Result<(), String> {
    let raw = serde_json::to_string_pretty(config).map_err(|e| format!("serialize config: {e}"))?;
    std::fs::write(path, raw).map_err(|e| format!("write config: {e}"))
}

#[tauri::command]
pub fn config_get(app: AppHandle) -> Result<Config, String> {
    Ok(read_config(&config_path(&app)?))
}

#[tauri::command]
pub fn config_save(app: AppHandle, config: Config) -> Result<(), String> {
    write_config(&config_path(&app)?, &config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_yields_defaults() {
        let dir = std::env::temp_dir().join(format!("gitpad-config-missing-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let config = read_config(&dir.join("config.json"));
        assert!(!config.onboarding_complete);
        assert_eq!(config.theme, "gitpad-dark");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn roundtrip_keeps_values_and_ignores_garbage() {
        let dir = std::env::temp_dir().join(format!("gitpad-config-rt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        let config = Config {
            onboarding_complete: true,
            theme: "vs".into(),
            preview_mode: "split".into(),
        };
        write_config(&path, &config).unwrap();
        let loaded = read_config(&path);
        assert!(loaded.onboarding_complete);
        assert_eq!(loaded.theme, "vs");
        assert_eq!(loaded.preview_mode, "split");
        std::fs::write(&path, "not json").unwrap();
        assert_eq!(read_config(&path).theme, "gitpad-dark");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
