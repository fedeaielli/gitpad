use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use tauri_plugin_log::log;

pub const SERVICE: &str = "com.gitpad.app";
pub const TOKEN_KEY: &str = "github_token";

/// The token when the keyring cannot hold it: a session-only copy, so a machine
/// whose Secret Service is missing or locked can still sign in.
static SESSION_TOKEN: Mutex<Option<String>> = Mutex::new(None);

/// Set once in `lib.rs` setup: where the token goes when the keyring is down.
/// Nobody except this module ever reads the file.
static FALLBACK_DIR: OnceLock<PathBuf> = OnceLock::new();
const TOKEN_FILE: &str = "auth.token";

pub fn use_fallback_dir(dir: PathBuf) {
    let _ = FALLBACK_DIR.set(dir);
}

fn fallback_path() -> Option<PathBuf> {
    FALLBACK_DIR.get().map(|dir| dir.join(TOKEN_FILE))
}

fn session_token() -> Option<String> {
    SESSION_TOKEN.lock().ok().and_then(|held| held.clone())
}

fn set_session_token(token: Option<&str>) {
    if let Ok(mut held) = SESSION_TOKEN.lock() {
        *held = token.map(str::to_string);
    }
}

/// 0600 from creation: the token must never be world-readable, even briefly.
fn write_secret(path: &Path, value: &str) -> Result<(), String> {
    use std::os::unix::fs::OpenOptionsExt;

    let dir = path
        .parent()
        .ok_or_else(|| "credential path has no parent".to_string())?;
    std::fs::create_dir_all(dir).map_err(|e| format!("credential dir {}: {e}", dir.display()))?;
    let tmp = path.with_extension("token.tmp");
    let result = (|| -> std::io::Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&tmp)?;
        file.write_all(value.as_bytes())?;
        file.sync_all()?;
        std::fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result.map_err(|e| format!("credential file {}: {e}", path.display()))
}

/// A missing file is normal; a broken one only costs a warning.
fn read_secret(path: &Path) -> Option<String> {
    match std::fs::read_to_string(path) {
        Ok(value) if !value.is_empty() => Some(value),
        Ok(_) => None,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => {
            log::warn!(target: "app", "auth: credential file: {e}");
            None
        }
    }
}

fn clear_secret(path: &Path) {
    match std::fs::remove_file(path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => log::warn!(target: "app", "auth: remove credential file: {e}"),
    }
}

/// Some launchers start GitPad without the session bus, so D-Bus tries the
/// disabled X11 autolaunch and every keyring call fails. The socket a desktop
/// session exports is the same address, so set it before anything uses it.
pub fn ensure_session_bus() {
    if std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_some() {
        return;
    }
    if let Some(address) = bus_address(std::env::var_os("XDG_RUNTIME_DIR").as_deref()) {
        // Safe here: called from `run` before Tauri starts any thread.
        unsafe { std::env::set_var("DBUS_SESSION_BUS_ADDRESS", address) }
    }
}

fn bus_address(runtime_dir: Option<&std::ffi::OsStr>) -> Option<String> {
    let socket = std::path::Path::new(runtime_dir?).join("bus");
    socket
        .exists()
        .then(|| format!("unix:path={}", socket.display()))
}

fn entry(key: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, key).map_err(|e| format!("keyring entry: {e}"))
}

fn keyring_write(key: &str, value: &str) -> Result<(), String> {
    entry(key).and_then(|entry| {
        entry
            .set_password(value)
            .map_err(|e| format!("keyring store: {e}"))
    })
}

fn keyring_read(key: &str) -> Result<Option<String>, String> {
    entry(key).and_then(|entry| match entry.get_password() {
        Ok(value) if !value.is_empty() => Ok(Some(value)),
        Ok(_) | Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("keyring get: {e}")),
    })
}

/// Writes one secret, reporting whether the keyring took it. Private on purpose:
/// the webview can neither read a secret nor pick the key it lands on, so only
/// `auth_set_pat` and the device flow touch the keyring.
fn store(key: &str, value: &str) -> bool {
    let fallback = fallback_path();
    store_with(
        key,
        value,
        || keyring_write(key, value),
        fallback.as_deref(),
    )
}

