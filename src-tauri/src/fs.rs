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
pub fn fs_read_file(app: AppHandle, path: String) -> Result<String, String> {
    let (root, rel) = resolve(&app, &path)?;
    std::fs::read_to_string(root.join(rel)).map_err(|e| {
        if e.kind() == std::io::ErrorKind::InvalidData {
            format!("{path} is not UTF-8 text")
        } else {
            format!("read {path}: {e}")
        }
    })
}

#[tauri::command]
pub async fn fs_write_file(
    app: AppHandle,
    path: String,
    content: String,
) -> Result<(), String> {
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
pub fn fs_create_dir(app: AppHandle, path: String) -> Result<(), String> {
    let (root, rel) = resolve(&app, &path)?;
    let abs = root.join(rel);
    std::fs::create_dir_all(&abs).map_err(|e| format!("create dir {path}: {e}"))?;
    let empty = std::fs::read_dir(&abs)
        .map_err(|e| format!("create dir {path}: {e}"))?
        .next()
        .is_none();
    if empty {
        std::fs::write(abs.join(".gitkeep"), "").map_err(|e| format!("create dir {path}: {e}"))?;
    }
    let _ = git::emit_current_status(&app);
    Ok(())
}

#[tauri::command]
pub async fn fs_delete(app: AppHandle, path: String) -> Result<(), String> {
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
pub async fn fs_rename(app: AppHandle, from: String, to: String) -> Result<(), String> {
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

#[tauri::command]
pub fn fs_list_dir(app: AppHandle, path: Option<String>) -> Result<Vec<FileEntry>, String> {
    let root = git::repo_dir(&app)?;
    let base = match path {
        Some(path) => root.join(resolve(&app, &path)?.1),
        None => root.clone(),
    };
    build_tree(&root, &base)
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
}
