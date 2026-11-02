pub const SERVICE: &str = "com.gitpad.app";
pub const TOKEN_KEY: &str = "github_token";

fn entry(key: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, key).map_err(|e| format!("keyring entry: {e}"))
}

/// Writes one secret. Private on purpose: the webview can neither read a secret
/// nor pick the key it lands on, so only `auth_set_pat` and the device flow
/// touch the keyring.
fn store(key: &str, value: &str) -> Result<(), String> {
    entry(key)?
        .set_password(value)
        .map_err(|e| format!("keyring store: {e}"))
}

fn read(key: &str) -> Result<Option<String>, String> {
    match entry(key)?.get_password() {
        Ok(value) if !value.is_empty() => Ok(Some(value)),
        Ok(_) | Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("keyring get: {e}")),
    }
}

/// Token used by the git/github modules. The webview only ever sees whether it
/// exists (`auth_has_token`); the value stays in Rust.
pub fn token() -> Result<String, String> {
    read(TOKEN_KEY)?.ok_or_else(|| "not authenticated".into())
}

/// Stores a token obtained outside the webview (the device flow).
pub async fn store_token(token: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || store(TOKEN_KEY, &token))
        .await
        .map_err(|e| format!("keyring task: {e}"))?
}

/// Stores the token the user pasted. The only write the webview can make.
#[tauri::command]
pub async fn auth_set_pat(token: String) -> Result<(), String> {
    let token = token.trim().to_string();
    if token.is_empty() {
        return Err("empty token".into());
    }
    store_token(token).await
}

/// Whether a token is stored. Real keyring failures are errors, not "absent".
#[tauri::command]
pub async fn auth_has_token() -> Result<bool, String> {
    tokio::task::spawn_blocking(|| read(TOKEN_KEY).map(|token| token.is_some()))
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

#[cfg(test)]
mod tests {
    use super::*;

    // Uses a throwaway key: writing TOKEN_KEY here would clobber the real token.
    #[test]
    fn keyring_roundtrip() {
        let key = "gitpad_test_key";
        store(key, "value").unwrap();
        assert_eq!(read(key).unwrap(), Some("value".into()));
        entry(key).unwrap().delete_credential().unwrap();
        assert_eq!(read(key).unwrap(), None);
    }
}