fn store_with(
    key: &str,
    value: &str,
    write: impl FnOnce() -> Result<(), String>,
    fallback: Option<&Path>,
) -> bool {
    match write() {
        Ok(()) => {
            if key == TOKEN_KEY {
                set_session_token(None);
                // The keyring now owns the token: drop a stale fallback copy.
                if let Some(path) = fallback {
                    clear_secret(path);
                }
            }
            true
        }
        Err(e) => {
            // A missing or locked Secret Service is not fatal: keep the token in
            // a protected file, or in memory if even that fails.
            log::warn!(target: "app", "auth: keyring unavailable: {e}");
            if key == TOKEN_KEY {
                set_session_token(Some(value));
                if let Some(path) = fallback {
                    match write_secret(path, value) {
                        Ok(()) => log::info!(target: "app", "auth: token kept in a protected file"),
                        Err(e) => {
                            log::warn!(target: "app", "auth: {e}; token lasts this session only")
                        }
                    }
                }
            }
            false
        }
    }
}

fn read(key: &str) -> Option<String> {
    let fallback = fallback_path();
    read_with(key, || keyring_read(key), fallback.as_deref())
}

fn read_with(
    key: &str,
    read: impl FnOnce() -> Result<Option<String>, String>,
    fallback: Option<&Path>,
) -> Option<String> {
    match read() {
        Ok(Some(value)) => {
            // The keyring wins; a file left by a keyring-less session is stale.
            if key == TOKEN_KEY {
                if let Some(path) = fallback {
                    clear_secret(path);
                }
            }
            Some(value)
        }
        Ok(None) if key == TOKEN_KEY => session_token().or_else(|| fallback.and_then(read_secret)),
        Ok(None) => None,
        Err(e) => {
            log::warn!(target: "app", "auth: keyring unavailable: {e}");
            if key == TOKEN_KEY {
                session_token().or_else(|| fallback.and_then(read_secret))
            } else {
                None
            }
        }
    }
}

/// Whether the keyring answers at all. Probed once: the UI warns at startup when
/// the token lives outside the keyring.
pub fn keyring_ok() -> bool {
    static OK: OnceLock<bool> = OnceLock::new();
    *OK.get_or_init(|| {
        let probed = entry(TOKEN_KEY).and_then(|entry| match entry.get_password() {
            Ok(_) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(format!("keyring get: {e}")),
        });
        match probed {
            Ok(()) => true,
            Err(e) => {
                log::warn!(target: "app", "auth: keyring unavailable: {e}");
                false
            }
        }
    })
}

/// Token used by the git/github modules. The webview only ever sees whether it
/// exists (`auth_has_token`); the value stays in Rust.
pub fn token() -> Result<String, String> {
    read(TOKEN_KEY).ok_or_else(|| "not authenticated".into())
}

/// Stores a token obtained outside the webview (the device flow). Returns
/// whether the keyring kept it; `false` means it went to the protected file,
/// or only this session when even that fails.
pub async fn store_token(token: String) -> Result<bool, String> {
    let persisted = tokio::task::spawn_blocking(move || store(TOKEN_KEY, &token))
        .await
        .map_err(|e| format!("keyring task: {e}"))?;
    if persisted {
        // Never the token itself: only that one is stored.
        log::info!(target: "app", "auth: token stored");
    }
    Ok(persisted)
}

/// Stores the token the user pasted. The only write the webview can make.
#[tauri::command]
pub async fn auth_set_pat(token: String) -> Result<bool, String> {
    let token = token.trim().to_string();
    if token.is_empty() {
        return Err("empty token".into());
    }
    store_token(token).await
}

/// Whether a token is available: keyring, protected file, or this session's copy.
#[tauri::command]
pub async fn auth_has_token() -> Result<bool, String> {
    tokio::task::spawn_blocking(|| Ok::<bool, String>(read(TOKEN_KEY).is_some()))
        .await
        .map_err(|e| format!("keyring task: {e}"))?
}

/// Answers once per process whether the token survives a restart.
#[tauri::command]
pub async fn auth_keyring_ok() -> Result<bool, String> {
    // Keyring probes are blocking: keep them off the main thread.
    tokio::task::spawn_blocking(keyring_ok)
        .await
        .map_err(|e| format!("keyring task: {e}"))
}

