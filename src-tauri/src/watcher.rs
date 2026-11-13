//! Debounced filesystem watch on the repository root.
//!
//! The watch reports what changed, never touches git: recomputing the status
//! from this thread would contend for `repo_gate` with every autosave.

use crate::git::FileChange;
use crate::AppState;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use notify_debouncer_full::notify::event::EventKind;
use notify_debouncer_full::notify::RecursiveMode;
use notify_debouncer_full::notify::RecommendedWatcher;
use notify_debouncer_full::{new_debouncer, DebounceEventResult, Debouncer, RecommendedCache};
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_log::log;

/// A burst of saves from another editor arrives as many events: coalesce them.
const DEBOUNCE: Duration = Duration::from_millis(500);

pub struct Handle {
    root: PathBuf,
    /// Dropping it stops the watch thread.
    _debouncer: Debouncer<RecommendedWatcher, RecommendedCache>,
}

/// The `.gitignore` rules of one repository.
pub(crate) struct Filters {
    gitignore: Gitignore,
}

pub(crate) fn filters_for(root: &Path) -> Filters {
    let mut builder = GitignoreBuilder::new(root);
    // A missing or broken .gitignore is normal: the watch still works.
    if let Some(err) = builder.add(root.join(".gitignore")) {
        log::debug!(target: "app", "watcher: {err}");
    }
    let gitignore = match builder.build() {
        Ok(gitignore) => gitignore,
        Err(e) => {
            log::warn!(target: "app", "watcher: bad .gitignore: {e}");
            Gitignore::empty()
        }
    };
    Filters { gitignore }
}

fn status_of(kind: &EventKind) -> Option<&'static str> {
    match kind {
        EventKind::Create(_) => Some("A"),
        EventKind::Remove(_) => Some("D"),
        EventKind::Modify(_) => Some("M"),
        // Unsupported backends report the catch-all: treat it as a content edit.
        EventKind::Any | EventKind::Other => Some("M"),
        EventKind::Access(_) => None,
    }
}

/// One event as the frontend knows it, or `None` when the path is out of scope.
pub(crate) fn classify(
    root: &Path,
    path: &Path,
    kind: &EventKind,
    filters: &Filters,
) -> Option<FileChange> {
    let status = status_of(kind)?;
    let rel = path.strip_prefix(root).ok()?;
    if rel.as_os_str().is_empty() {
        return None;
    }
    // Our own atomic save writes `.git/gitpad-save.tmp`, and a commit rewrites
    // the index: every one of those would fire the watch.
    if rel.components().any(|c| c == Component::Normal(".git".as_ref())) {
        return None;
    }
    if filters
        .gitignore
        .matched_path_or_any_parents(path, path.is_dir())
        .is_ignore()
    {
        return None;
    }
    Some(FileChange {
        path: crate::fs::rel_path(rel),
        status: status.to_string(),
    })
}

/// One debounced batch: dedupe paths, then tell the frontend once.
fn on_batch(app: &AppHandle, root: &Path, filters: &Filters, result: DebounceEventResult) {
    let events = match result {
        Ok(events) => events,
        Err(errors) => {
            for e in errors {
                log::warn!(target: "app", "watcher: {e}");
            }
            return;
        }
    };
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut changes: Vec<FileChange> = Vec::new();
    for debounced in events {
        for path in &debounced.paths {
            let Some(change) = classify(root, path, &debounced.kind, filters) else {
                continue;
            };
            // Keep the first position, take the latest status: a create followed
            // by a write in the same batch is one modified file.
            if let Some(&index) = seen.get(&change.path) {
                changes[index].status = change.status;
            } else {
                seen.insert(change.path.clone(), changes.len());
                changes.push(change);
            }
        }
    }
    if changes.is_empty() {
        return;
    }
    if let Err(e) = app.emit("fs-changed", &changes) {
        log::warn!(target: "app", "watcher: emit: {e}");
    }
}

