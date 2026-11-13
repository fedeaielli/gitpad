mod auth;
mod config;
mod fs;
mod git;
mod github;
mod search;
mod watcher;

use tauri::AppHandle;
use tauri_plugin_log::log;

#[cfg(test)]
pub(crate) mod test_util {
    use std::sync::{mpsc, Arc, RwLock};
    use std::time::Duration;

    /// Fails unless `op` blocks on the gate and finishes once it is released.
    pub(crate) fn assert_waits_for_gate<T, F>(op: F)
    where
        F: FnOnce(Arc<RwLock<()>>) -> T + Send + 'static,
        T: Send + 'static,
    {
        let gate = Arc::new(RwLock::new(()));
        let held = gate.write().unwrap();
        let (tx, rx) = mpsc::channel();
        let task_gate = gate.clone();
        let task = std::thread::spawn(move || {
            let _ = tx.send(op(task_gate));
        });
        assert!(
            rx.recv_timeout(Duration::from_millis(200)).is_err(),
            "operation did not wait for the gate"
        );
        drop(held);
        assert!(
            rx.recv_timeout(Duration::from_secs(10)).is_ok(),
            "operation did not finish after the gate was released"
        );
        task.join().unwrap();
        assert!(gate.try_write().is_ok(), "operation left the gate locked");
    }
}

#[cfg(unix)]
use nix::sys::signal::{pthread_sigmask, SigSet, SigmaskHow, Signal};
use reqwest::Client;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
use tauri::{Emitter, Manager};

pub struct AppState {
    pub http: Client,
    /// Cached clone path: every command resolves the same repository.
    pub repo_path: std::sync::Mutex<Option<std::path::PathBuf>>,
    /// Guards the worktree against git2 rewrites (pull, reset, merge). Take it
    /// only inside `spawn_blocking`, never across an `await` on the async runtime.
    pub repo_gate: std::sync::Arc<std::sync::RwLock<()>>,
    /// Serializes autosave commits against each other. Take it only after
    /// `repo_gate.read()` and only inside `spawn_blocking`, never across an
    /// `await`: the order is always repo_gate.read() -> commit_gate.lock().
    pub commit_gate: std::sync::Arc<std::sync::Mutex<()>>,
    /// The watch on the active repository; `None` until one is started.
    pub watcher: std::sync::Mutex<Option<crate::watcher::Handle>>,
}

static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);
/// Bumped on every `CloseRequested`: a watchdog may only close the request it
/// was started for, not a later one.
static CLOSE_GEN: AtomicU64 = AtomicU64::new(0);
/// Set by the frontend as soon as it handles `app-closing`; the watchdog below
/// only fires when the webview is dead and never acknowledges.
static CLOSE_ACK: AtomicBool = AtomicBool::new(false);

/// A watchdog forces the exit only for its own, still-unacknowledged close.
/// A late ack from a previous close is theoretically possible, but it would
/// have to arrive after an entire close-and-reopen cycle: accepted.
fn watchdog_should_exit(captured: u64, current: u64, acked: bool) -> bool {
    captured == current && !acked
}

/// How long a closing path waits for the worktree gate before skipping the
/// final commit and push. One second leaves room for the commit and push
/// inside the 3 s (shutdown) and 5 s (terminate) caps.
const GATE_WAIT: Duration = Duration::from_millis(1000);

/// Push the last local commits (short timeout, offline is fine) and exit.
async fn shutdown(app: tauri::AppHandle) {
    if SHUTTING_DOWN.swap(true, Ordering::SeqCst) {
        return;
    }
    log::info!(target: "app", "shutdown: flushing the last commit");
    if let Ok(path) = git::repo_dir(&app) {
        let gate = app.state::<AppState>().repo_gate.clone();
        // The worktree gate is taken with a deadline: if a pull or an import
        // holds it, skip the closing commit instead of burning the timeout.
        // Push only: fetching at exit just delays closing, and a rejected push
        // (remote moved) leaves the local commit for the next sync.
        let flushed = tokio::time::timeout(
            Duration::from_secs(3),
            tokio::task::spawn_blocking(move || {
                // Keyring reads are blocking: keep them off the async runtime.
                let token = auth::token().ok();
                git::push_only_blocking(&path, None, token, git::GateWait::Until(&gate, GATE_WAIT))
            }),
        )
        .await;
        if flushed.is_err() {
            log::warn!(target: "app", "shutdown: the last push timed out");
        }
    }
    log::info!(target: "app", "shutdown: bye");
    app.exit(0);
}

