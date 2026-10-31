use crate::git;
use serde::Serialize;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use tauri::{AppHandle, Manager};

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<FileEntry>>,
    /// Seconds since the epoch, for the "modified" sort mode.
    pub modified: Option<u64>,
}

/// Resolves a repo-relative (or absolute-inside-repo) path and refuses anything that escapes the clone.
fn resolve_in(root: &Path, path: &str) -> Result<PathBuf, String> {
    let candidate = Path::new(path);
    let rel = if candidate.is_absolute() {
        candidate
            .strip_prefix(root)
            .map_err(|_| format!("path outside repository: {path}"))?
            .to_path_buf()
    } else {
        candidate.to_path_buf()
    };
    for component in rel.components() {
        match component {
            Component::Normal(name) if name == ".git" => {
                return Err("refusing to touch .git".to_string())
            }
            Component::Normal(_) | Component::CurDir => {}
            _ => return Err(format!("invalid path: {path}")),
        }
    }
    // Symlinks would let a path inside the clone reach files outside app_data.
    let root = root
        .canonicalize()
        .map_err(|e| format!("repository: {e}"))?;
    let abs = root.join(&rel);
    let mut probe = abs.as_path();
    while !probe.exists() {
        probe = probe
            .parent()
            .ok_or_else(|| format!("path outside repository: {path}"))?;
    }
    let real = probe
        .canonicalize()
        .map_err(|e| format!("path {path}: {e}"))?;
    if !real.starts_with(&root) {
        return Err(format!("path escapes repository: {path}"));
    }
    if abs
        .symlink_metadata()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
    {
        return Err(format!("refusing symlink: {path}"));
    }
    Ok(rel)
}

fn resolve(app: &AppHandle, path: &str) -> Result<(PathBuf, PathBuf), String> {
    let root = git::repo_dir(app)?;
    let rel = resolve_in(&root, path)?;
    Ok((root, rel))
}

