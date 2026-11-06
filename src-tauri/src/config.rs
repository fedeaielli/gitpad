use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};
use tauri_plugin_log::log;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Config {
    pub onboarding_complete: bool,
    pub theme: String,
    pub preview_mode: String,
    pub font_size: u32,
    pub font_family: String,
    pub sidebar_width: u32,
    pub sidebar_open: bool,
    pub auto_sync: bool,
    pub save_debounce_ms: u32,
    pub push_idle_ms: u32,
    pub keybindings: String,
    pub extensions: BTreeMap<String, String>,
    /// Hex accent override; empty means "use the theme's own accent".
    pub accent: String,
    /// File tree ordering: "name", "modified" or "type".
    pub sort_mode: String,
    pub sort_desc: bool,
    /// Keep folders above files whatever the ordering.
    pub folders_first: bool,
    pub show_hidden: bool,
    /// Clone location picked by the user; empty means "the default repos dir".
    pub repo_path: String,
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
            sidebar_width: 240,
            sidebar_open: true,
            auto_sync: true,
            save_debounce_ms: 2000,
            push_idle_ms: 30_000,
            keybindings: "vscode".into(),
            extensions,
            accent: String::new(),
            sort_mode: "name".into(),
            sort_desc: false,
            folders_first: false,
            show_hidden: false,
            repo_path: String::new(),
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
    let Ok(raw) = std::fs::read_to_string(path) else {
        return Config::default();
    };
    let mut values = match serde_json::from_str::<Value>(&raw) {
        Ok(Value::Object(map)) => map,
        Ok(_) => {
            log::warn!(target: "app", "{path:?} is not a config object, using defaults");
            set_aside(path);
            return Config::default();
        }
        Err(e) => {
            log::warn!(target: "app", "cannot parse {path:?}: {e}, using defaults");
            set_aside(path);
            return Config::default();
        }
    };
    // Read field by field: one wrong type used to sink the whole file, so every
    // preference came back at its default with no trace of why.
    let default = Config::default();
    Config {
        onboarding_complete: take(&mut values, "onboardingComplete", default.onboarding_complete),
        theme: take(&mut values, "theme", default.theme),
        preview_mode: take(&mut values, "previewMode", default.preview_mode),
        font_size: take(&mut values, "fontSize", default.font_size),
        font_family: take(&mut values, "fontFamily", default.font_family),
        sidebar_width: take(&mut values, "sidebarWidth", default.sidebar_width),
        sidebar_open: take(&mut values, "sidebarOpen", default.sidebar_open),
        auto_sync: take(&mut values, "autoSync", default.auto_sync),
        save_debounce_ms: take(&mut values, "saveDebounceMs", default.save_debounce_ms),
        push_idle_ms: take(&mut values, "pushIdleMs", default.push_idle_ms),
        keybindings: take(&mut values, "keybindings", default.keybindings),
        extensions: take(&mut values, "extensions", default.extensions),
        accent: take(&mut values, "accent", default.accent),
        sort_mode: take(&mut values, "sortMode", default.sort_mode),
        sort_desc: take(&mut values, "sortDesc", default.sort_desc),
        folders_first: take(&mut values, "foldersFirst", default.folders_first),
        show_hidden: take(&mut values, "showHidden", default.show_hidden),
        repo_path: take(&mut values, "repoPath", default.repo_path),
    }
}

/// The next save would overwrite the only copy of a file we cannot read.
fn set_aside(path: &Path) {
    let backup = path.with_extension("json.corrupt");
    if let Err(e) = std::fs::rename(path, &backup) {
        log::warn!(target: "app", "cannot set aside {path:?}: {e}");
    }
}

fn take<T: DeserializeOwned>(values: &mut Map<String, Value>, key: &str, fallback: T) -> T {
    let Some(value) = values.remove(key) else {
        return fallback;
    };
    serde_json::from_value(value).unwrap_or_else(|e| {
        log::warn!(target: "app", "ignoring {key}: {e}");
        fallback
    })
}

/// Sync Tauri commands run on the main thread, so two saves never overlap and
/// the fixed `config.json.tmp` name is safe.
fn write_config(path: &Path, config: &Config) -> Result<(), String> {
    let raw = serde_json::to_string_pretty(config).map_err(|e| format!("serialize config: {e}"))?;
    // Same directory as config.json so the rename stays atomic.
    let tmp = path.with_extension("json.tmp");
    crate::fs::write_atomic(path, &tmp, raw.as_bytes()).map_err(|e| format!("write config: {e}"))
}

