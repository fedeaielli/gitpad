mod auth;
mod fs;
mod git;
mod github;
mod search;

use reqwest::Client;
use tauri::Manager;

pub struct AppState {
    pub http: Client,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let http = Client::builder().user_agent("GitPad").build()?;
            app.manage(AppState { http });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            auth::auth_store,
            auth::auth_get,
            auth::auth_clear,
            fs::fs_read_file,
            fs::fs_write_file,
            fs::fs_create_dir,
            fs::fs_delete,
            fs::fs_rename,
            fs::fs_list_dir,
            git::git_clone,
            git::git_pull,
            git::git_push,
            git::git_status,
            git::git_conflicts,
            git::git_resolve_conflict,
            github::github_validate_token,
            github::github_get_user,
            github::github_init_repo,
            github::github_device_flow_start,
            github::github_device_flow_poll,
            search::search_global,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let app = window.app_handle().clone();
                tauri::async_runtime::spawn(async move {
                    if let Ok(path) = git::repo_dir(&app) {
                        let token = auth::token().ok();
                        let _ = tokio::time::timeout(
                            std::time::Duration::from_secs(3),
                            tokio::task::spawn_blocking(move || {
                                git::push_blocking(&path, None, token)
                            }),
                        )
                        .await;
                    }
                    app.exit(0);
                });
            }
        })
        .run(tauri::generate_context!())
        .expect("error while building tauri application");
}