fn build_tree(root: &Path, dir: &Path) -> Result<Vec<FileEntry>, String> {
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(|e| format!("read dir {}: {e}", dir.display()))? {
        let entry = entry.map_err(|e| format!("read dir {}: {e}", dir.display()))?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name == ".git" {
            continue;
        }
        let abs = entry.path();
        let rel = abs
            .strip_prefix(root)
            .map_err(|_| "path outside repository".to_string())?
            .to_string_lossy()
            .to_string();
        let file_type = entry
            .file_type()
            .map_err(|e| format!("read dir {}: {e}", dir.display()))?;
        let modified = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs());
        if file_type.is_dir() {
            entries.push(FileEntry {
                name,
                path: rel,
                kind: "dir".into(),
                children: Some(build_tree(root, &abs)?),
                modified,
            });
        } else if file_type.is_file() {
            entries.push(FileEntry {
                name,
                path: rel,
                kind: "file".into(),
                children: None,
                modified,
            });
        }
    }
    entries.sort_by(|a, b| {
        a.kind
            .cmp(&b.kind)
            .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(entries)
}

fn repo_gate(app: &AppHandle) -> std::sync::Arc<std::sync::RwLock<()>> {
    app.state::<crate::AppState>().repo_gate.clone()
}

fn commit_gate(app: &AppHandle) -> std::sync::Arc<std::sync::Mutex<()>> {
    app.state::<crate::AppState>().commit_gate.clone()
}

/// Replace `target` with one rename: a crash leaves either the old or the new
/// file, never a truncated one. `tmp` must be on the same filesystem.
pub(crate) fn write_atomic(target: &Path, tmp: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let result = (|| -> std::io::Result<()> {
        let mut file = std::fs::File::create(tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        if let Ok(metadata) = std::fs::metadata(target) {
            std::fs::set_permissions(tmp, metadata.permissions())?;
        }
        drop(file);
        std::fs::rename(tmp, target).map_err(|e| {
            if e.kind() == std::io::ErrorKind::CrossesDevices {
                std::io::Error::new(
                    e.kind(),
                    format!(
                        "rename {} -> {}: {e} (temp and target must share a filesystem)",
                        tmp.display(),
                        target.display()
                    ),
                )
            } else {
                e
            }
        })?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(tmp);
    } else if let Some(dir) = target.parent() {
        // Best effort: the rename is already durable enough to be atomic.
        if let Ok(dir) = std::fs::File::open(dir) {
            let _ = dir.sync_all();
        }
    }
    result
}

/// Write the file and commit it while holding the commit gate: two autosaves
/// cannot race on `.git/index.lock`.
fn write_and_commit(
    root: &std::path::Path,
    rel: &std::path::Path,
    content: &str,
    commit_gate: &std::sync::Mutex<()>,
) -> Result<(), String> {
    let _commit = commit_gate.lock().unwrap_or_else(|e| e.into_inner());
    // The temp lives in .git: same filesystem as the worktree, and invisible to
    // build_tree, search and commit_pending.
    let git_dir = root.join(".git");
    if !git_dir.is_dir() {
        return Err(format!(
            "cannot save atomically: {} is not a directory",
            git_dir.display()
        ));
    }
    write_atomic(
        &root.join(rel),
        &git_dir.join("gitpad-save.tmp"),
        content.as_bytes(),
    )
    .map_err(|e| format!("write {}: {e}", rel.display()))?;
    let message = format!("gitpad: update {} {}", rel.display(), git::iso8601_now());
    git::commit_paths(root, &[rel.to_path_buf()], &message).map_err(|e| e.to_string())
}

const MAX_OPEN_BYTES: u64 = 5 * 1024 * 1024;

fn read_error(label: &str, e: std::io::Error) -> git::GitError {
    match e.kind() {
        std::io::ErrorKind::NotFound => git::GitError::NotFound(format!("read {label}: {e}")),
        std::io::ErrorKind::InvalidData => {
            git::GitError::Other(format!("{label} is not UTF-8 text"))
        }
        _ => git::GitError::Other(format!("read {label}: {e}")),
    }
}

/// Read a text file for the editor, refusing huge buffers and keeping NOT_FOUND
/// so the frontend can close the tab instead of marking it stale.
fn read_text(abs: &Path, label: &str) -> Result<String, git::GitError> {
    let metadata = std::fs::metadata(abs).map_err(|e| read_error(label, e))?;
    if metadata.len() > MAX_OPEN_BYTES {
        let mb = metadata.len() as f64 / (1024.0 * 1024.0);
        return Err(git::GitError::Other(format!(
            "{label}: file too large to open: {mb:.1} MB, limit 5 MB"
        )));
    }
    std::fs::read_to_string(abs).map_err(|e| read_error(label, e))
}

#[tauri::command]
pub async fn fs_read_file(app: AppHandle, path: String) -> Result<String, git::GitError> {
    let (root, rel) = resolve(&app, &path)?;
    let label = path;
    let gate = repo_gate(&app);
    let content = tokio::task::spawn_blocking(move || {
        let _gate = gate.read().map_err(|e| e.to_string())?;
        read_text(&root.join(rel), &label)
    })
    .await
    .map_err(|e| format!("read task: {e}"))??;
    Ok(content)
}

#[tauri::command]
pub async fn fs_write_file(
    app: AppHandle,
    path: String,
    content: String,
) -> Result<(), git::GitError> {
    let (root, rel) = resolve(&app, &path)?;
    let gate = repo_gate(&app);
    let commit_gate = commit_gate(&app);
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        let _gate = gate.read().map_err(|e| e.to_string())?;
        write_and_commit(&root, &rel, &content, &commit_gate)
    })
    .await
    .map_err(|e| format!("git task: {e}"))??;
    // Autosave flow: refresh ahead/behind so the status bar shows "queued".
    let app_status = app.clone();
    let _ = tokio::task::spawn_blocking(move || git::emit_current_status(&app_status)).await;
    Ok(())
}

#[tauri::command]
pub async fn fs_create_dir(app: AppHandle, path: String) -> Result<(), git::GitError> {
    let (root, rel) = resolve(&app, &path)?;
    let label = path;
    let gate = repo_gate(&app);
    tokio::task::spawn_blocking(move || create_dir_in(&root, &rel, &label, &gate))
        .await
        .map_err(|e| format!("create dir task: {e}"))??;
    let app_status = app.clone();
    let _ = tokio::task::spawn_blocking(move || git::emit_current_status(&app_status)).await;
    Ok(())
}

fn create_dir_in(
    root: &Path,
    rel: &Path,
    label: &str,
    gate: &std::sync::RwLock<()>,
) -> Result<(), String> {
    let _gate = gate.write().map_err(|e| e.to_string())?;
    let abs = root.join(rel);
    std::fs::create_dir_all(&abs).map_err(|e| format!("create dir {label}: {e}"))?;
    let empty = std::fs::read_dir(&abs)
        .map_err(|e| format!("create dir {label}: {e}"))?
        .next()
        .is_none();
    if empty {
        std::fs::write(abs.join(".gitkeep"), "")
            .map_err(|e| format!("create dir {label}: {e}"))?;
    }
    Ok(())
}

#[tauri::command]
pub async fn fs_delete(app: AppHandle, path: String) -> Result<(), git::GitError> {
    let (root, rel) = resolve(&app, &path)?;
    if rel.as_os_str().is_empty() {
        return Err("refusing to delete the repository root".into());
    }
    let label = rel.display().to_string();
    let root_task = root.clone();
    let gate = repo_gate(&app);
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let _gate = gate.write().map_err(|e| e.to_string())?;
        let abs = root_task.join(&rel);
        let metadata =
            std::fs::symlink_metadata(&abs).map_err(|e| format!("delete {label}: {e}"))?;
        if metadata.is_dir() {
            std::fs::remove_dir_all(&abs).map_err(|e| format!("delete {label}: {e}"))?;
        } else {
            std::fs::remove_file(&abs).map_err(|e| format!("delete {label}: {e}"))?;
        }
        // The filesystem is the source of truth: a failed commit is retried by the next push.
        if let Err(e) = git::commit_all(&root_task, &format!("gitpad: delete {label}")) {
            eprintln!("gitpad: commit after delete failed: {e}");
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())??;
    let app_status = app.clone();
    let _ = tokio::task::spawn_blocking(move || git::emit_current_status(&app_status)).await;
    Ok(())
}

fn rename_in(root: &Path, from: &str, to: &str) -> Result<(), String> {
    let from_rel = resolve_in(root, from)?;
    let to_rel = resolve_in(root, to)?;
    if from_rel.as_os_str().is_empty() {
        return Err("refusing to rename the repository root".into());
    }
    if root.join(&to_rel).symlink_metadata().is_ok() {
        return Err(format!("target already exists: {to}"));
    }
    std::fs::rename(root.join(from_rel), root.join(to_rel)).map_err(|e| format!("rename {from}: {e}"))
}

#[tauri::command]
pub async fn fs_rename(app: AppHandle, from: String, to: String) -> Result<(), git::GitError> {
    let root = git::repo_dir(&app)?;
    let root_task = root.clone();
    let gate = repo_gate(&app);
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let _gate = gate.write().map_err(|e| e.to_string())?;
        rename_in(&root_task, &from, &to)?;
        // The filesystem is the source of truth: a failed commit is retried by the next push.
        if let Err(e) = git::commit_all(&root_task, &format!("gitpad: rename {from}")) {
            eprintln!("gitpad: commit after rename failed: {e}");
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())??;
    let app_status = app.clone();
    let _ = tokio::task::spawn_blocking(move || git::emit_current_status(&app_status)).await;
    Ok(())
}

/// `name.ext` -> `name (1).ext` so an import never overwrites an existing file.
fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let candidate = dir.join(name);
    if !candidate.exists() {
        return candidate;
    }
    let path = Path::new(name);
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let ext = path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    for n in 1.. {
        let candidate = dir.join(format!("{stem} ({n}){ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!("an unused import name always exists")
}

/// Recursive copy for imported folders; `.git` and symlinks are skipped.
fn copy_tree(src: &Path, dst: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dst).map_err(|e| format!("import {}: {e}", src.display()))?;
    let entries = std::fs::read_dir(src).map_err(|e| format!("import {}: {e}", src.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("import {}: {e}", src.display()))?;
        if entry.file_name() == ".git" {
            continue;
        }
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|e| format!("import {}: {e}", path.display()))?;
        let target = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_tree(&path, &target)?;
        } else if file_type.is_file() {
            std::fs::copy(&path, &target)
                .map_err(|e| format!("import {}: {e}", path.display()))?;
        }
    }
    Ok(())
}