/// SIGTERM: commit what is on disk and push before dying. `push_only_blocking`
/// commits pending changes first, so this is the whole shutdown path.
/// Unix-only: Windows has no SIGTERM and relies on `CloseRequested`.
#[cfg(unix)]
fn terminate(app: tauri::AppHandle) {
    log::info!(target: "app", "terminate: flushing the last commit");
    if let Ok(path) = git::repo_dir(&app) {
        let gate = app.state::<AppState>().repo_gate.clone();
        // Same deadline as shutdown: a held gate means no closing commit.
        let token = auth::token().ok();
        // 5 s cap: a hanging push must not keep the process alive. No tokio
        // here: this thread is outside the async runtime.
        let (done, wait) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = done.send(git::push_only_blocking(
                &path,
                None,
                token,
                git::GateWait::Until(&gate, GATE_WAIT),
            ));
        });
        if wait.recv_timeout(Duration::from_secs(5)).is_err() {
            log::warn!(target: "app", "terminate: the last push timed out");
        }
    }
    app.exit(0);
}

/// SIGTERM is blocked in every thread so a dedicated thread can wait for it:
/// signal handlers may only touch async-signal-safe code, git2 is not.
#[cfg(unix)]
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
/// A hang after the ack no longer trips the watchdog; accepted, because the
/// flush only writes locally and the exit push has its own timeout.
#[tauri::command]
fn app_closing_ack() {
    CLOSE_ACK.store(true, Ordering::SeqCst);
}

/// The webview reports what the user was shown, so one log carries both sides.
/// The category becomes the log target: the logs tab filters on it.
#[tauri::command]
fn app_log(level: String, category: String, message: String) {
    let target = log_target(&category);
    match level_of(&level) {
        log::Level::Error => log::error!(target: target, "{message}"),
        log::Level::Warn => log::warn!(target: target, "{message}"),
        _ => log::info!(target: target, "{message}"),
    }
}

/// Unknown categories fall back to the editor, the busiest one.
fn log_target(category: &str) -> &'static str {
    match category {
        "git" => "git",
        "app" => "app",
        _ => "editor",
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogFile {
    name: String,
    /// The file the running session appends to.
    current: bool,
    /// Milliseconds since the epoch.
    modified: u64,
    size: u64,
}

/// Only the files the log plugin wrote, inside the log directory: the name
/// comes from the webview, so it must not be able to walk out.
fn safe_log_name(name: &str) -> bool {
    name.starts_with("gitpad")
        && name.ends_with(".log")
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains("..")
}

fn log_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path().app_log_dir().map_err(|e| format!("log dir: {e}"))
}

/// Newest first. Sessions are matched inside the files by the frontend.
#[tauri::command]
fn logs_list(app: AppHandle) -> Result<Vec<LogFile>, String> {
    let dir = log_dir(&app)?;
    let mut files: Vec<LogFile> = std::fs::read_dir(&dir)
        .map_err(|e| format!("read {}: {e}", dir.display()))?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if !safe_log_name(&name) {
                return None;
            }
            let meta = entry.metadata().ok()?;
            Some(LogFile {
                current: name == "gitpad.log",
                modified: meta
                    .modified()
                    .ok()?
                    .duration_since(std::time::UNIX_EPOCH)
                    .ok()?
                    .as_millis() as u64,
                size: meta.len(),
                name,
            })
        })
        .collect();
    files.sort_by(|a, b| b.modified.cmp(&a.modified));
    Ok(files)
}