/// Start watching `root`, or leave the running watch alone when it already does.
///
/// Called at startup and after every change of the active repository. A failure
/// is not fatal: the file tree still refreshes when the window regains focus.
pub fn ensure(app: &AppHandle, root: &Path) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mut slot = state
        .watcher
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if let Some(handle) = slot.as_ref() {
        if handle.root == root && root.is_dir() {
            return Ok(());
        }
    }
    // Drop the previous watch before taking the new one.
    *slot = None;

    let filters = filters_for(root);
    let emit_app = app.clone();
    let emit_root = root.to_path_buf();
    let mut debouncer = new_debouncer(DEBOUNCE, None, move |result: DebounceEventResult| {
        on_batch(&emit_app, &emit_root, &filters, result)
    })
    .map_err(|e| format!("watcher: {e}"))?;
    debouncer
        .watch(root, RecursiveMode::Recursive)
        .map_err(|e| format!("watch {}: {e}", root.display()))?;
    log::info!(target: "app", "watching {}", root.display());
    *slot = Some(Handle {
        root: root.to_path_buf(),
        _debouncer: debouncer,
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify_debouncer_full::notify::event::{CreateKind, ModifyKind, RemoveKind};
    use std::path::PathBuf;

    /// A private directory under the system temp dir, removed by the caller.
    fn temp_repo(tag: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "gitpad-watcher-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("temp dir");
        root
    }

    #[test]
    fn maps_event_kinds_to_porcelain_codes() {
        let root = temp_repo("kinds");
        let filters = filters_for(&root);
        let file = root.join("note.md");
        assert_eq!(
            classify(&root, &file, &EventKind::Create(CreateKind::File), &filters).map(|c| c.status),
            Some("A".to_string())
        );
        assert_eq!(
            classify(
                &root,
                &file,
                &EventKind::Modify(ModifyKind::Name(
                    notify_debouncer_full::notify::event::RenameMode::Any
                )),
                &filters
            )
            .map(|c| c.status),
            Some("M".to_string())
        );
        assert_eq!(
            classify(&root, &file, &EventKind::Remove(RemoveKind::File), &filters).map(|c| c.status),
            Some("D".to_string())
        );
        assert!(classify(&root, &file, &EventKind::Any, &filters).is_some());
        assert!(classify(
            &root,
            &file,
            &EventKind::Access(notify_debouncer_full::notify::event::AccessKind::Read),
            &filters
        )
        .is_none());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn keeps_the_relative_path() {
        let root = temp_repo("relative");
        let filters = filters_for(&root);
        let change = classify(
            &root,
            &root.join("sub/note.md"),
            &EventKind::Create(CreateKind::File),
            &filters,
        )
        .expect("change");
        assert_eq!(change.path, "sub/note.md");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn ignores_everything_under_dot_git() {
        let root = temp_repo("dotgit");
        let filters = filters_for(&root);
        for path in [
            root.join(".git/gitpad-save.tmp"),
            root.join(".git/index"),
            root.join(".git/refs/heads/main"),
        ] {
            assert!(
                classify(&root, &path, &EventKind::Any, &filters).is_none(),
                "{} must be ignored",
                path.display()
            );
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn ignores_paths_outside_the_repository() {
        let root = temp_repo("outside");
        let other = temp_repo("outside-other");
        let filters = filters_for(&root);
        assert!(classify(
            &root,
            &other.join("note.md"),
            &EventKind::Create(CreateKind::File),
            &filters
        )
        .is_none());
        assert!(classify(&root, &root, &EventKind::Any, &filters).is_none());
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&other);
    }

    #[test]
    fn drops_gitignored_paths() {
        let root = temp_repo("gitignore");
        std::fs::write(root.join(".gitignore"), "build/\nsecret.md\n").expect("write");
        let filters = filters_for(&root);
        for path in [root.join("build/out.md"), root.join("secret.md")] {
            assert!(
                classify(&root, &path, &EventKind::Any, &filters).is_none(),
                "{} must be ignored",
                path.display()
            );
        }
        assert!(classify(&root, &root.join("keep.md"), &EventKind::Any, &filters).is_some());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Needs a real inotify watch: kept out of the default run, where a loaded
    /// machine or a container without inotify would make it flake.
    #[test]
    #[ignore]
    fn reports_a_real_write_once() {
        use std::sync::mpsc;
        let root = temp_repo("live");
        let (tx, rx) = mpsc::channel::<Vec<PathBuf>>();
        let mut debouncer = new_debouncer(
            Duration::from_millis(200),
            None,
            move |result: DebounceEventResult| {
                if let Ok(events) = result {
                    for e in events {
                        let _ = tx.send(e.paths.clone());
                    }
                }
            },
        )
        .expect("debouncer");
        debouncer
            .watch(&root, RecursiveMode::Recursive)
            .expect("watch");
        std::fs::write(root.join("note.md"), "hello").expect("write");
        let paths = rx
            .recv_timeout(Duration::from_secs(2))
            .expect("an event for the write");
        assert!(paths.iter().any(|p| p.ends_with("note.md")));
        let _ = std::fs::remove_dir_all(&root);
    }
}
