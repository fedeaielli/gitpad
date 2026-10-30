use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Config {
    pub onboarding_complete: bool,
    pub theme: String,
    pub preview_mode: String,
    pub font_size: u32,
    pub font_family: String,
    pub auto_sync: bool,
    pub save_debounce_ms: u32,
    pub push_idle_ms: u32,
    pub keybindings: String,
    pub extensions: BTreeMap<String, String>,
}

impl Default for Config {
    fn default() -> Self {
        let extensions = [
            ("txt", "plaintext"),
            ("md", "markdown"),
            ("json", "json"),
            ("yaml", "yaml"),
            ("toml", "toml"),
            ("sh", "shell"),
            ("py", "python"),
            ("rs", "rust"),
            ("js", "javascript"),
            ("ts", "typescript"),
        ]
        .into_iter()
        .map(|(ext, lang)| (ext.to_string(), lang.to_string()))
        .collect();
        Self {
            onboarding_complete: false,
            theme: "gitpad-dark".into(),
            preview_mode: "off".into(),
            font_size: 13,
            font_family: "ui-monospace, monospace".into(),
            auto_sync: true,
            save_debounce_ms: 2000,
            push_idle_ms: 30_000,
            keybindings: "vscode".into(),
            extensions,
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
        assert_eq!(config.font_size, 13);
        assert!(config.auto_sync);
        assert_eq!(config.extensions.get("md").map(String::as_str), Some("markdown"));
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
            font_size: 16,
            font_family: "monospace".into(),
            auto_sync: false,
            save_debounce_ms: 1500,
            push_idle_ms: 60_000,
            keybindings: "vim".into(),
            extensions: [("md".to_string(), "markdown".to_string())].into(),
        };
        write_config(&path, &config).unwrap();
        let loaded = read_config(&path);
        assert!(loaded.onboarding_complete);
        assert_eq!(loaded.theme, "vs");
        assert_eq!(loaded.preview_mode, "split");
        assert_eq!(loaded.font_size, 16);
        assert!(!loaded.auto_sync);
        assert_eq!(loaded.keybindings, "vim");
        assert_eq!(loaded.extensions.len(), 1);
        std::fs::write(&path, "not json").unwrap();
        assert_eq!(read_config(&path).theme, "gitpad-dark");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A config written before the settings fields existed must still load.
    #[test]
    fn old_config_gets_defaults_for_new_fields() {
        let dir = std::env::temp_dir().join(format!("gitpad-config-old-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        std::fs::write(
            &path,
            r#"{"onboardingComplete":true,"theme":"vs-dark","previewMode":"full"}"#,
        )
        .unwrap();
        let config = read_config(&path);
        assert!(config.onboarding_complete);
        assert_eq!(config.theme, "vs-dark");
        assert_eq!(config.font_size, 13);
        assert_eq!(config.save_debounce_ms, 2000);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