/// Copies `src` into `dest_dir` and returns the name it got there.
fn import_into(src: &Path, dest_dir: &Path) -> Result<PathBuf, String> {
    let meta =
        std::fs::symlink_metadata(src).map_err(|e| format!("import {}: {e}", src.display()))?;
    if meta.file_type().is_symlink() {
        return Err(format!("refusing to import symlink: {}", src.display()));
    }
    let src = src
        .canonicalize()
        .map_err(|e| format!("import {}: {e}", src.display()))?;
    let dest = dest_dir
        .canonicalize()
        .map_err(|e| format!("import destination {}: {e}", dest_dir.display()))?;
    if !dest.is_dir() {
        return Err(format!(
            "import destination is not a folder: {}",
            dest_dir.display()
        ));
    }
    if meta.is_dir() && dest.starts_with(&src) {
        return Err("refusing to import a folder into itself".into());
    }
    let name = src
        .file_name()
        .ok_or_else(|| format!("import {}: no file name", src.display()))?
        .to_string_lossy()
        .to_string();
    let target = unique_path(&dest, &name);
    if meta.is_dir() {
        copy_tree(&src, &target)?;
    } else if meta.is_file() {
        std::fs::copy(&src, &target).map_err(|e| format!("import {}: {e}", src.display()))?;
    } else {
        return Err(format!("unsupported file type: {}", src.display()));
    }
    Ok(PathBuf::from(target.file_name().unwrap_or_default()))
}

