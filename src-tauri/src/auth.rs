pub const SERVICE: &str = "com.gitpad.app";
pub const TOKEN_KEY: &str = "github_token";

fn entry(key: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, key).map_err(|e| format!("keyring entry: {e}"))
}

#[tauri::command]
pub async fn auth_store(key: String, value: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        entry(&key)?
            .set_password(&value)
            .map_err(|e| format!("keyring store: {e}"))
    })
    .await
    .map_err(|e| format!("keyring task: {e}"))?
}

/// Reads the stored token without ever handing it to the webview.
fn read_token() -> Result<Option<String>, String> {
    match entry(TOKEN_KEY)?.get_password() {
        Ok(value) if !value.is_empty() => Ok(Some(value)),
        Ok(_) | Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("keyring get: {e}")),
    }
}

/// Whether a token is stored. Real keyring failures are errors, not "absent".
#[tauri::command]
pub async fn auth_has_token() -> Result<bool, String> {
    tokio::task::spawn_blocking(|| read_token().map(|token| token.is_some()))
        .await
        .map_err(|e| format!("keyring task: {e}"))?
}

#[tauri::command]
pub async fn auth_get(key: String) -> Result<Option<String>, String> {
    tokio::task::spawn_blocking(move || match entry(&key)?.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("keyring get: {e}")),
    })
    .await
    .map_err(|e| format!("keyring task: {e}"))?
}

#[tauri::command]
pub async fn auth_clear() -> Result<(), String> {
    tokio::task::spawn_blocking(|| match entry(TOKEN_KEY)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("keyring clear: {e}")),
    })
    .await
    .map_err(|e| format!("keyring task: {e}"))?
}

/// Token used by the git/github modules. The webview only ever sees whether it
/// exists (`auth_has_token`); the value stays in Rust.
pub fn token() -> Result<String, String> {
    read_token()?.ok_or_else(|| "not authenticated".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn keyring_roundtrip() {
        let key = "gitpad_test_key";
        auth_store(key.into(), "value".into()).await.unwrap();
        assert_eq!(auth_get(key.into()).await.unwrap(), Some("value".into()));
        entry(key).unwrap().delete_credential().unwrap();
        assert_eq!(auth_get(key.into()).await.unwrap(), None);
    }
}
