use std::sync::{Mutex, OnceLock};
use tauri_plugin_log::log;

pub const SERVICE: &str = "com.gitpad.app";
pub const TOKEN_KEY: &str = "github_token";

/// The token when the keyring cannot hold it: a session-only copy, so a machine
/// whose Secret Service is missing or locked can still sign in.
static SESSION_TOKEN: Mutex<Option<String>> = Mutex::new(None);

fn session_token() -> Option<String> {
    SESSION_TOKEN.lock().ok().and_then(|held| held.clone())
}

fn set_session_token(token: Option<&str>) {
    if let Ok(mut held) = SESSION_TOKEN.lock() {
        *held = token.map(str::to_string);
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
    store_with(key, value, || keyring_write(key, value))
}

fn store_with(
    key: &str,
    value: &str,
    write: impl FnOnce() -> Result<(), String>,
) -> bool {
    match write() {
        Ok(()) => {
            if key == TOKEN_KEY {
                set_session_token(None);
            }
            true
        }
        Err(e) => {
            // A missing or locked Secret Service is not fatal: keep it for this
            // session and let the UI say so.
            log::warn!(target: "app", "auth: token kept in memory only: {e}");
            if key == TOKEN_KEY {
                set_session_token(Some(value));
            }
            false
        }
    }
}

fn read(key: &str) -> Option<String> {
    read_with(key, || keyring_read(key))
}

fn read_with(key: &str, read: impl FnOnce() -> Result<Option<String>, String>) -> Option<String> {
    match read() {
        Ok(Some(value)) => Some(value),
        Ok(None) if key == TOKEN_KEY => session_token(),
        Ok(None) => None,
        Err(e) => {
            log::warn!(target: "app", "auth: keyring unavailable: {e}");
            (key == TOKEN_KEY).then(session_token).flatten()
        }
    }
}

/// Whether the keyring answers at all. Probed once: the UI warns at startup when
/// the token can only live for this session.
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
/// whether the keyring kept it; `false` means it lives for this session only.
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

/// Whether a token is available: from the keyring, or from this session's copy.
#[tauri::command]
pub async fn auth_has_token() -> Result<bool, String> {
    tokio::task::spawn_blocking(|| Ok::<bool, String>(read(TOKEN_KEY).is_some()))
        .await
        .map_err(|e| format!("keyring task: {e}"))?
}

/// Answers once per process whether the token survives a restart.
#[tauri::command]
pub fn auth_keyring_ok() -> bool {
    keyring_ok()
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
    })
    .await
    .map_err(|e| format!("keyring task: {e}"))?;
    log::info!(target: "app", "auth: token cleared");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
        set_session_token(None);
        // A broken keyring must not make the login fail.
        assert!(!store_with(TOKEN_KEY, "secret", || Err("no service".into())));
        assert_eq!(session_token(), Some("secret".into()));
        assert_eq!(
            read_with(TOKEN_KEY, || Err("no service".into())),
            Some("secret".into())
        );
        // The copy belongs to the token only.
        assert_eq!(read_with("other", || Err("no service".into())), None);
        // A keyring that works again wins over the stale copy.
        assert!(store_with(TOKEN_KEY, "fresh", || Ok(())));
        assert_eq!(session_token(), None);
        assert_eq!(
            read_with(TOKEN_KEY, || Ok(Some("fresh".into()))),
            Some("fresh".into())
        );
        set_session_token(None);
    }
}