/// Native picker hands us a path outside the repo; the copy itself lands inside.
#[tauri::command]
pub async fn fs_import(
    app: AppHandle,
    source: String,
    dest_dir: Option<String>,
) -> Result<String, git::GitError> {
    let root = git::repo_dir(&app)?;
    let dest_rel = match dest_dir.as_deref() {
        Some(dir) => resolve_in(&root, dir)?,
        None => PathBuf::new(),
    };
    let gate = repo_gate(&app);
    let root_task = root.clone();
    let src = PathBuf::from(&source);
    let rel = tauri::async_runtime::spawn_blocking(move || {
        import_locked(&root_task, &src, &dest_rel, &gate)
    })
    .await
    .map_err(|e| e.to_string())??;
    let app_status = app.clone();
    let _ = tokio::task::spawn_blocking(move || git::emit_current_status(&app_status)).await;
    Ok(rel.to_string_lossy().to_string())
}

fn import_locked(
    root: &Path,
    src: &Path,
    dest_rel: &Path,
    gate: &std::sync::RwLock<()>,
) -> Result<PathBuf, String> {
    // The gate covers the whole copy and its commit: waiting for the autosave
    // is better than committing a half-copied tree. The buffer stays in the
    // webview until this returns.
    let _gate = gate.write().map_err(|e| e.to_string())?;
    let name = import_into(src, &root.join(dest_rel))?;
    let rel = dest_rel.join(name);
    // The copy already happened: a failed commit is retried by the next push.
    if let Err(e) = git::commit_all(root, &format!("gitpad: import {}", rel.display())) {
        eprintln!("gitpad: commit after import failed: {e}");
    }
    Ok(rel)
}

