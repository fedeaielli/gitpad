use crate::auth;
use git2::{
    build::{CheckoutBuilder, RepoBuilder}, FetchOptions, IndexAddOption, Oid, PushOptions, Repository,
    ResetType, Signature, StatusOptions,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConflictFile {
    pub path: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConflictVersions {
    pub ours: String,
    pub theirs: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GitStatus {
    pub synced: bool,
    pub modified_files: Vec<String>,
    pub conflicts: Vec<ConflictFile>,
    pub ahead: usize,
    pub behind: usize,
    pub diverged: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Resolution {
    AcceptOurs,
    AcceptTheirs,
}

/// Errors cross the Tauri boundary as `{ "code": ..., "message": ... }`.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "code", content = "message", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GitError {
    NotFound(String),
    IoError(String),
    Other(String),
}

impl GitError {
    pub fn not_found() -> Self {
        Self::NotFound("no repository cloned".into())
    }
}

impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(m) | Self::IoError(m) | Self::Other(m) => f.write_str(m),
        }
    }
}

// Lets `?` widen plain String errors produced by helpers and escaping messages.
impl From<String> for GitError {
    fn from(value: String) -> Self {
        Self::Other(value)
    }
}

impl From<GitError> for String {
    fn from(value: GitError) -> Self {
        value.to_string()
    }
}

impl From<&str> for GitError {
    fn from(value: &str) -> Self {
        Self::Other(value.to_string())
    }
}

/// A missing repository is an expected state; anything else is a real IO problem.
fn open_err(e: git2::Error) -> GitError {
    match e.code() {
        git2::ErrorCode::NotFound => GitError::not_found(),
        _ => GitError::IoError(e.to_string()),
    }
}

pub fn open_repo(path: &Path) -> Result<Repository, GitError> {
    match std::fs::metadata(path.join(".git")) {
        Ok(_) => Repository::open(path).map_err(open_err),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(GitError::not_found()),
        Err(e) => Err(GitError::IoError(format!("{}: {e}", path.display()))),
    }
}

fn git_err(e: git2::Error) -> GitError {
    GitError::Other(format!("git: {}", e.message()))
}

fn signature() -> Result<Signature<'static>, GitError> {
    Signature::now("GitPad", "gitpad@localhost").map_err(git_err)
}

fn repos_root(app: &AppHandle) -> Result<PathBuf, GitError> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("app data dir: {e}"))?
        .join("repos");
    std::fs::create_dir_all(&dir).map_err(|e| format!("create repos dir: {e}"))?;
    Ok(dir)
}

/// The clone lives in app_data_dir/repos/<repo name>, so the name is discovered, never hardcoded.
pub fn repo_dir(app: &AppHandle) -> Result<PathBuf, GitError> {
    let cache = &app.state::<crate::AppState>().repo_path;
    let mut cached = cache.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(path) = cached.as_ref().filter(|p| p.join(".git").exists()) {
        return Ok(path.clone());
    }
    let root = repos_root(app)?;
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&root)
        .map_err(|e| GitError::IoError(format!("read repos dir: {e}")))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_dir() && p.join(".git").exists())
        .collect();
    dirs.sort();
    let found = dirs.into_iter().next().ok_or_else(GitError::not_found)?;
    *cached = Some(found.clone());
    Ok(found)
}

/// Warm the clone path cache off the main thread; a missing repo is fine.
pub fn prime_repo_path(app: &AppHandle) {
    let _ = repo_dir(app);
}

fn remember_repo(app: &AppHandle, path: PathBuf) {
    *app.state::<crate::AppState>()
        .repo_path
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = Some(path);
}

fn repo_name_from_url(url: &str) -> Result<String, GitError> {
    let trimmed = url.trim().trim_end_matches('/');
    let name = trimmed.rsplit('/').next().unwrap_or_default();
    let name = name.strip_suffix(".git").unwrap_or(name);
    if name.is_empty() {
        return Err(format!("invalid clone url: {url}").into());
    }
    Ok(name.to_string())
}

