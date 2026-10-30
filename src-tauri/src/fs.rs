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
        if !matches!(component, Component::Normal(_)) {
            return Err(format!("invalid path: {path}"));
        }
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
    std::fs::read_to_string(root.join(rel)).map_err(|e| format!("read {path}: {e}"))
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
        .map_err(|e| format!("git task: {e}"))?
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
    Ok(())
}

#[tauri::command]
pub fn fs_delete(app: AppHandle, path: String) -> Result<(), String> {
    let (root, rel) = resolve(&app, &path)?;
    if rel.as_os_str().is_empty() {
        return Err("refusing to delete the repository root".into());
    }
    let abs = root.join(rel);
    let metadata = std::fs::symlink_metadata(&abs).map_err(|e| format!("delete {path}: {e}"))?;
    if metadata.is_dir() {
        std::fs::remove_dir_all(&abs).map_err(|e| format!("delete {path}: {e}"))
    } else {
        std::fs::remove_file(&abs).map_err(|e| format!("delete {path}: {e}"))
    }
}

#[tauri::command]
pub fn fs_rename(app: AppHandle, from: String, to: String) -> Result<(), String> {
    let (root, from_rel) = resolve(&app, &from)?;
    let (_, to_rel) = resolve(&app, &to)?;
    std::fs::rename(root.join(from_rel), root.join(to_rel)).map_err(|e| format!("rename {from}: {e}"))
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
}
