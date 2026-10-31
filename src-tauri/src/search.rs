use crate::git;
use ignore::WalkBuilder;
use serde::Serialize;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use tauri::AppHandle;

const MAX_RESULTS: usize = 200;
const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub snippet: String,
}

pub fn search_in(root: &Path, query: &str) -> Result<Vec<SearchResult>, String> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }
    let mut results = Vec::new();
    for entry in WalkBuilder::new(root).build().flatten() {
        if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if metadata.len() > MAX_FILE_BYTES {
            continue;
        }
        let Ok(handle) = File::open(entry.path()) else {
            continue;
        };
        let file = entry
            .path()
            .strip_prefix(root)
            .map_err(|_| "path outside repository".to_string())?
            .to_string_lossy()
            .to_string();
        for (index, line) in BufReader::new(handle).lines().enumerate() {
            // Non-UTF-8 files are skipped silently.
            let Ok(line) = line else {
                break;
            };
            if let Some(column) = line.find(query) {
                results.push(SearchResult {
                    file: file.clone(),
                    line: index + 1,
                    column: column + 1,
                    snippet: line.trim().chars().take(200).collect(),
                });
                if results.len() >= MAX_RESULTS {
                    return Ok(results);
                }
            }
        }
    }
    Ok(results)
}

#[tauri::command]
pub async fn search_global(app: AppHandle, query: String) -> Result<Vec<SearchResult>, git::GitError> {
    let root = git::repo_dir(&app)?;
    let results = tokio::task::spawn_blocking(move || search_in(&root, &query))
        .await
        .map_err(|e| format!("search task: {e}"))??;
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_matches_and_ignores_gitignored_files() {
        let root = std::env::temp_dir().join(format!("gitpad-search-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("notes")).unwrap();
        std::fs::create_dir_all(root.join("build")).unwrap();
        git2::Repository::init(&root).unwrap();
        std::fs::write(root.join("notes/a.md"), "hello\nsay hello again\n").unwrap();
        std::fs::write(root.join("b.txt"), "nothing\n").unwrap();
        std::fs::write(root.join("build/ignored.md"), "hello\n").unwrap();
        std::fs::write(root.join(".gitignore"), "build/\n").unwrap();

        let results = search_in(&root, "hello").unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].file, "notes/a.md");
        assert_eq!(results[0].line, 1);
        assert_eq!(results[0].column, 1);
        assert_eq!(results[1].line, 2);
        assert_eq!(results[1].column, 5);
        assert!(search_in(&root, "  ").unwrap().is_empty());

        let _ = std::fs::remove_dir_all(&root);
    }
}