fn callbacks(token: Option<String>) -> git2::RemoteCallbacks<'static> {
    let mut cbs = git2::RemoteCallbacks::new();
    cbs.credentials(move |_, _, _| match &token {
        Some(t) => git2::Cred::userpass_plaintext("x-access-token", t),
        None => git2::Cred::default(),
    });
    cbs
}

fn fetch(repo: &Repository, token: Option<String>) -> Result<(), GitError> {
    let mut remote = repo.find_remote("origin").map_err(git_err)?;
    let mut opts = FetchOptions::new();
    opts.remote_callbacks(callbacks(token));
    remote
        .fetch(&[] as &[&str], Some(&mut opts), None)
        .map_err(git_err)
}

fn current_branch(repo: &Repository) -> Result<String, GitError> {
    let head = repo.head().map_err(git_err)?;
    head.shorthand()
        .map(str::to_string)
        .ok_or_else(|| GitError::Other("detached HEAD".into()))
}

fn branch_oids(repo: &Repository) -> (Option<Oid>, Option<Oid>) {
    let Ok(branch) = current_branch(repo) else {
        return (None, None);
    };
    let local = repo.refname_to_id(&format!("refs/heads/{branch}")).ok();
    let remote = repo
        .refname_to_id(&format!("refs/remotes/origin/{branch}"))
        .ok();
    (local, remote)
}

fn status_of(repo: &Repository) -> Result<GitStatus, GitError> {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .exclude_submodules(true);
    let statuses = repo.statuses(Some(&mut opts)).map_err(git_err)?;
    let modified_files: Vec<String> = statuses
        .iter()
        .filter_map(|e| e.path().map(str::to_string))
        .collect();

    let index = repo.index().map_err(git_err)?;
    let mut conflicts = Vec::new();
    if index.has_conflicts() {
        for entry in index.conflicts().map_err(git_err)? {
            let entry = entry.map_err(git_err)?;
            let path = entry
                .our
                .or(entry.their)
                .or(entry.ancestor)
                .and_then(|e| String::from_utf8(e.path).ok())
                .unwrap_or_default();
            if !path.is_empty() {
                conflicts.push(ConflictFile { path });
            }
        }
    }

    let (ahead, behind) = match branch_oids(repo) {
        (Some(local), Some(remote)) => repo.graph_ahead_behind(local, remote).map_err(git_err)?,
        _ => (0, 0),
    };
    Ok(GitStatus {
        synced: ahead == 0 && behind == 0 && modified_files.is_empty() && conflicts.is_empty(),
        modified_files,
        conflicts,
        ahead,
        behind,
        diverged: ahead > 0 && behind > 0,
    })
}

/// Parents recorded in MERGE_HEAD while a merge is waiting for conflict resolution.
fn merge_parents(repo: &Repository) -> Vec<git2::Commit<'_>> {
    let Ok(raw) = std::fs::read_to_string(repo.path().join("MERGE_HEAD")) else {
        return Vec::new();
    };
    raw.lines()
        .filter_map(|line| Oid::from_str(line.trim()).ok())
        .filter_map(|oid| repo.find_commit(oid).ok())
        .collect()
}

/// Commit on HEAD, keeping the merge parent, then clear MERGE_HEAD.
fn commit_head(repo: &Repository, tree: &git2::Tree, message: &str) -> Result<(), GitError> {
    let sig = signature()?;
    let head = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let mut parents: Vec<git2::Commit> = head.into_iter().collect();
    parents.extend(merge_parents(repo));
    let refs: Vec<&git2::Commit> = parents.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, message, tree, &refs)
        .map_err(git_err)?;
    if repo.state() == git2::RepositoryState::Merge {
        repo.cleanup_state().map_err(git_err)?;
    }
    Ok(())
}