#[tauri::command]
pub async fn auth_clear() -> Result<(), String> {
    tokio::task::spawn_blocking(|| {
        let cleared = entry(TOKEN_KEY).and_then(|entry| match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(format!("keyring clear: {e}")),
        });
        // Logging out must work even when the keyring never held the token.
        if let Err(e) = cleared {
            log::warn!(target: "app", "auth: {e}");
        }
        set_session_token(None);
        if let Some(path) = fallback_path() {
            clear_secret(&path);
        }
    })
    .await
    .map_err(|e| format!("keyring task: {e}"))?;
    log::info!(target: "app", "auth: token cleared");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The session copy and the keyring are process-wide: tests that touch the
    /// token must not race each other.
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("gitpad-auth-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    // Uses a throwaway key: writing TOKEN_KEY here would clobber the real token.
    #[test]
    fn keyring_roundtrip() {
        let key = "gitpad_test_key";
        // Headless runners have no Secret Service: nothing to test there.
        if !store(key, "value") {
            eprintln!("keyring backend unavailable, skipping");
            return;
        }
        assert_eq!(read(key), Some("value".into()));
        entry(key).unwrap().delete_credential().unwrap();
        assert_eq!(read(key), None);
    }

    #[test]
    fn bus_address_points_at_the_runtime_socket() {
        assert_eq!(bus_address(None), None);
        let dir = std::env::temp_dir().join(format!("gitpad-bus-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(bus_address(Some(dir.as_os_str())), None);
        std::fs::write(dir.join("bus"), "").unwrap();
        assert_eq!(
            bus_address(Some(dir.as_os_str())),
            Some(format!("unix:path={}/bus", dir.display()))
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_failing_keyring_falls_back_to_the_session_copy() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_session_token(None);
        // A broken keyring must not make the login fail.
        assert!(!store_with(
            TOKEN_KEY,
            "secret",
            || Err("no service".into()),
            None
        ));
        assert_eq!(session_token(), Some("secret".into()));
        assert_eq!(
            read_with(TOKEN_KEY, || Err("no service".into()), None),
            Some("secret".into())
        );
        // The copy belongs to the token only.
        assert_eq!(read_with("other", || Err("no service".into()), None), None);
        // A keyring that works again wins over the stale copy.
        assert!(store_with(TOKEN_KEY, "fresh", || Ok(()), None));
        assert_eq!(session_token(), None);
        assert_eq!(
            read_with(TOKEN_KEY, || Ok(Some("fresh".into())), None),
            Some("fresh".into())
        );
        set_session_token(None);
    }

    #[test]
    fn failing_keyring_keeps_the_token_in_a_protected_file() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_session_token(None);
        let dir = temp("file");
        let path = dir.join(TOKEN_FILE);

        assert!(!store_with(
            TOKEN_KEY,
            "secret",
            || Err("no service".into()),
            Some(&path)
        ));
        assert_eq!(read_secret(&path), Some("secret".into()));
        assert_eq!(session_token(), Some("secret".into()));
        // A restart loses the session copy: the file is what remains.
        set_session_token(None);
        assert_eq!(
            read_with(TOKEN_KEY, || Err("no service".into()), Some(&path)),
            Some("secret".into())
        );
        assert_eq!(
            read_with(TOKEN_KEY, || Ok(None), Some(&path)),
            Some("secret".into())
        );

        clear_secret(&path);
        set_session_token(None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_working_keyring_removes_the_fallback_file() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_session_token(None);
        let dir = temp("removed");
        let path = dir.join(TOKEN_FILE);
        write_secret(&path, "old").unwrap();

        assert!(store_with(TOKEN_KEY, "fresh", || Ok(()), Some(&path)));
        assert!(!path.exists());
        assert_eq!(session_token(), None);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn credential_file_is_readable_only_by_its_owner() {
        use std::os::unix::fs::PermissionsExt;

        let dir = temp("mode");
        let path = dir.join(TOKEN_FILE);
        write_secret(&path, "value").unwrap();

        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        assert_eq!(read_secret(&path), Some("value".into()));
        clear_secret(&path);
        assert_eq!(read_secret(&path), None);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
