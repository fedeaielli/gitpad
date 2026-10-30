use crate::{auth, AppState};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

const API: &str = "https://api.github.com";
const OAUTH: &str = "https://github.com/login";
const DEFAULT_REPO_NAME: &str = "gitpad_notes";
/// OAuth App client id (public). Empty until the app is registered.
const GITHUB_CLIENT_ID: &str = "";
const DEVICE_CODE_TTL_SECS: u64 = 900;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserInfo {
    pub login: String,
    pub avatar_url: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoInfo {
    pub name: String,
    pub full_name: String,
    pub clone_url: String,
    pub private: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCode {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Deserialize)]
struct ApiError {
    message: String,
}

#[derive(Deserialize)]
struct DevicePoll {
    access_token: Option<String>,
    error: Option<String>,
}

async fn user_from_token(http: &reqwest::Client, token: &str) -> Result<UserInfo, String> {
    let res = http
        .get(format!("{API}/user"))
        .bearer_auth(token)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("github: {e}"))?;
    parse(res).await
}

async fn parse<T: DeserializeOwned>(res: reqwest::Response) -> Result<T, String> {
    let status = res.status();
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        return Err(match serde_json::from_str::<ApiError>(&body) {
            Ok(e) => format!("github {status}: {}", e.message),
            Err(_) => format!("github {status}"),
        });
    }
    res.json()
        .await
        .map_err(|e| format!("github response: {e}"))
}

#[tauri::command]
pub async fn github_validate_token(
    state: State<'_, AppState>,
    token: String,
) -> Result<UserInfo, String> {
    user_from_token(&state.http, &token).await
}

#[tauri::command]
pub async fn github_get_user(state: State<'_, AppState>) -> Result<UserInfo, String> {
    let token = auth::token()?;
    user_from_token(&state.http, &token).await
}

#[tauri::command]
pub async fn github_init_repo(state: State<'_, AppState>) -> Result<RepoInfo, String> {
    let token = auth::token()?;
    let user = user_from_token(&state.http, &token).await?;
    let res = state
        .http
        .get(format!("{API}/repos/{}/{DEFAULT_REPO_NAME}", user.login))
        .bearer_auth(&token)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("github: {e}"))?;
    if res.status() != reqwest::StatusCode::NOT_FOUND {
        return parse(res).await;
    }
    let body = serde_json::json!({
        "name": DEFAULT_REPO_NAME,
        "private": true,
        "auto_init": true,
    });
    let res = state
        .http
        .post(format!("{API}/user/repos"))
        .bearer_auth(&token)
        .header("Accept", "application/vnd.github+json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("github: {e}"))?;
    parse(res).await
}

#[tauri::command]
pub async fn github_device_flow_start(
    state: State<'_, AppState>,
) -> Result<DeviceCode, String> {
    if GITHUB_CLIENT_ID.is_empty() {
        return Err("device flow not configured: set GITHUB_CLIENT_ID".into());
    }
    let res = state
        .http
        .post(format!("{OAUTH}/device/code"))
        .header("Accept", "application/json")
        .form(&[("client_id", GITHUB_CLIENT_ID), ("scope", "repo")])
        .send()
        .await
        .map_err(|e| format!("github: {e}"))?;
    parse(res).await
}

#[tauri::command]
pub fn github_device_flow_poll(
    app: AppHandle,
    state: State<'_, AppState>,
    device_code: String,
) -> Result<(), String> {
    if GITHUB_CLIENT_ID.is_empty() {
        return Err("device flow not configured: set GITHUB_CLIENT_ID".into());
    }
    let http = state.http.clone();
    tauri::async_runtime::spawn(async move {
        let mut interval = std::time::Duration::from_secs(5);
        let deadline =
            std::time::Instant::now() + std::time::Duration::from_secs(DEVICE_CODE_TTL_SECS);
        loop {
            if std::time::Instant::now() > deadline {
                let _ = app.emit("auth-error", "device code expired");
                return;
            }
            tokio::time::sleep(interval).await;
            let res = http
                .post(format!("{OAUTH}/oauth/access_token"))
                .header("Accept", "application/json")
                .form(&[
                    ("client_id", GITHUB_CLIENT_ID),
                    ("device_code", device_code.as_str()),
                    ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ])
                .send()
                .await;
            let poll: DevicePoll = match res {
                Ok(res) => match res.json().await {
                    Ok(poll) => poll,
                    Err(e) => {
                        let _ = app.emit("auth-error", format!("github response: {e}"));
                        return;
                    }
                },
                Err(e) => {
                    let _ = app.emit("auth-error", format!("github: {e}"));
                    return;
                }
            };
            if let Some(token) = poll.access_token {
                match auth::auth_store(auth::TOKEN_KEY.into(), token) {
                    Ok(()) => {
                        let _ = app.emit("auth-success", ());
                    }
                    Err(e) => {
                        let _ = app.emit("auth-error", e);
                    }
                }
                return;
            }
            match poll.error.as_deref() {
                Some("authorization_pending") => {}
                Some("slow_down") => interval += std::time::Duration::from_secs(5),
                Some("expired_token") => {
                    let _ = app.emit("auth-error", "device code expired");
                    return;
                }
                Some("access_denied") => {
                    let _ = app.emit("auth-error", "access denied");
                    return;
                }
                Some(other) => {
                    let _ = app.emit("auth-error", other);
                    return;
                }
                None => {
                    let _ = app.emit("auth-error", "unexpected github response");
                    return;
                }
            }
        }
    });
    Ok(())
}