fn commit_pending(repo: &Repository, message: Option<&str>) -> Result<(), GitError> {
    let mut index = repo.index().map_err(git_err)?;
    index
        .add_all(["*"], IndexAddOption::DEFAULT, None)
        .map_err(git_err)?;
    // add_all does not stage deletions; update_all does.
    index.update_all(["*"], None).map_err(git_err)?;
    index.write().map_err(git_err)?;
    let tree_id = index.write_tree().map_err(git_err)?;
    let tree = repo.find_tree(tree_id).map_err(git_err)?;
    let parent = repo
        .head()
        .ok()
        .and_then(|h| h.peel_to_commit().ok());
    // A merge commit records the remote parent even when the tree matches HEAD.
    let merging = !merge_parents(repo).is_empty();
    if parent.as_ref().map(|p| p.tree_id()) == Some(tree_id) && !merging {
        return Ok(());
    }
    let message = message
        .map(str::to_string)
        .unwrap_or_else(|| format!("gitpad: sync {}", iso8601_now()));
    commit_head(repo, &tree, &message)
}

/// Stage + commit specific paths (used by fs_write_file for the per-file autosave commit).
pub fn commit_paths(root: &Path, paths: &[PathBuf], message: &str) -> Result<(), GitError> {
    let repo = open_repo(root)?;
    let mut index = repo.index().map_err(git_err)?;
    for path in paths {
        index.add_path(path).map_err(git_err)?;
    }
    index.write().map_err(git_err)?;
    // Mid-merge: only stage; the merge commit happens once every conflict is resolved.
    if repo.state() == git2::RepositoryState::Merge && index.has_conflicts() {
        return Ok(());
    }
    let tree_id = index.write_tree().map_err(git_err)?;
    let tree = repo.find_tree(tree_id).map_err(git_err)?;
    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let merging = !merge_parents(&repo).is_empty();
    if parent.as_ref().map(|p| p.tree_id()) == Some(tree_id) && !merging {
        return Ok(());
    }
    commit_head(&repo, &tree, message)
}

/// Stage everything (including deletions/renames) and commit with `prefix + ISO8601`.
pub fn commit_all(root: &Path, prefix: &str) -> Result<(), GitError> {
    let repo = open_repo(root)?;
    commit_pending(&repo, Some(&format!("{prefix} {}", iso8601_now())))
}

fn pull_repo(repo: &Repository, token: Option<String>) -> Result<GitStatus, GitError> {
    // always commit before pull — reset(Hard) is used below
    commit_pending(repo, None)?;
    fetch(repo, token)?;
    let (Some(local), Some(remote)) = branch_oids(repo) else {
        return status_of(repo);
    };
    let annotated = repo.find_annotated_commit(remote).map_err(git_err)?;
    let (analysis, _) = repo.merge_analysis(&[&annotated]).map_err(git_err)?;
    if analysis.is_fast_forward() {
        let remote_commit = repo.find_commit(remote).map_err(git_err)?;
        repo.reset(remote_commit.as_object(), ResetType::Hard, None)
            .map_err(git_err)?;
    } else if analysis.is_normal() {
        let mut checkout = CheckoutBuilder::new();
        repo.merge(&[&annotated], None, Some(&mut checkout))
            .map_err(git_err)?;
        let mut index = repo.index().map_err(git_err)?;
        if index.has_conflicts() {
            return status_of(repo);
        }
        let tree_id = index.write_tree().map_err(git_err)?;
        let tree = repo.find_tree(tree_id).map_err(git_err)?;
        let sig = signature()?;
        let local_commit = repo.find_commit(local).map_err(git_err)?;
        let remote_commit = repo.find_commit(remote).map_err(git_err)?;
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "gitpad: merge origin",
            &tree,
            &[&local_commit, &remote_commit],
        )
        .map_err(git_err)?;
    }
    status_of(repo)
}

