use crate::git;
use serde::Serialize;
use std::path::{Component, Path, PathBuf};
use tauri::AppHandle;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<FileEntry>>,
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
        if file_type.is_dir() {
            entries.push(FileEntry {
                name,
                path: rel,
                kind: "dir".into(),
                children: Some(build_tree(root, &abs)?),
            });
        } else if file_type.is_file() {
            entries.push(FileEntry {
                name,
                path: rel,
                kind: "file".into(),
                children: None,
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

#[tauri::command]
pub async fn fs_read_file(app: AppHandle, path: String) -> Result<String, git::GitError> {
    let (root, rel) = resolve(&app, &path)?;
    let label = path;
    let content = tokio::task::spawn_blocking(move || {
        std::fs::read_to_string(root.join(rel)).map_err(|e| {
            if e.kind() == std::io::ErrorKind::InvalidData {
                format!("{label} is not UTF-8 text")
            } else {
                format!("read {label}: {e}")
            }
        })
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
    std::fs::write(root.join(&rel), content).map_err(|e| format!("write {path}: {e}"))?;
    let message = format!("gitpad: update {} {}", rel.display(), git::iso8601_now());
    tokio::task::spawn_blocking(move || git::commit_paths(&root, &[rel], &message))
        .await
        .map_err(|e| format!("git task: {e}"))??;
    // Autosave flow: refresh ahead/behind so the status bar shows "queued".
    let _ = git::emit_current_status(&app);
    Ok(())
}

#[tauri::command]
pub async fn fs_create_dir(app: AppHandle, path: String) -> Result<(), git::GitError> {
    let (root, rel) = resolve(&app, &path)?;
    let label = path;
    tokio::task::spawn_blocking(move || -> Result<(), String> {
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
    })
    .await
    .map_err(|e| format!("create dir task: {e}"))??;
    let _ = git::emit_current_status(&app);
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
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
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
    let _ = git::emit_current_status(&app);
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
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        rename_in(&root_task, &from, &to)?;
        // The filesystem is the source of truth: a failed commit is retried by the next push.
        if let Err(e) = git::commit_all(&root_task, &format!("gitpad: rename {from}")) {
            eprintln!("gitpad: commit after rename failed: {e}");
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())??;
    let _ = git::emit_current_status(&app);
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
    let root_task = root.clone();
    let src = PathBuf::from(&source);
    let rel = tauri::async_runtime::spawn_blocking(move || -> Result<PathBuf, String> {
        let name = import_into(&src, &root_task.join(&dest_rel))?;
        let rel = dest_rel.join(name);
        // The copy already happened: a failed commit is retried by the next push.
        if let Err(e) = git::commit_all(&root_task, &format!("gitpad: import {}", rel.display())) {
            eprintln!("gitpad: commit after import failed: {e}");
        }
        Ok(rel)
    })
    .await
    .map_err(|e| e.to_string())??;
    let _ = git::emit_current_status(&app);
    Ok(rel.to_string_lossy().to_string())
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
}