/// Raw text: parsing and filtering stay in one place, in the frontend.
#[tauri::command]
fn logs_read(app: AppHandle, name: String) -> Result<String, String> {
    if !safe_log_name(&name) {
        return Err(format!("not a log file: {name}"));
    }
    let path = log_dir(&app)?.join(&name);
    let meta = std::fs::metadata(&path).map_err(|e| format!("{name}: {e}"))?;
    // A rotated file can be large: cap the read so the webview stays responsive.
    const MAX_READ: u64 = 8 * 1024 * 1024;
    let bytes = if meta.len() > MAX_READ {
        let file = std::fs::File::open(&path).map_err(|e| format!("{name}: {e}"))?;
        let skip = meta.len() - MAX_READ;
        use std::io::{Read, Seek, SeekFrom};
        let mut file = file;
        file.seek(SeekFrom::Start(skip))
            .map_err(|e| format!("{name}: {e}"))?;
        let mut buf = Vec::with_capacity(MAX_READ as usize);
        file.take(MAX_READ)
            .read_to_end(&mut buf)
            .map_err(|e| format!("{name}: {e}"))?;
        buf
    } else {
        std::fs::read(&path).map_err(|e| format!("{name}: {e}"))?
    };
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn level_of(name: &str) -> log::Level {
    match name {
        "error" => log::Level::Error,
        "warn" => log::Level::Warn,
        _ => log::Level::Info,
    }
}

/// The URL reaches a process argv: never let anything but a web page through.
fn openable(url: &str) -> bool {
    url.starts_with("https://") || url.starts_with("http://")
}

#[tauri::command]
async fn open_external(url: String) -> Result<(), String> {
    if !openable(&url) {
        return Err("only http(s) links can be opened".into());
    }
    // A cold browser start keeps the opener alive for many seconds. Sync commands
    // run on the main thread, where that wait froze the window before it could
    // paint the device code, so the wait belongs on the blocking pool.
    tauri::async_runtime::spawn_blocking(move || {
        tauri_plugin_opener::open_url(&url, None::<&str>).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("open task: {e}"))?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Before anything reaches for the keyring.
    auth::ensure_session_bus();
    // Must happen before Tauri spawns threads: they inherit the mask.
    #[cfg(unix)]
    let sigterm = block_sigterm().ok();
    let result = tauri::Builder::default()
        // Two instances on the same repository would autosave, commit and push
        // twice: a second launch only raises the window that is already open.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            // Archive on rotation instead of deleting: the logs tab reads the
            // previous sessions back. 5 files x 5 MB caps the folder at ~25 MB.
            tauri_plugin_log::Builder::new()
                .targets([
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir {
                        file_name: Some("gitpad".into()),
                    }),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
                ])
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepSome(5))
                .max_file_size(5 * 1024 * 1024)
                .level(tauri_plugin_log::log::LevelFilter::Info)
                .build(),
        )
        .setup(move |app| {
            log::info!(target: "app", "gitpad {} starting", env!("CARGO_PKG_VERSION"));
            // The token file is the only fallback when the keyring is down.
            match app.path().app_data_dir() {
                Ok(dir) => auth::use_fallback_dir(dir),
                Err(e) => log::warn!(
                    target: "app",
                    "auth: no app data dir ({e}); the token will not survive a restart"
                ),
            }
            let http = Client::builder()
                .user_agent("GitPad")
                .timeout(Duration::from_secs(15))
                .build()?;
            app.manage(AppState {
                http,
                repo_path: std::sync::Mutex::new(None),
                repo_gate: std::sync::Arc::new(std::sync::RwLock::new(())),
                commit_gate: std::sync::Arc::new(std::sync::Mutex::new(())),
                watcher: std::sync::Mutex::new(None),
            });
            // Warm the clone path once, off the main thread, so no command
            // pays for the directory scan on the hot path. The watch starts on
            // the same task: a missing repository is fine.
            let prime = app.handle().clone();
            tauri::async_runtime::spawn_blocking(move || {
                git::prime_repo_path(&prime);
                if let Ok(root) = git::repo_dir(&prime) {
                    if let Err(e) = watcher::ensure(&prime, &root) {
                        log::warn!(target: "app", "watcher: {e}");
                    }
                }
            });
            // Unix only: on Windows `CloseRequested` is the whole close path.
            #[cfg(unix)]
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
            auth::auth_set_pat,
            auth::auth_has_token,
            auth::auth_keyring_ok,
            auth::auth_clear,
            fs::fs_read_file,
            fs::fs_read_binary,
            fs::fs_write_file,
            fs::fs_create_dir,
            fs::fs_delete,
            fs::fs_rename,
            fs::fs_list_dir,
            fs::fs_repo_name,
            fs::fs_list_fonts,
            fs::reveal_in_filemanager,
            fs::fs_import,
            git::git_clone,
            git::git_move_repo,
            git::git_import_repo,
            git::git_use_local,
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
            git::git_resolve_conflict,
            git::git_conflict_versions,
            github::github_validate_token,
            github::github_get_user,
            github::github_device_flow_available,
            github::github_init_repo,
            github::github_device_flow_start,
            github::github_device_flow_poll,
            github::github_device_flow_cancel,
            search::search_global,
            config::config_get,
            config::config_save,
            app_exit,
            app_closing_ack,
            app_log,
            open_external,
            logs_list,
            logs_read,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let generation = CLOSE_GEN.fetch_add(1, Ordering::SeqCst) + 1;
                CLOSE_ACK.store(false, Ordering::SeqCst);
                let _ = window.emit("app-closing", ());
                // Force-close only if the webview never answers (crash/hang). The
                // frontend acks immediately, so the unsaved-changes dialog can stay
                // open as long as the user needs without racing this timer.
                let app = window.app_handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_secs(5));
                    if watchdog_should_exit(
                        generation,
                        CLOSE_GEN.load(Ordering::SeqCst),
                        CLOSE_ACK.load(Ordering::SeqCst),
                    ) {
                        app.exit(0);
                    }
                });
            }
        })
        .run(tauri::generate_context!());

    if let Err(err) = result {
        log::error!(target: "app", "startup: {err}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::safe_log_name;
    use super::watchdog_should_exit;
    use super::level_of;
    use super::log_target;
    use tauri_plugin_log::log;

    #[test]
    fn level_of_maps_the_names_the_webview_sends() {
        assert_eq!(level_of("error"), log::Level::Error);
        assert_eq!(level_of("warn"), log::Level::Warn);
        assert_eq!(level_of("info"), log::Level::Info);
        assert_eq!(level_of("nonsense"), log::Level::Info);
    }

    #[test]
    fn log_target_maps_the_categories_the_webview_sends() {
        assert_eq!(log_target("git"), "git");
        assert_eq!(log_target("app"), "app");
        assert_eq!(log_target("editor"), "editor");
        assert_eq!(log_target("whatever"), "editor");
    }

    #[test]
    fn open_external_only_takes_http_links() {
        // The URL reaches a process argv: never let anything else through.
        assert!(!super::openable("file:///etc/passwd"));
        assert!(!super::openable("--help"));
        assert!(!super::openable("javascript:alert(1)"));
        assert!(super::openable("https://github.com/login/device"));
    }

    #[test]
    fn log_names_cannot_leave_the_log_directory() {
        assert!(safe_log_name("gitpad.log"));
        assert!(safe_log_name("gitpad_2026-11-04_13-21-04.log"));
        assert!(!safe_log_name("../gitpad.log"));
        assert!(!safe_log_name("/etc/gitpad.log"));
        assert!(!safe_log_name("gitpad/../evil.log"));
        assert!(!safe_log_name("notes.txt"));
        assert!(!safe_log_name("other.log"));
    }

    #[test]
    fn watchdog_exits_for_its_own_unacked_close() {
        assert!(watchdog_should_exit(7, 7, false));
    }

    #[test]
    fn watchdog_stands_down_once_acked() {
        assert!(!watchdog_should_exit(7, 7, true));
    }

    #[test]
    fn watchdog_ignores_a_later_close_request() {
        assert!(!watchdog_should_exit(7, 8, false));
    }
}