pub fn push_blocking(
    path: &Path,
    message: Option<String>,
    token: Option<String>,
) -> Result<GitStatus, GitError> {
    let repo = open_repo(path)?;
    commit_pending(&repo, message.as_deref())?;
    let status = pull_repo(&repo, token.clone())?;
    if !status.conflicts.is_empty() {
        return Ok(status);
    }
    let branch = current_branch(&repo)?;
    let refspec = format!("refs/heads/{branch}:refs/heads/{branch}");
    let mut remote = repo.find_remote("origin").map_err(git_err)?;
    let mut opts = PushOptions::new();
    opts.remote_callbacks(callbacks(token));
    remote
        .push(&[refspec.as_str()], Some(&mut opts))
        .map_err(|e| format!("git push: {}", e.message()))?;
    status_of(&repo)
}

fn resolve_conflict(root: &Path, path: &str, resolution: Resolution) -> Result<(), GitError> {
    let repo = open_repo(root)?;
    let mut index = repo.index().map_err(git_err)?;
    let target = {
        let mut found = None;
        if index.has_conflicts() {
            for entry in index.conflicts().map_err(git_err)? {
                let entry = entry.map_err(git_err)?;
                let entry_path = entry
                    .our
                    .as_ref()
                    .or(entry.their.as_ref())
                    .or(entry.ancestor.as_ref())
                    .map(|e| String::from_utf8_lossy(&e.path).to_string());
                if entry_path.as_deref() == Some(path) {
                    found = Some(match resolution {
                        Resolution::AcceptOurs => entry.our.map(|e| e.id),
                        Resolution::AcceptTheirs => entry.their.map(|e| e.id),
                    });
                    break;
                }
            }
        }
        found.ok_or_else(|| format!("no conflict for {path}"))?
    };
    let abs = root.join(path);
    match target {
        Some(oid) => {
            let blob = repo.find_blob(oid).map_err(git_err)?;
            std::fs::write(&abs, blob.content()).map_err(|e| format!("write {path}: {e}"))?;
            index.add_path(Path::new(path)).map_err(git_err)?;
        }
        None => {
            let _ = std::fs::remove_file(&abs);
            index.remove_path(Path::new(path)).map_err(git_err)?;
        }
    }
    index.write().map_err(git_err)
}

fn blob_text(repo: &Repository, oid: Oid) -> String {
    repo.find_blob(oid)
        .map(|blob| String::from_utf8_lossy(blob.content()).to_string())
        .unwrap_or_default()
}

/// Both sides of a conflicted path, read from index stages 2 (ours) and 3 (theirs).
fn conflict_versions(root: &Path, path: &str) -> Result<ConflictVersions, GitError> {
    let repo = open_repo(root)?;
    let index = repo.index().map_err(git_err)?;
    if index.has_conflicts() {
        for entry in index.conflicts().map_err(git_err)? {
            let entry = entry.map_err(git_err)?;
            let entry_path = entry
                .our
                .as_ref()
                .or(entry.their.as_ref())
                .or(entry.ancestor.as_ref())
                .map(|e| String::from_utf8_lossy(&e.path).to_string());
            if entry_path.as_deref() != Some(path) {
                continue;
            }
            return Ok(ConflictVersions {
                ours: entry
                    .our
                    .as_ref()
                    .map(|e| blob_text(&repo, e.id))
                    .unwrap_or_default(),
                theirs: entry
                    .their
                    .as_ref()
                    .map(|e| blob_text(&repo, e.id))
                    .unwrap_or_default(),
            });
        }
    }
    Err(format!("no conflict for {path}").into())
}

fn emit_status(app: &AppHandle, status: &GitStatus) -> Result<(), GitError> {
    app.emit("sync-status", status)
        .map_err(|e| format!("emit: {e}"))?;
    if !status.conflicts.is_empty() {
        app.emit("conflict-detected", &status.conflicts)
            .map_err(|e| format!("emit: {e}"))?;
    }
    Ok(())
}