#[tauri::command]
pub async fn reveal_in_filemanager(app: AppHandle, path: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let (root, rel) = resolve(&app, &path)?;
    app.opener()
        .reveal_item_in_dir(root.join(rel))
        .map_err(|e| format!("reveal {path}: {e}"))
}

#[tauri::command]
pub async fn fs_list_dir(app: AppHandle, path: Option<String>) -> Result<Vec<FileEntry>, git::GitError> {
    let root = git::repo_dir(&app)?;
    let base = match path {
        Some(path) => root.join(resolve(&app, &path)?.1),
        None => root.clone(),
    };
    let entries = tokio::task::spawn_blocking(move || build_tree(&root, &base))
        .await
        .map_err(|e| format!("list task: {e}"))??;
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Path jail, exercised against a real temp root.
    #[test]
    fn rejects_paths_that_escape_the_repo() {
        let root = std::env::temp_dir().join(format!("gitpad-jail-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        assert!(resolve_in(&root, "../outside.txt").is_err());
        assert!(resolve_in(&root, "nested/../../outside.txt").is_err());
        assert!(resolve_in(&root, "/etc/passwd").is_err());
        assert_eq!(
            resolve_in(&root, "notes/hello.md").unwrap(),
            PathBuf::from("notes/hello.md")
        );
        assert_eq!(
            resolve_in(&root, root.join("hello.md").to_str().unwrap()).unwrap(),
            PathBuf::from("hello.md")
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn builds_tree_skipping_git_and_sorting_dirs_first() {
        let root = std::env::temp_dir().join(format!("gitpad-tree-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("dir")).unwrap();
        std::fs::create_dir_all(root.join(".git")).unwrap();
        std::fs::write(root.join("b.md"), "b").unwrap();
        std::fs::write(root.join("dir/a.md"), "a").unwrap();

        let tree = build_tree(&root, &root).unwrap();
        assert_eq!(tree.len(), 2);
        assert_eq!(tree[0].kind, "dir");
        assert_eq!(tree[1].name, "b.md");
        assert_eq!(tree[0].path, "dir");
        assert_eq!(tree[0].children.as_ref().unwrap()[0].path, "dir/a.md");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_git_directories_and_symlinks() {
        let root = std::env::temp_dir().join(format!("gitpad-links-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("sub/.git")).unwrap();
        std::fs::write(root.join("note.md"), "note").unwrap();

        assert!(resolve_in(&root, ".git/config").is_err());
        assert!(resolve_in(&root, "sub/.git/config").is_err());

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let outside =
                std::env::temp_dir().join(format!("gitpad-outside-{}", std::process::id()));
            std::fs::write(&outside, "secret").unwrap();
            symlink(&outside, root.join("link")).unwrap();
            symlink(&outside, root.join("link.md")).unwrap();
            assert!(resolve_in(&root, "link").is_err());
            assert!(resolve_in(&root, "link.md").is_err());
            let _ = std::fs::remove_file(&outside);
        }

        assert_eq!(resolve_in(&root, "note.md").unwrap(), PathBuf::from("note.md"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn rename_refuses_existing_target_and_root() {
        let root = std::env::temp_dir().join(format!("gitpad-rename-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("a.md"), "a").unwrap();
        std::fs::write(root.join("b.md"), "b").unwrap();

        assert!(rename_in(&root, "a.md", "b.md").is_err());
        assert!(rename_in(&root, "", "x.md").is_err());
        assert_eq!(std::fs::read_to_string(root.join("b.md")).unwrap(), "b");

        rename_in(&root, "a.md", "c.md").unwrap();
        assert!(!root.join("a.md").exists());
        assert_eq!(std::fs::read_to_string(root.join("c.md")).unwrap(), "a");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn import_copies_files_and_renames_collisions() {
        let base = std::env::temp_dir().join(format!("gitpad-import-file-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let src = base.join("src");
        let root = base.join("repo");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(src.join("a.md"), "one").unwrap();

        let name = import_into(&src.join("a.md"), &root).unwrap();
        assert_eq!(name, PathBuf::from("a.md"));
        assert_eq!(std::fs::read_to_string(root.join("a.md")).unwrap(), "one");

        let name = import_into(&src.join("a.md"), &root).unwrap();
        assert_eq!(name, PathBuf::from("a (1).md"));
        assert_eq!(std::fs::read_to_string(root.join("a (1).md")).unwrap(), "one");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn import_folder_skips_git_and_refuses_self_copy() {
        let base = std::env::temp_dir().join(format!("gitpad-import-dir-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let src = base.join("src/notes");
        let root = base.join("repo");
        std::fs::create_dir_all(src.join(".git")).unwrap();
        std::fs::create_dir_all(src.join("sub")).unwrap();
        std::fs::create_dir_all(root.join("target")).unwrap();
        std::fs::write(src.join("keep.md"), "k").unwrap();
        std::fs::write(src.join("sub/nested.md"), "n").unwrap();
        std::fs::write(src.join(".git/config"), "x").unwrap();

        let name = import_into(&base.join("src/notes"), &root.join("target")).unwrap();
        assert_eq!(name, PathBuf::from("notes"));
        assert_eq!(
            std::fs::read_to_string(root.join("target/notes/keep.md")).unwrap(),
            "k"
        );
        assert_eq!(
            std::fs::read_to_string(root.join("target/notes/sub/nested.md")).unwrap(),
            "n"
        );
        assert!(!root.join("target/notes/.git").exists());
        assert!(import_into(&root.join("target/notes"), &root.join("target/notes/sub")).is_err());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn create_dir_holds_and_releases_the_gate() {
        let root = std::env::temp_dir().join(format!("gitpad-mkdir-gate-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let gate = std::sync::RwLock::new(());

        create_dir_in(&root, Path::new("notes"), "notes", &gate).unwrap();
        assert!(root.join("notes/.gitkeep").is_file());
        assert!(gate.try_write().is_ok());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn import_holds_and_releases_the_gate() {
        let base = std::env::temp_dir().join(format!("gitpad-import-gate-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let root = base.join("repo");
        std::fs::create_dir_all(&root).unwrap();
        git2::Repository::init(&root).unwrap();
        std::fs::write(base.join("a.md"), "one").unwrap();
        let gate = std::sync::RwLock::new(());

        let rel = import_locked(&root, &base.join("a.md"), Path::new(""), &gate).unwrap();
        assert_eq!(rel, PathBuf::from("a.md"));
        assert_eq!(std::fs::read_to_string(root.join("a.md")).unwrap(), "one");
        assert!(gate.try_write().is_ok());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn read_text_reports_missing_files_as_not_found() {
        let dir = std::env::temp_dir().join(format!("gitpad-read-missing-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let err = read_text(&dir.join("nope.md"), "nope.md").unwrap_err();
        assert!(matches!(err, git::GitError::NotFound(_)));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_text_refuses_files_over_the_limit() {
        let dir = std::env::temp_dir().join(format!("gitpad-read-large-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("big.log");
        std::fs::write(&path, vec![b'a'; 6 * 1024 * 1024]).unwrap();

        let err = read_text(&path, "big.log").unwrap_err();
        assert!(err.to_string().contains("limit 5 MB"), "{err}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_text_keeps_the_not_utf8_error() {
        let dir = std::env::temp_dir().join(format!("gitpad-read-binary-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("binary.bin");
        std::fs::write(&path, [0xff, 0xfe]).unwrap();

        let err = read_text(&path, "binary.bin").unwrap_err();
        assert!(err.to_string().contains("not UTF-8"), "{err}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_text_returns_normal_files() {
        let dir = std::env::temp_dir().join(format!("gitpad-read-ok-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("note.md");
        std::fs::write(&path, "hello").unwrap();

        assert_eq!(read_text(&path, "note.md").unwrap(), "hello");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_atomic_replaces_content_and_leaves_no_temp() {
        let dir = std::env::temp_dir().join(format!("gitpad-atomic-ok-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("note.md");
        let tmp = dir.join("save.tmp");
        std::fs::write(&target, "old").unwrap();

        write_atomic(&target, &tmp, b"new").unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "new");
        assert!(!tmp.exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn write_atomic_keeps_existing_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = std::env::temp_dir().join(format!("gitpad-atomic-mode-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("secret.md");
        std::fs::write(&target, "old").unwrap();
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o600)).unwrap();

        write_atomic(&target, &dir.join("save.tmp"), b"new").unwrap();
        let mode = std::fs::metadata(&target).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn write_atomic_failure_keeps_the_old_content() {
        use std::os::unix::fs::PermissionsExt;

        // Root ignores directory permissions, so the failure cannot be provoked.
        if std::process::Command::new("id")
            .arg("-u")
            .output()
            .map(|out| out.stdout.starts_with(b"0"))
            .unwrap_or(false)
        {
            return;
        }

        let dir = std::env::temp_dir().join(format!("gitpad-atomic-fail-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let ro = dir.join("ro");
        std::fs::create_dir_all(&ro).unwrap();
        let target = ro.join("note.md");
        let tmp = dir.join("save.tmp");
        std::fs::write(&target, "old").unwrap();
        std::fs::set_permissions(&ro, std::fs::Permissions::from_mode(0o500)).unwrap();

        assert!(write_atomic(&target, &tmp, b"new").is_err());
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "old");
        assert!(!tmp.exists());

        std::fs::set_permissions(&ro, std::fs::Permissions::from_mode(0o700)).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn autosave_temp_stays_out_of_git_and_the_file_tree() {
        let dir = std::env::temp_dir().join(format!("gitpad-atomic-git-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join("repo");
        std::fs::create_dir_all(&root).unwrap();
        git2::Repository::init(&root).unwrap();

        let gate = std::sync::Mutex::new(());
        write_and_commit(&root, std::path::Path::new("note.md"), "new", &gate).unwrap();

        assert!(!root.join(".git/gitpad-save.tmp").exists());
        let repo = git2::Repository::open(&root).unwrap();
        let mut opts = git2::StatusOptions::new();
        opts.include_untracked(true).recurse_untracked_dirs(true);
        assert!(repo.statuses(Some(&mut opts)).unwrap().is_empty());
        let tree = build_tree(&root, &root).unwrap();
        assert!(tree.iter().all(|entry| entry.name != "gitpad-save.tmp"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn concurrent_autosaves_commit_both_files() {
        let base = std::env::temp_dir().join(format!("gitpad-concurrent-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let root = base.join("repo");
        std::fs::create_dir_all(&root).unwrap();
        git2::Repository::init(&root).unwrap();

        let commit_gate = std::sync::Mutex::new(());
        let rounds = 20;
        for round in 0..rounds {
            let a = format!("a{round}.md");
            let b = format!("b{round}.md");
            std::thread::scope(|scope| {
                let gate = &commit_gate;
                let root = &root;
                let a = scope.spawn(|| {
                    write_and_commit(root, std::path::Path::new(&a), "a", gate)
                });
                let b = scope.spawn(|| {
                    write_and_commit(root, std::path::Path::new(&b), "b", gate)
                });
                assert!(a.join().unwrap().is_ok());
                assert!(b.join().unwrap().is_ok());
            });
        }

        assert!(!root.join(".git/index.lock").exists());
        let repo = git2::Repository::open(&root).unwrap();
        let mut walk = repo.revwalk().unwrap();
        walk.push_head().unwrap();
        assert_eq!(walk.count(), rounds * 2);
        let _ = std::fs::remove_dir_all(&base);
    }
}
