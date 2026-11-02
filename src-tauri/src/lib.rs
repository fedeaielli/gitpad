mod auth;
mod config;
mod fs;
mod git;
mod github;
mod search;

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
    if let Ok(path) = git::repo_dir(&app) {
        let gate = app.state::<AppState>().repo_gate.clone();
        // The worktree gate is taken with a deadline: if a pull or an import
        // holds it, skip the closing commit instead of burning the timeout.
        // Push only: fetching at exit just delays closing, and a rejected push
        // (remote moved) leaves the local commit for the next sync.
        let _ = tokio::time::timeout(
            Duration::from_secs(3),
            tokio::task::spawn_blocking(move || {
                // Keyring reads are blocking: keep them off the async runtime.
                let token = auth::token().ok();
                git::push_only_blocking(&path, None, token, git::GateWait::Until(&gate, GATE_WAIT))
            }),
        )
        .await;
    }
    app.exit(0);
}

/// SIGTERM: commit what is on disk and push before dying. `push_only_blocking`
/// commits pending changes first, so this is the whole shutdown path.
fn terminate(app: tauri::AppHandle) {
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
/// A hang after the ack no longer trips the watchdog; accepted, because the
/// flush only writes locally and the exit push has its own timeout.
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
                repo_gate: std::sync::Arc::new(std::sync::RwLock::new(())),
                commit_gate: std::sync::Arc::new(std::sync::Mutex::new(())),
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
            auth::auth_has_token,
            auth::auth_clear,
            fs::fs_read_file,
            fs::fs_write_file,
            fs::fs_create_dir,
            fs::fs_delete,
            fs::fs_rename,
            fs::fs_list_dir,
            fs::fs_repo_name,
            fs::reveal_in_filemanager,
            fs::fs_import,
            git::git_clone,
            git::git_move_repo,
            git::git_import_repo,
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
        eprintln!("gitpad: {err}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::watchdog_should_exit;

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