/// Autosave fires on every keystroke; the status bar only needs a few
/// updates per second, so coalesce emits into one per 250 ms.
static LAST_EMIT: std::sync::Mutex<Option<std::time::Instant>> = std::sync::Mutex::new(None);
const EMIT_INTERVAL: std::time::Duration = std::time::Duration::from_millis(250);

pub fn emit_current_status(app: &AppHandle) -> Result<(), GitError> {
    {
        let mut last = LAST_EMIT.lock().unwrap_or_else(|e| e.into_inner());
        if (*last).is_some_and(|t| t.elapsed() < EMIT_INTERVAL) {
            return Ok(());
        }
        *last = Some(std::time::Instant::now());
    }
    let repo = open_repo(&repo_dir(app)?)?;
    emit_status(app, &status_of(&repo)?)
}

pub fn iso8601_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    iso8601(secs)
}

fn iso8601(secs: u64) -> String {
    let (year, month, day) = civil_from_days((secs / 86_400) as i64);
    let rem = secs % 86_400;
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// Howard Hinnant's days-to-civil conversion (no chrono dependency).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if month <= 2 { y + 1 } else { y }, month, day)
}

#[tauri::command]
pub async fn git_clone(app: AppHandle, url: String) -> Result<(), GitError> {
    let name = repo_name_from_url(&url)?;
    let target = repos_root(&app)?.join(name);
    if target.join(".git").exists() {
        remember_repo(&app, target.clone());
        return Ok(());
    }
    if let Ok(existing) = repo_dir(&app) {
        return Err(GitError::Other(format!(
            "a repository is already cloned at {}; remove it before cloning another",
            existing.display()
        )));
    }
    let clone_target = target.clone();
    tokio::task::spawn_blocking(move || {
        let token = auth::token().ok();
        let mut builder = RepoBuilder::new();
        let mut opts = FetchOptions::new();
        opts.remote_callbacks(callbacks(token));
        builder.fetch_options(opts);
        builder.clone(&url, &clone_target).map(|_| ()).map_err(git_err)
    })
    .await
    .map_err(|e| format!("git task: {e}"))??;
    remember_repo(&app, target);
    Ok(())
}

#[tauri::command]
pub async fn git_pull(app: AppHandle) -> Result<GitStatus, GitError> {
    let path = repo_dir(&app)?;
    let status = tokio::task::spawn_blocking(move || {
        // Keyring reads are blocking: keep them off the async runtime.
        let token = auth::token().ok();
        let repo = open_repo(&path)?;
        pull_repo(&repo, token)
    })
    .await
    .map_err(|e| format!("git task: {e}"))??;
    emit_status(&app, &status)?;
    Ok(status)
}

#[tauri::command]
pub async fn git_push(app: AppHandle, message: Option<String>) -> Result<GitStatus, GitError> {
    let path = repo_dir(&app)?;
    let status = tokio::task::spawn_blocking(move || {
        // Keyring reads are blocking: keep them off the async runtime.
        let token = auth::token().ok();
        push_blocking(&path, message, token)
    })
        .await
        .map_err(|e| format!("git task: {e}"))??;
    emit_status(&app, &status)?;
    Ok(status)
}

#[tauri::command]
pub async fn git_status(app: AppHandle) -> Result<GitStatus, GitError> {
    let path = repo_dir(&app)?;
    tokio::task::spawn_blocking(move || {
        let repo = open_repo(&path)?;
        status_of(&repo)
    })
    .await
    .map_err(|e| format!("git task: {e}"))?
}

#[tauri::command]
pub async fn git_conflicts(app: AppHandle) -> Result<Vec<ConflictFile>, GitError> {
    let path = repo_dir(&app)?;
    tokio::task::spawn_blocking(move || {
        let repo = open_repo(&path)?;
        Ok(status_of(&repo)?.conflicts)
    })
    .await
    .map_err(|e| format!("git task: {e}"))?
}