/// Clone path chosen by the user; `None` when the config keeps the default.
pub fn configured_repo_path(app: &AppHandle) -> Option<PathBuf> {
    let path = config_path(app).ok()?;
    let configured = read_config(&path).repo_path;
    (!configured.is_empty()).then(|| PathBuf::from(configured))
}

/// Saves the clone location from Rust: the frontend's debounced save can be
/// lost if the app closes right after a move.
pub fn store_repo_path(app: &AppHandle, path: &Path) -> Result<(), String> {
    let file = config_path(app)?;
    let mut config = read_config(&file);
    config.repo_path = path.to_string_lossy().into_owned();
    write_config(&file, &config)
}

#[tauri::command]
pub fn config_get(app: AppHandle) -> Result<Config, String> {
    Ok(read_config(&config_path(&app)?))
}
#[tauri::command]
pub fn config_save(app: AppHandle, config: Config) -> Result<(), String> {
    let result = config_path(&app).and_then(|file| write_config(&file, &config));
    // The frontend only shows a toast; the log is what is left the next day.
    if let Err(e) = &result {
        log::warn!(target: "app", "config_save: {e}");
    }
    result
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
            sidebar_width: 300,
            sidebar_open: false,
            auto_sync: false,
            save_debounce_ms: 1500,
            push_idle_ms: 60_000,
            keybindings: "vim".into(),
            extensions: [("md".to_string(), "markdown".to_string())].into(),
            accent: "#10b981".into(),
            sort_mode: "modified".into(),
            sort_desc: true,
            folders_first: true,
            show_hidden: true,
            repo_path: "/tmp/notes".into(),
        };
        write_config(&path, &config).unwrap();
        assert!(!dir.join("config.json.tmp").exists());
        let loaded = read_config(&path);
        assert!(loaded.onboarding_complete);
        assert_eq!(loaded.theme, "vs");
        assert_eq!(loaded.preview_mode, "split");
        assert_eq!(loaded.font_size, 16);
        assert_eq!(loaded.sidebar_width, 300);
        assert!(!loaded.sidebar_open);
        assert_eq!(loaded.sort_mode, "modified");
        assert!(loaded.sort_desc);
        assert!(loaded.folders_first);
        assert!(loaded.show_hidden);
        assert_eq!(loaded.repo_path, "/tmp/notes");
        assert!(!loaded.auto_sync);
        assert_eq!(loaded.keybindings, "vim");
        assert_eq!(loaded.extensions.len(), 1);
        assert_eq!(loaded.accent, "#10b981");
        std::fs::write(&path, "not json").unwrap();
        assert_eq!(read_config(&path).theme, "gitpad-dark");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// One bad value must not reset every other preference.
    #[test]
    fn a_bad_field_falls_back_alone() {
        let dir = std::env::temp_dir().join(format!("gitpad-config-bad-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        std::fs::write(
            &path,
            r#"{"theme":"vs","fontSize":13.5,"sortMode":"type","sortDesc":true}"#,
        )
        .unwrap();
        let config = read_config(&path);
        assert_eq!(config.theme, "vs");
        assert_eq!(config.font_size, 13);
        assert_eq!(config.sort_mode, "type");
        assert!(config.sort_desc);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Whatever we cannot read is kept: the next save overwrites it.
    #[test]
    fn a_corrupt_file_is_kept_aside() {
        let dir = std::env::temp_dir().join(format!("gitpad-config-corrupt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        std::fs::write(&path, "not json").unwrap();
        let config = read_config(&path);
        assert_eq!(config.theme, "gitpad-dark");
        assert!(!path.exists());
        assert!(dir.join("config.json.corrupt").exists());
        assert_eq!(std::fs::read_to_string(dir.join("config.json.corrupt")).unwrap(), "not json");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A config written before the settings fields existed must still load.
    #[test]
    fn old_config_gets_defaults_for_new_fields() {        let dir = std::env::temp_dir().join(format!("gitpad-config-old-{}", std::process::id()));
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
        assert!(!config.folders_first);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
