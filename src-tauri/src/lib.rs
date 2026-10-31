mod auth;
mod config;
mod fs;
mod git;
mod github;
mod search;

use nix::sys::signal::{pthread_sigmask, SigSet, SigmaskHow, Signal};
use reqwest::Client;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{Emitter, Manager};

pub struct AppState {
    pub http: Client,
    /// Cached clone path: every command resolves the same repository.
    pub repo_path: std::sync::Mutex<Option<std::path::PathBuf>>,
}

static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);
/// Set by the frontend as soon as it handles `app-closing`; the watchdog below
/// only fires when the webview is dead and never acknowledges.
static CLOSE_ACK: AtomicBool = AtomicBool::new(false);

/// Push the last local commits (short timeout, offline is fine) and exit.
async fn shutdown(app: tauri::AppHandle) {
    if SHUTTING_DOWN.swap(true, Ordering::SeqCst) {
        return;
    }
    if let Ok(path) = git::repo_dir(&app) {
        let token = auth::token().ok();
        // Push only: fetching at exit just delays closing, and a rejected push
        // (remote moved) leaves the local commit for the next sync.
        let _ = tokio::time::timeout(
            Duration::from_secs(3),
            tokio::task::spawn_blocking(move || git::push_only_blocking(&path, None, token)),
        )
        .await;
    }
    app.exit(0);
}

/// SIGTERM: commit what is on disk and push before dying. `push_only_blocking`
/// commits pending changes first, so this is the whole shutdown path.
fn terminate(app: tauri::AppHandle) {
    if let Ok(path) = git::repo_dir(&app) {
        let token = auth::token().ok();
        // 5 s cap: a hanging push must not keep the process alive. No tokio
        // here: this thread is outside the async runtime.
        let (done, wait) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = done.send(git::push_only_blocking(&path, None, token));
        });
        let _ = wait.recv_timeout(Duration::from_secs(5));
    }
    app.exit(0);
}

/// SIGTERM is blocked in every thread so a dedicated thread can wait for it:
/// signal handlers may only touch async-signal-safe code, git2 is not.
fn block_sigterm() -> nix::Result<SigSet> {
    let mut set = SigSet::empty();
    set.add(Signal::SIGTERM);
    pthread_sigmask(SigmaskHow::SIG_BLOCK, Some(&set), None)?;
    Ok(set)
}

/// Called by the frontend once pending edits are flushed.
#[tauri::command]
async fn app_exit(app: tauri::AppHandle) {
    shutdown(app).await;
}

/// First thing the frontend does on `app-closing`: proves the webview is alive.
#[tauri::command]
fn app_closing_ack() {
    CLOSE_ACK.store(true, Ordering::SeqCst);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Must happen before Tauri spawns threads: they inherit the mask.
    let sigterm = block_sigterm().ok();
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            let http = Client::builder()
                .user_agent("GitPad")
                .timeout(Duration::from_secs(15))
                .build()?;
            app.manage(AppState {
                http,
                repo_path: std::sync::Mutex::new(None),
            });
            // Warm the clone path once, off the main thread, so no command
            // pays for the directory scan on the hot path.
            let prime = app.handle().clone();
            tauri::async_runtime::spawn_blocking(move || git::prime_repo_path(&prime));
            if let Some(set) = sigterm {
                let term = app.handle().clone();
                std::thread::spawn(move || {
                    if set.wait().is_ok() {
                        terminate(term);
                    }
                });
            }
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
            fs::reveal_in_filemanager,
            fs::fs_import,
            git::git_clone,
            git::git_pull,
            git::git_push,
            git::git_discard_file,
            git::git_log,
            git::git_commit_files,
            git::git_restore_file,
            git::git_revert_commit,
            git::git_file_log,
            git::git_file_diff,
            git::git_status,
            git::git_conflicts,
            git::git_resolve_conflict,
            git::git_conflict_versions,
            github::github_validate_token,
            github::github_get_user,
            github::github_device_flow_available,
            github::github_init_repo,
            github::github_device_flow_start,
            github::github_device_flow_poll,
            search::search_global,
            config::config_get,
            config::config_save,
            app_exit,
            app_closing_ack,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                CLOSE_ACK.store(false, Ordering::SeqCst);
                let _ = window.emit("app-closing", ());
                // Force-close only if the webview never answers (crash/hang). The
                // frontend acks immediately, so the unsaved-changes dialog can stay
                // open as long as the user needs without racing this timer.
                let app = window.app_handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_secs(5));
                    if !CLOSE_ACK.load(Ordering::SeqCst) {
                        app.exit(0);
                    }
                });
            }
        })
        .run(tauri::generate_context!());

    if let Err(err) = result {
        eprintln!("gitpad: {err}");
        std::process::exit(1);
    }
}