#[tauri::command]
pub async fn git_resolve_conflict(
    app: AppHandle,
    path: String,
    resolution: Resolution,
) -> Result<(), GitError> {
    let root = repo_dir(&app)?;
    tokio::task::spawn_blocking(move || resolve_conflict(&root, &path, resolution))
        .await
        .map_err(|e| format!("git task: {e}"))?
}

#[tauri::command]
pub async fn git_conflict_versions(
    app: AppHandle,
    path: String,
) -> Result<ConflictVersions, GitError> {
    let root = repo_dir(&app)?;
    tokio::task::spawn_blocking(move || conflict_versions(&root, &path))
        .await
        .map_err(|e| format!("git task: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("gitpad-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn commit_file(repo: &Repository, name: &str, content: &str) {
        std::fs::write(repo.workdir().unwrap().join(name), content).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new(name)).unwrap();
        index.write().unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let sig = signature().unwrap();
        let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
        let parents: Vec<&git2::Commit> = parent.iter().collect();
        repo.commit(Some("HEAD"), &sig, &sig, "test", &tree, &parents)
            .unwrap();
    }

    fn push_current(repo: &Repository) {
        let branch = current_branch(repo).unwrap();
        let refspec = format!("refs/heads/{branch}:refs/heads/{branch}");
        let mut remote = repo.find_remote("origin").unwrap();
        remote.push(&[refspec.as_str()], None).unwrap();
    }

    #[test]
    fn iso8601_matches_known_dates() {
        assert_eq!(iso8601(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso8601(1_700_000_000), "2023-11-14T22:13:20Z");
    }

    #[test]
    fn fast_forward_pull_picks_up_remote_changes() {
        let root = tmp("ff");
        let remote_path = root.join("remote.git");
        Repository::init_bare(&remote_path).unwrap();

        let a_path = root.join("a");
        let a = Repository::clone(remote_path.to_str().unwrap(), &a_path).unwrap();
        commit_file(&a, "a.txt", "1");
        push_current(&a);

        let b = Repository::clone(remote_path.to_str().unwrap(), root.join("b")).unwrap();
        commit_file(&b, "b.txt", "2");
        push_current(&b);

        let status = pull_repo(&a, None).unwrap();
        assert!(status.conflicts.is_empty());
        assert_eq!(status.behind, 0);
        assert_eq!(
            std::fs::read_to_string(a_path.join("b.txt")).unwrap(),
            "2"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn diverged_pull_creates_merge_commit() {
        let root = tmp("diverged");
        let remote_path = root.join("remote.git");
        Repository::init_bare(&remote_path).unwrap();

        let a_path = root.join("a");
        let a = Repository::clone(remote_path.to_str().unwrap(), &a_path).unwrap();
        commit_file(&a, "a.txt", "1");
        push_current(&a);

        let b = Repository::clone(remote_path.to_str().unwrap(), root.join("b")).unwrap();
        commit_file(&b, "b.txt", "2");
        push_current(&b);

        commit_file(&a, "c.txt", "3");
        let status = pull_repo(&a, None).unwrap();
        assert!(status.conflicts.is_empty());
        assert_eq!(status.behind, 0);
        assert!(a_path.join("b.txt").exists());
        assert!(a_path.join("c.txt").exists());
        assert_eq!(a.head().unwrap().peel_to_commit().unwrap().parent_count(), 2);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn commit_all_records_deletions() {
        let root = tmp("commit-all");
        let repo = Repository::init(&root).unwrap();
        commit_file(&repo, "note.md", "hello");
        std::fs::remove_file(root.join("note.md")).unwrap();
        commit_all(&root, "gitpad: delete note.md").unwrap();

        let repo = Repository::open(&root).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        assert!(head.tree().unwrap().get_name("note.md").is_none());
        assert!(head.message().unwrap_or("").starts_with("gitpad: delete note.md "));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn conflict_versions_and_merge_commit_keep_both_parents() {
        let root = tmp("conflict");
        let remote_path = root.join("remote.git");
        Repository::init_bare(&remote_path).unwrap();

        let a_path = root.join("a");
        let a = Repository::clone(remote_path.to_str().unwrap(), &a_path).unwrap();
        commit_file(&a, "note.md", "base\n");
        push_current(&a);

        let b = Repository::clone(remote_path.to_str().unwrap(), root.join("b")).unwrap();
        commit_file(&b, "note.md", "theirs\n");
        push_current(&b);

        commit_file(&a, "note.md", "ours\n");
        let status = pull_repo(&a, None).unwrap();
        assert_eq!(status.conflicts.len(), 1);
        assert!(a_path.join(".git/MERGE_HEAD").exists());

        let versions = conflict_versions(&a_path, "note.md").unwrap();
        assert_eq!(versions.ours, "ours\n");
        assert_eq!(versions.theirs, "theirs\n");

        resolve_conflict(&a_path, "note.md", Resolution::AcceptOurs).unwrap();
        commit_pending(&a, Some("gitpad: merge")).unwrap();

        let repo = Repository::open(&a_path).unwrap();
        assert!(!a_path.join(".git/MERGE_HEAD").exists());
        assert_eq!(
            repo.head().unwrap().peel_to_commit().unwrap().parent_count(),
            2
        );
        assert_eq!(
            std::fs::read_to_string(a_path.join("note.md")).unwrap(),
            "ours\n"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn pull_keeps_uncommitted_local_changes() {
        let root = tmp("dirty-pull");
        let remote_path = root.join("remote.git");
        Repository::init_bare(&remote_path).unwrap();

        let a_path = root.join("a");
        let a = Repository::clone(remote_path.to_str().unwrap(), &a_path).unwrap();
        commit_file(&a, "a.txt", "1");
        push_current(&a);

        let b = Repository::clone(remote_path.to_str().unwrap(), root.join("b")).unwrap();
        commit_file(&b, "b.txt", "2");
        push_current(&b);

        std::fs::write(a_path.join("a.txt"), "local edit").unwrap();
        commit_pending(&a, None).unwrap();
        let status = pull_repo(&a, None).unwrap();
        assert!(status.conflicts.is_empty());
        assert_eq!(
            std::fs::read_to_string(a_path.join("a.txt")).unwrap(),
            "local edit"
        );
        assert_eq!(std::fs::read_to_string(a_path.join("b.txt")).unwrap(), "2");
        let _ = std::fs::remove_dir_all(&root);
    }
    #[test]
    fn open_repo_distinguishes_missing_from_io() {
        use std::os::unix::fs::PermissionsExt;

        fn err(path: &Path) -> String {
            match open_repo(path) {
                Ok(_) => String::new(),
                Err(e) => {
                    let value = serde_json::to_value(&e).unwrap();
                    value["code"].as_str().unwrap_or_default().to_string()
                }
            }
        }

        let root = tmp("open-err");
        std::fs::create_dir_all(root.join("empty")).unwrap();
        assert_eq!(err(&root.join("empty")), "NOT_FOUND");

        let locked = root.join("locked");
        std::fs::create_dir_all(locked.join(".git")).unwrap();
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
        assert_eq!(err(&locked), "IO_ERROR");
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
        let _ = std::fs::remove_dir_all(&root);
    }


    #[test]
    fn git_error_serializes_with_code_and_message() {
        let value = serde_json::to_value(GitError::NotFound("no repository cloned".into())).unwrap();
        assert_eq!(value["code"], "NOT_FOUND");
        assert_eq!(value["message"], "no repository cloned");
        let io = serde_json::to_value(GitError::IoError("denied".into())).unwrap();
        assert_eq!(io["code"], "IO_ERROR");
        assert_eq!(io["message"], "denied");
    }
}
