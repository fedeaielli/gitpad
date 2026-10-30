pub const SERVICE: &str = "com.gitpad.app";
pub const TOKEN_KEY: &str = "github_token";

fn entry(key: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, key).map_err(|e| format!("keyring entry: {e}"))
}

#[tauri::command]
pub fn auth_store(key: String, value: String) -> Result<(), String> {
    entry(&key)?
        .set_password(&value)
        .map_err(|e| format!("keyring store: {e}"))
}

#[tauri::command]
pub fn auth_get(key: String) -> Result<Option<String>, String> {
    match entry(&key)?.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("keyring get: {e}")),
    }
}

#[tauri::command]
pub fn auth_clear() -> Result<(), String> {
    match entry(TOKEN_KEY)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("keyring clear: {e}")),
    }
}

/// Token used by the git/github modules. Never leaves the Rust side.
pub fn token() -> Result<String, String> {
    match entry(TOKEN_KEY)?.get_password() {
        Ok(value) if !value.is_empty() => Ok(value),
        Ok(_) | Err(keyring::Error::NoEntry) => Err("not authenticated".into()),
        Err(e) => Err(format!("keyring get: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyring_roundtrip() {
        let key = "gitpad_test_key";
        auth_store(key.into(), "value".into()).unwrap();
        assert_eq!(auth_get(key.into()).unwrap(), Some("value".into()));
        entry(key).unwrap().delete_credential().unwrap();
        assert_eq!(auth_get(key.into()).unwrap(), None);
    }
}
