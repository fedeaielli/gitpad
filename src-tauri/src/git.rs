use crate::auth;
use git2::{
    build::{CheckoutBuilder, RepoBuilder}, Commit, Diff, FetchOptions, IndexAddOption, Oid,
    PushOptions, Repository, ResetType, Signature, Sort, StatusOptions,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_log::log;

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
pub struct FileChange {
    pub path: String,
    /// Porcelain-style code: "M", "A" (untracked or staged new) or "D".
    pub status: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GitStatus {
    pub synced: bool,
    pub branch: String,
    pub changes: Vec<FileChange>,
    pub modified_files: Vec<String>,
    pub conflicts: Vec<ConflictFile>,
    pub ahead: usize,
    pub behind: usize,
    pub diverged: bool,
    /// Epoch seconds of the last push GitPad made from this clone.
    pub last_push: Option<u64>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CommitEntry {
    pub hash: String,
    pub short_hash: String,
    pub message: String,
    pub author: String,
    pub timestamp: u64,
    pub files_changed: usize,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileDiff {
    pub original: String,
    pub modified: String,
}

const LOG_LIMIT: usize = 100;
const FILE_LOG_LIMIT: usize = 50;
/// A file untouched for thousands of commits would make the walk unbounded.
const FILE_LOG_SCAN: usize = 1000;

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
    // A missing ref is a normal probe (unborn HEAD, empty history), not a fault.
    if e.code() != git2::ErrorCode::NotFound {
        log::warn!(target: "git", "git: {}", e.message());
    }
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

/// Where a fresh clone goes when the wizard picked no folder: next to the
/// user's documents, because a hidden app-data folder is a bad place for notes.
pub fn default_clone_dir(app: &AppHandle) -> Result<PathBuf, GitError> {
    let path = app.path();
    Ok(pick_clone_dir(
        path.document_dir().ok(),
        path.home_dir().ok(),
        repos_root(app)?,
    ))
}

/// Documents, else home, else the old app-data folder: never a hidden one when
/// a visible one exists. A missing document dir is normal (no XDG entry, a
/// container, a removed folder), so each candidate must be a real directory.
fn pick_clone_dir(
    documents: Option<PathBuf>,
    home: Option<PathBuf>,
    fallback: PathBuf,
) -> PathBuf {
    documents
        .filter(|p| p.is_dir())
        .or(home.filter(|p| p.is_dir()))
        .unwrap_or(fallback)
}

/// The clone's name is discovered, never hardcoded: a location written by a
/// clone wins, and the app-data scan only finds clones made before the default
/// moved next to the user's documents.
pub fn repo_dir(app: &AppHandle) -> Result<PathBuf, GitError> {
    let cache = &app.state::<crate::AppState>().repo_path;
    let mut cached = cache.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(path) = cached.as_ref().filter(|p| p.join(".git").exists()) {
        return Ok(path.clone());
    }
    // A location picked by the user wins over scanning the default folder.
    if let Some(path) = crate::config::configured_repo_path(app).filter(|p| p.join(".git").exists())
    {
        *cached = Some(path.clone());
        return Ok(path);
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
    match repo_dir(app) {
        Ok(path) => log::info!(target: "git", "repository: {}", path.display()),
        Err(_) => log::info!(target: "git", "repository: none yet"),
    }
}

fn remember_repo(app: &AppHandle, path: PathBuf) {
    *app.state::<crate::AppState>()
        .repo_path
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = Some(path.clone());
    // Every path change goes through here: one call site, never forgotten.
    // A watch that fails to start is not fatal, the tree still refreshes on focus.
    if let Err(e) = crate::watcher::ensure(app, &path) {
        log::warn!(target: "app", "watcher: {e}");
    }
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

fn commit_tree_diff<'r>(repo: &'r Repository, commit: &Commit<'_>) -> Result<Diff<'r>, GitError> {
    let tree = commit.tree().map_err(git_err)?;
    let parent_tree = match commit.parent(0) {
        Ok(parent) => Some(parent.tree().map_err(git_err)?),
        Err(_) => None, // root commit: everything shows as added
    };
    repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)
        .map_err(git_err)
}

fn entry_of(hash: String, commit: &Commit<'_>, files_changed: usize) -> CommitEntry {
    CommitEntry {
        short_hash: hash[..7].to_string(),
        hash,
        message: commit.summary().unwrap_or("(no message)").to_string(),
        author: commit.author().name().unwrap_or("unknown").to_string(),
        timestamp: commit.time().seconds().max(0) as u64,
        files_changed,
    }
}

fn log_entries(repo: &Repository) -> Result<Vec<CommitEntry>, GitError> {
    let mut walk = repo.revwalk().map_err(git_err)?;
    walk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME)
        .map_err(git_err)?;
    walk.push_head().map_err(git_err)?;
    let mut entries = Vec::new();
    for oid in walk.take(LOG_LIMIT) {
        let oid = oid.map_err(git_err)?;
        let commit = repo.find_commit(oid).map_err(git_err)?;
        let files_changed = commit_tree_diff(repo, &commit)?.deltas().len();
        entries.push(entry_of(oid.to_string(), &commit, files_changed));
    }
    Ok(entries)
}

fn file_log(repo: &Repository, rel: &Path) -> Result<Vec<CommitEntry>, GitError> {
    let mut walk = repo.revwalk().map_err(git_err)?;
    walk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME)
        .map_err(git_err)?;
    walk.push_head().map_err(git_err)?;
    let mut entries = Vec::new();
    // shortcut: only the newest FILE_LOG_SCAN commits are inspected.
    for oid in walk.take(FILE_LOG_SCAN) {
        let oid = oid.map_err(git_err)?;
        let commit = repo.find_commit(oid).map_err(git_err)?;
        let diff = commit_tree_diff(repo, &commit)?;
        let touched = diff
            .deltas()
            .any(|d| d.new_file().path() == Some(rel) || d.old_file().path() == Some(rel));
        if !touched {
            continue;
        }
        let files_changed = diff.deltas().len();
        entries.push(entry_of(oid.to_string(), &commit, files_changed));
        if entries.len() >= FILE_LOG_LIMIT {
            break;
        }
    }
    Ok(entries)
}

fn content_at(repo: &Repository, tree: &git2::Tree<'_>, rel: &Path) -> String {
    tree.get_path(rel)
        .ok()
        .and_then(|entry| entry.to_object(repo).ok())
        .and_then(|object| {
            object
                .as_blob()
                .map(|b| String::from_utf8_lossy(b.content()).to_string())
        })
        .unwrap_or_default()
}

fn file_diff(repo: &Repository, hash: &str, rel: &Path) -> Result<FileDiff, GitError> {
    let oid = Oid::from_str(hash)
        .map_err(|e| GitError::Other(format!("bad commit {hash}: {e}")))?;
    let commit = repo.find_commit(oid).map_err(git_err)?;
    let modified = content_at(repo, &commit.tree().map_err(git_err)?, rel);
    let original = match commit.parent(0) {
        Ok(parent) => content_at(repo, &parent.tree().map_err(git_err)?, rel),
        Err(_) => String::new(), // root commit: nothing before it
    };
    Ok(FileDiff { original, modified })
}

fn delta_code(delta: git2::Delta) -> &'static str {
    match delta {
        git2::Delta::Added | git2::Delta::Untracked => "A",
        git2::Delta::Deleted => "D",
        _ => "M", // Modified, Renamed, Copied, TypeChange
    }
}

fn commit_files(repo: &Repository, hash: &str) -> Result<Vec<FileChange>, GitError> {
    let oid = Oid::from_str(hash)
        .map_err(|e| GitError::Other(format!("bad commit {hash}: {e}")))?;
    let commit = repo.find_commit(oid).map_err(git_err)?;
    let diff = commit_tree_diff(repo, &commit)?;
    Ok(diff
        .deltas()
        .filter_map(|delta| {
            let path = delta.new_file().path()?.to_string_lossy().to_string();
            Some(FileChange {
                path,
                status: delta_code(delta.status()).to_string(),
            })
        })
        .collect())
}

fn restore_file(
    repo: &Repository,
    hash: &str,
    rel: &Path,
    gate: &std::sync::RwLock<()>,
) -> Result<GitStatus, GitError> {
    let _gate = gate.write().map_err(|e| e.to_string())?;
    let oid = Oid::from_str(hash)
        .map_err(|e| GitError::Other(format!("bad commit {hash}: {e}")))?;
    let commit = repo.find_commit(oid).map_err(git_err)?;
    let short = &hash[..7.min(hash.len())];
    if commit.tree().map_err(git_err)?.get_path(rel).is_err() {
        return Err(GitError::Other(format!(
            "{} does not exist in {short}",
            rel.display()
        )));
    }
    let mut checkout = CheckoutBuilder::new();
    checkout.path(rel).force();
    repo.checkout_tree(commit.as_object(), Some(&mut checkout))
        .map_err(git_err)?;
    status_of(repo)
}

fn revert_commit(
    repo: &Repository,
    hash: &str,
    gate: &std::sync::RwLock<()>,
) -> Result<GitStatus, GitError> {
    let _gate = gate.write().map_err(|e| e.to_string())?;
    let oid = Oid::from_str(hash)
        .map_err(|e| GitError::Other(format!("bad commit {hash}: {e}")))?;
    let commit = repo.find_commit(oid).map_err(git_err)?;
    if commit.parent_count() > 1 {
        return Err(GitError::Other("cannot revert a merge commit".into()));
    }
    if commit.parent_count() == 0 {
        return Err(GitError::Other("cannot revert the first commit".into()));
    }
    repo.revert(&commit, None).map_err(git_err)?;
    if repo.index().map_err(git_err)?.has_conflicts() {
        // Conflicts stay in the index; the modal already handles them.
        return status_of(repo);
    }
    let short = &hash[..7.min(hash.len())];
    commit_pending(repo, Some(&format!("gitpad: revert {short}")))?;
    status_of(repo)
}

fn change_code(status: git2::Status) -> &'static str {
    if status.is_wt_new() || status.is_index_new() {
        "A"
    } else if status.is_wt_deleted() || status.is_index_deleted() {
        "D"
    } else {
        "M"
    }
}

fn status_of(repo: &Repository) -> Result<GitStatus, GitError> {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .exclude_submodules(true);
    let statuses = repo.statuses(Some(&mut opts)).map_err(git_err)?;
    let changes: Vec<FileChange> = statuses
        .iter()
        .filter_map(|entry| {
            let path = entry.path()?.to_string();
            Some(FileChange {
                path,
                status: change_code(entry.status()).to_string(),
            })
        })
        .collect();
    let modified_files: Vec<String> = changes.iter().map(|c| c.path.clone()).collect();

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
    let branch = current_branch(repo).unwrap_or_else(|_| "HEAD".into());
    Ok(GitStatus {
        synced: ahead == 0 && behind == 0 && modified_files.is_empty() && conflicts.is_empty(),
        branch,
        changes,
        modified_files,
        conflicts,
        ahead,
        behind,
        diverged: ahead > 0 && behind > 0,
        last_push: last_push_secs(repo),
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
    log::info!(target: "git", "commit: {message}");
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

/// Paths whose content a pull changed, so open tabs reload instead of
/// overwriting the new version at the next autosave.
fn changed_paths(repo: &Repository, before: Option<Oid>) -> Result<Vec<FileChange>, GitError> {
    let after = repo.head().ok().and_then(|head| head.target());
    let Some(after) = after else {
        return Ok(Vec::new());
    };
    if before == Some(after) {
        return Ok(Vec::new());
    }
    let after_tree = repo.find_commit(after).map_err(git_err)?.tree().map_err(git_err)?;
    let before_tree = match before {
        Some(oid) => Some(repo.find_commit(oid).map_err(git_err)?.tree().map_err(git_err)?),
        None => None,
    };
    let diff = repo
        .diff_tree_to_tree(before_tree.as_ref(), Some(&after_tree), None)
        .map_err(git_err)?;
    Ok(diff
        .deltas()
        .filter_map(|delta| {
            let path = delta
                .new_file()
                .path()
                .or_else(|| delta.old_file().path())?;
            Some(FileChange {
                path: path.to_string_lossy().to_string(),
                status: delta_code(delta.status()).to_string(),
            })
        })
        .collect())
}

fn pull_repo_with(
    repo: &Repository,
    token: Option<String>,
    gate: &std::sync::RwLock<()>,
    message: Option<&str>,
) -> Result<(GitStatus, Vec<FileChange>), GitError> {
    // refs only: no worktree writes, readers must not wait for the network
    // Local-only repositories have no origin: nothing to fetch.
    if repo.find_remote("origin").is_ok() {
        fetch(repo, token)?;
    }
    // The gate covers commit + reset/merge: a read in between would see a
    // truncated file.
    let _gate = gate.write().map_err(|e| e.to_string())?;
    // Committed under the lock so a write that slips in before the reset is
    // not discarded by reset(Hard).
    commit_pending(repo, message)?;
    let before = repo.head().ok().and_then(|head| head.target());
    let (Some(local), Some(remote)) = branch_oids(repo) else {
        log::info!(target: "git", "pull: local only, nothing to fetch");
        return Ok((status_of(repo)?, Vec::new()));
    };
    let annotated = repo.find_annotated_commit(remote).map_err(git_err)?;
    let (analysis, _) = repo.merge_analysis(&[&annotated]).map_err(git_err)?;
    if analysis.is_fast_forward() {
        let remote_commit = repo.find_commit(remote).map_err(git_err)?;
        repo.reset(remote_commit.as_object(), ResetType::Hard, None)
            .map_err(git_err)?;
        log::info!(target: "git", "pull: fast-forward");
    } else if analysis.is_normal() {
        let mut checkout = CheckoutBuilder::new();
        repo.merge(&[&annotated], None, Some(&mut checkout))
            .map_err(git_err)?;
        let mut index = repo.index().map_err(git_err)?;
        if index.has_conflicts() {
            // HEAD did not move, but the merge rewrote the worktree: report
            // every changed path, conflicts included.
            let status = status_of(repo)?;
            log::warn!(target: "git", "pull: {} conflict(s)", status.conflicts.len());
            let changes = status.changes.clone();
            return Ok((status, changes));
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
        log::info!(target: "git", "pull: merged origin");
    } else {
        log::info!(target: "git", "pull: up to date");
    }
    let status = status_of(repo)?;
    let changes = changed_paths(repo, before)?;
    Ok((status, changes))
}

fn pull_repo(
    repo: &Repository,
    token: Option<String>,
    gate: &std::sync::RwLock<()>,
) -> Result<(GitStatus, Vec<FileChange>), GitError> {
    pull_repo_with(repo, token, gate, None)
}

pub fn push_blocking(
    path: &Path,
    message: Option<String>,
    token: Option<String>,
    gate: std::sync::Arc<std::sync::RwLock<()>>,
) -> Result<(GitStatus, Vec<FileChange>, Option<GitError>), GitError> {
    let repo = open_repo(path)?;
    let (status, changes) = pull_repo_with(&repo, token.clone(), &gate, message.as_deref())?;
    if !status.conflicts.is_empty() {
        return Ok((status, changes, None));
    }
    // The pull already rewrote the worktree: report it even when the push fails.
    match push_refs(&repo, token) {
        Ok(()) => Ok((status_of(&repo)?, changes, None)),
        Err(e) => Ok((status, changes, Some(e))),
    }
}

/// How `push_only_blocking` waits for the worktree gate.
pub enum GateWait<'a> {
    /// Wait as long as the operation takes (manual push).
    Block(&'a std::sync::RwLock<()>),
    /// Give up after `wait` and skip the commit and push (closing paths).
    Until(&'a std::sync::RwLock<()>, std::time::Duration),
}

/// `RwLock::write` has no timeout: poll it until the deadline.
fn try_write_until<'a>(
    gate: &'a std::sync::RwLock<()>,
    wait: std::time::Duration,
) -> Option<std::sync::RwLockWriteGuard<'a, ()>> {
    let deadline = std::time::Instant::now() + wait;
    loop {
        match gate.try_write() {
            Ok(guard) => return Some(guard),
            // A poisoned gate must not keep the app from closing: take it anyway.
            Err(std::sync::TryLockError::Poisoned(e)) => return Some(e.into_inner()),
            Err(std::sync::TryLockError::WouldBlock) => {
                if std::time::Instant::now() >= deadline {
                    return None;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    }
}

/// Commit and push without pulling first (used by the push-only button).
/// Returns the current status without committing or pushing when an `Until`
/// gate never frees up: every autosave is already committed, and the next
/// start pushes whatever is left.
pub fn push_only_blocking(
    path: &Path,
    message: Option<String>,
    token: Option<String>,
    gate: GateWait<'_>,
) -> Result<GitStatus, GitError> {
    let repo = open_repo(path)?;
    let (local, remote) = branch_oids(&repo);
    if let (Some(local), Some(remote)) = (local, remote) {
        // Nothing to commit and nothing to send: skip the network round-trip.
        let status = status_of(&repo)?;
        if local == remote && status.changes.is_empty() {
            return Ok(status);
        }
    }
    // The gate covers the commit only: the network must not hold it.
    let guard = match gate {
        GateWait::Block(gate) => Some(gate.write().map_err(|e| e.to_string())?),
        GateWait::Until(gate, wait) => match try_write_until(gate, wait) {
            Some(guard) => Some(guard),
            None => return status_of(&repo),
        },
    };
    commit_pending(&repo, message.as_deref())?;
    drop(guard);
    push_refs(&repo, token)?;
    status_of(&repo)
}

fn push_refs(repo: &Repository, token: Option<String>) -> Result<(), GitError> {
    // A local-only repository keeps its commits; there is no remote to send to.
    if repo.find_remote("origin").is_err() {
        return Ok(());
    }
    let branch = current_branch(repo)?;
    let refspec = format!("refs/heads/{branch}:refs/heads/{branch}");
    let mut remote = repo.find_remote("origin").map_err(git_err)?;
    let mut opts = PushOptions::new();
    opts.remote_callbacks(callbacks(token));
    remote
        .push(&[refspec.as_str()], Some(&mut opts))
        .map_err(|e| {
            log::warn!(target: "git", "push: {}", e.message());
            format!("git push: {}", e.message())
        })?;
    log::info!(target: "git", "push: {branch}");
    record_push(repo);
    Ok(())
}

/// The timestamp lives in the clone's own config: it survives restarts and
/// moves with the clone, without ever being pushed to GitHub.
fn record_push(repo: &Repository) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    if let Ok(mut config) = repo.config() {
        let _ = config.set_i64("gitpad.last-push", now);
    }
}

fn last_push_secs(repo: &Repository) -> Option<u64> {
    let value = repo.config().ok()?.get_i64("gitpad.last-push").ok()?;
    (value > 0).then_some(value as u64)
}

fn resolve_conflict(
    root: &Path,
    path: &str,
    resolution: Resolution,
) -> Result<(GitStatus, Vec<FileChange>), GitError> {
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
            let tmp = repo.path().join("gitpad-resolve.tmp");
            crate::fs::write_atomic(&abs, &tmp, blob.content())
                .map_err(|e| format!("write {path}: {e}"))?;
            index.add_path(Path::new(path)).map_err(git_err)?;
        }
        None => {
            let _ = std::fs::remove_file(&abs);
            index.remove_path(Path::new(path)).map_err(git_err)?;
        }
    }
    index.write().map_err(git_err)?;
    drop(index);
    let status = status_of(&repo)?;
    let changes = vec![FileChange {
        path: path.to_string(),
        status: "M".to_string(),
    }];
    Ok((status, changes))
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

/// The clone always lands in `<parent>/<repo name>`: the picker only chooses the folder around it.
fn clone_target(parent: &Path, name: &str) -> Result<PathBuf, GitError> {
    if !parent.is_absolute() {
        return Err(format!("clone folder must be absolute: {}", parent.display()).into());
    }
    let meta = std::fs::metadata(parent)
        .map_err(|e| GitError::IoError(format!("clone folder {}: {e}", parent.display())))?;
    if !meta.is_dir() {
        return Err(format!("clone folder is not a folder: {}", parent.display()).into());
    }
    Ok(parent.join(name))
}

/// Recursive copy for moving a clone across filesystems; symlinks are skipped.
fn copy_tree(src: &Path, dst: &Path) -> Result<(), GitError> {
    let io = |what: &Path, e: std::io::Error| GitError::IoError(format!("{}: {e}", what.display()));
    std::fs::create_dir_all(dst).map_err(|e| io(dst, e))?;
    for entry in std::fs::read_dir(src).map_err(|e| io(src, e))? {
        let entry = entry.map_err(|e| io(src, e))?;
        let path = entry.path();
        let target = dst.join(entry.file_name());
        let kind = entry.file_type().map_err(|e| io(&path, e))?;
        if kind.is_dir() {
            copy_tree(&path, &target)?;
        } else if kind.is_file() {
            std::fs::copy(&path, &target).map_err(|e| io(&path, e))?;
        }
    }
    Ok(())
}

/// Rename when possible; otherwise copy to a sibling folder, verify it and swap.
/// The source stays until the destination is verified, so a failure leaves no
/// half-moved clone behind.
fn move_clone(from: &Path, to: &Path) -> Result<(), GitError> {
    if to.exists() {
        return Err(format!("a folder already exists at {}", to.display()).into());
    }
    // Same filesystem: one syscall. The rename already happened, so a clone
    // that does not open is a warning, not a failed move.
    if std::fs::rename(from, to).is_ok() {
        if let Err(e) = open_repo(to) {
            log::warn!(
                target: "git",
                "move: {} does not open after the move: {e}",
                to.display()
            );
        }
        return Ok(());
    }
    // Other filesystems: stage a verified copy next to the destination first.
    stage_copy(from, to)?;
    if let Err(e) = std::fs::remove_dir_all(from) {
        // The clone is already verified at its destination: a leftover copy is
        // better than failing a move that succeeded.
        log::warn!(target: "git", "move: leaving {} in place: {e}", from.display());
    }
    Ok(())
}

/// Cross-filesystem half of `move_clone`: copy, verify, then swap. Nothing is
/// written directly into `to`, so a failure never leaves a partial clone there.
fn stage_copy(from: &Path, to: &Path) -> Result<(), GitError> {
    let parent = to
        .parent()
        .ok_or_else(|| GitError::Other(format!("no parent folder for {}", to.display())))?;
    let name = to
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .ok_or_else(|| GitError::Other(format!("no name in {}", to.display())))?;
    let tmp = parent.join(format!(".{name}.move-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    let staged = copy_tree(from, &tmp)
        .and_then(|_| verify_copy(from, &tmp))
        .and_then(|_| {
            std::fs::rename(&tmp, to)
                .map_err(|e| GitError::IoError(format!("move into {}: {e}", to.display())))
        });
    if let Err(e) = staged {
        // A partial copy must not block the next attempt.
        let _ = std::fs::remove_dir_all(&tmp);
        return Err(e);
    }
    Ok(())
}

/// A copy is trustworthy only if the commit and the file count match.
fn verify_copy(from: &Path, to: &Path) -> Result<(), GitError> {
    let source = open_repo(from)?;
    let copy = open_repo(to)?;
    if head_oid(&source) != head_oid(&copy) {
        return Err(GitError::Other("copy does not match the clone".into()));
    }
    if count_files(from)? != count_files(to)? {
        return Err(GitError::Other("copy is missing files".into()));
    }
    Ok(())
}

fn head_oid(repo: &Repository) -> Option<git2::Oid> {
    repo.head().ok().and_then(|head| head.target())
}

/// Regular files only: `copy_tree` skips symlinks, so they never reach the copy.
fn count_files(dir: &Path) -> Result<usize, GitError> {
    let io = |path: &Path, e: std::io::Error| GitError::IoError(format!("{}: {e}", path.display()));
    let mut total = 0;
    for entry in std::fs::read_dir(dir).map_err(|e| io(dir, e))? {
        let entry = entry.map_err(|e| io(dir, e))?;
        let path = entry.path();
        let kind = entry.file_type().map_err(|e| io(&path, e))?;
        if kind.is_dir() {
            total += count_files(&path)?;
        } else if kind.is_file() {
            total += 1;
        }
    }
    Ok(total)
}

/// A new clone may only land where nothing is already versioned.
fn fresh_clone_target(parent: &Path, name: &str) -> Result<PathBuf, GitError> {
    let target = clone_target(parent, name)?;
    if target.join(".git").exists() {
        return Err(format!("a repository is already cloned at {}", target.display()).into());
    }
    Ok(target)
}

#[tauri::command]
pub async fn git_clone(
    app: AppHandle,
    url: String,
    parent: Option<String>,
) -> Result<String, GitError> {
    let name = repo_name_from_url(&url)?;
    let target = match parent.as_deref().map(str::trim).filter(|p| !p.is_empty()) {
        Some(parent) => clone_target(Path::new(parent), &name)?,
        None => default_clone_dir(&app)?.join(name),
    };
    if target.join(".git").exists() {
        remember_repo(&app, target.clone());
        return Ok(target.to_string_lossy().into_owned());
    }
    if let Ok(existing) = repo_dir(&app) {
        return Err(GitError::Other(format!(
            "a repository is already cloned at {}; remove it before cloning another",
            existing.display()
        )));
    }
    clone_into(&app, &url, &target).await?;
    remember_repo(&app, target.clone());
    Ok(target.to_string_lossy().into_owned())
}

/// Clone another repository and make it the active one; the old clone stays on disk.
#[tauri::command]
pub async fn git_import_repo(app: AppHandle, url: String, parent: String) -> Result<MovedRepo, GitError> {
    // Empty when nothing was configured yet: the caller then has no tabs to close.
    let from = repo_dir(&app)
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_default();
    let name = repo_name_from_url(&url)?;
    let target = fresh_clone_target(Path::new(parent.trim()), &name)?;
    clone_into(&app, &url, &target).await?;
    remember_repo(&app, target.clone());
    log::info!(target: "git", "repository: {} (imported)", target.display());
    if let Err(e) = crate::config::store_repo_path(&app, &target) {
        log::warn!(target: "git", "import: could not save the new location: {e}");
    }
    Ok(MovedRepo {
        from,
        to: target.to_string_lossy().into_owned(),
    })
}

/// Adopt a folder as the active repository, initializing it when needed.
/// Returns the canonical path, whether `.git` was created, and whether an
/// `origin` remote exists. Sync so the tests can call it without an AppHandle.
fn use_local_repo(path: &Path) -> Result<(PathBuf, bool, bool), GitError> {
    let path = path
        .canonicalize()
        .map_err(|e| GitError::IoError(format!("{}: {e}", path.display())))?;
    if !path.is_dir() {
        return Err(GitError::Other(format!(
            "{} is not a folder",
            path.display()
        )));
    }
    let initialized = !path.join(".git").exists();
    if initialized {
        Repository::init(&path).map_err(git_err)?;
    }
    let repo = open_repo(&path)?;
    let remote = repo.find_remote("origin").is_ok();
    Ok((path, initialized, remote))
}

/// A folder the user picked, and what it turned out to be.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalRepo {
    /// Previous active repository, empty when none was configured yet.
    pub from: String,
    pub to: String,
    pub initialized: bool,
    pub remote: bool,
}

/// Use `path` as the repository: an existing one is opened, a plain folder is
/// initialized in place. Switching closes the caller's tabs, not ours.
#[tauri::command]
pub async fn git_use_local(app: AppHandle, path: String) -> Result<LocalRepo, GitError> {
    let from = repo_dir(&app)
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_default();
    let gate = app.state::<crate::AppState>().repo_gate.clone();
    let chosen = PathBuf::from(path.trim());
    let (to, initialized, remote) = tokio::task::spawn_blocking(move || {
        let _gate = gate.write().map_err(|e| e.to_string())?;
        use_local_repo(&chosen)
    })
    .await
    .map_err(|e| format!("git task: {e}"))??;
    // repo_dir reads the cache first, so it must learn the new path.
    remember_repo(&app, to.clone());
    log::info!(
        target: "git",
        "repository: {} ({})",
        to.display(),
        if remote { "with origin" } else { "local only" }
    );
    if let Err(e) = crate::config::store_repo_path(&app, &to) {
        log::warn!(target: "git", "use_local: could not save the new location: {e}");
    }
    Ok(LocalRepo {
        from,
        to: to.to_string_lossy().into_owned(),
        initialized,
        remote,
    })
}

/// Shared clone body: the caller owns target resolution and the repo cache.
async fn clone_into(app: &AppHandle, url: &str, target: &Path) -> Result<(), GitError> {
    let url = url.to_string();
    let target_task = target.to_path_buf();
    let gate = app.state::<crate::AppState>().repo_gate.clone();
    tokio::task::spawn_blocking(move || {
        log::info!(target: "git", "clone: {url}");
        let token = auth::token().ok();
        let _gate = gate.write().map_err(|e| e.to_string())?;
        let result = clone_repo(&url, &target_task, token);
        if result.is_ok() {
            log::info!(target: "git", "clone: done");
        }
        result
    })
    .await
    .map_err(|e| format!("git task: {e}"))?
}

/// Sync clone body: the caller owns the gate, the token and the target.
fn clone_repo(url: &str, target: &Path, token: Option<String>) -> Result<(), GitError> {
    let mut builder = RepoBuilder::new();
    let mut opts = FetchOptions::new();
    opts.remote_callbacks(callbacks(token));
    builder.fetch_options(opts);
    builder.clone(url, target).map(|_| ()).map_err(git_err)
}

/// Where a move took the clone: open tabs need the old prefix to follow it.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MovedRepo {
    pub from: String,
    pub to: String,
}

/// Moves the clone into another folder and returns its old and new path.
#[tauri::command]
pub async fn git_move_repo(app: AppHandle, parent: String) -> Result<MovedRepo, GitError> {
    let from = repo_dir(&app)?;
    let name = from
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .ok_or_else(|| GitError::Other("repository has no name".into()))?;
    let to = clone_target(Path::new(parent.trim()), &name)?;
    if to == from {
        let path = from.to_string_lossy().into_owned();
        return Ok(MovedRepo { from: path.clone(), to: path });
    }
    if to.exists() {
        return Err(format!("a folder already exists at {}", to.display()).into());
    }
    let gate = app.state::<crate::AppState>().repo_gate.clone();
    let from_task = from.clone();
    let to_task = to.clone();
    tokio::task::spawn_blocking(move || -> Result<(), GitError> {
        let _gate = gate.write().map_err(|e| e.to_string())?;
        move_clone(&from_task, &to_task)
    })
    .await
    .map_err(|e| format!("git task: {e}"))??;
    remember_repo(&app, to.clone());
    log::info!(target: "git", "repository: {} (moved from {})", to.display(), from.display());
    if let Err(e) = crate::config::store_repo_path(&app, &to) {
        log::warn!(target: "git", "move: could not save the new location: {e}");
    }
    Ok(MovedRepo {
        from: from.to_string_lossy().into_owned(),
        to: to.to_string_lossy().into_owned(),
    })
}

#[tauri::command]
pub async fn git_pull(app: AppHandle) -> Result<GitStatus, GitError> {
    let path = repo_dir(&app)?;
    let gate = app.state::<crate::AppState>().repo_gate.clone();
    let (status, changes) = tokio::task::spawn_blocking(move || {
        // Keyring reads are blocking: keep them off the async runtime.
        let token = auth::token().ok();
        let repo = open_repo(&path)?;
        pull_repo(&repo, token, &gate)
    })
    .await
    .map_err(|e| format!("git task: {e}"))??;
    if !changes.is_empty() {
        let _ = app.emit("pull-changed", &changes);
    }
    emit_status(&app, &status)?;
    Ok(status)
}

#[tauri::command]
pub async fn git_push(
    app: AppHandle,
    message: Option<String>,
    pull: Option<bool>,
) -> Result<GitStatus, GitError> {
    let path = repo_dir(&app)?;
    let gate = app.state::<crate::AppState>().repo_gate.clone();
    let (status, changes, push_error) = tokio::task::spawn_blocking(move || {
        // Keyring reads are blocking: keep them off the async runtime.
        let token = auth::token().ok();
        if pull == Some(false) {
            push_only_blocking(&path, message, token, GateWait::Block(&gate))
                .map(|status| (status, Vec::new(), None))
        } else {
            push_blocking(&path, message, token, gate)
        }
    })
    .await
    .map_err(|e| format!("git task: {e}"))??;
    // The worktree already changed: reload tabs before the push error surfaces.
    if !changes.is_empty() {
        let _ = app.emit("pull-changed", &changes);
    }
    emit_status(&app, &status)?;
    if let Some(e) = push_error {
        return Err(e);
    }
    Ok(status)
}

/// Relative paths only: a discard must stay inside the repository.
fn safe_rel_path(path: &str) -> Result<PathBuf, GitError> {
    let candidate = Path::new(path);
    if candidate.is_absolute()
        || candidate
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(GitError::Other(format!("path outside repository: {path}")));
    }
    Ok(candidate.to_path_buf())
}

fn discard_file(
    repo: &Repository,
    rel: &Path,
    gate: &std::sync::RwLock<()>,
) -> Result<(), GitError> {
    let _gate = gate.write().map_err(|e| e.to_string())?;
    let index = repo.index().map_err(git_err)?;
    // A conflict lives in stages 1-3, never 0: discarding would delete a file
    // the merge still needs. Resolution belongs to the conflict dialog.
    if (1..=3).any(|stage| index.get_path(rel, stage).is_some()) {
        return Err(GitError::Other(format!(
            "{} has a merge conflict: resolve it in the conflict dialog",
            rel.display()
        )));
    }
    let in_index = index.get_path(rel, 0).is_some();
    drop(index);
    // A staged deletion/rename is already gone from stage 0 but still in HEAD.
    let in_head = repo
        .head()
        .ok()
        .and_then(|head| head.peel_to_tree().ok())
        .map(|tree| tree.get_path(rel).is_ok())
        .unwrap_or(false);
    if in_index || in_head {
        let mut checkout = CheckoutBuilder::new();
        checkout.path(rel).force();
        repo.checkout_head(Some(&mut checkout)).map_err(git_err)
    } else {
        let workdir = repo
            .workdir()
            .ok_or_else(|| GitError::IoError("bare repository".into()))?;
        match std::fs::remove_file(workdir.join(rel)) {
            Ok(()) => Ok(()),
            // Another tool may have removed it first: already discarded.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(GitError::IoError(format!("remove {}: {e}", rel.display()))),
        }
    }
}

#[tauri::command]
pub async fn git_discard_file(app: AppHandle, path: String) -> Result<GitStatus, GitError> {
    let root = repo_dir(&app)?;
    let rel = safe_rel_path(&path)?;
    let gate = app.state::<crate::AppState>().repo_gate.clone();
    let status = tokio::task::spawn_blocking(move || {
        let repo = open_repo(&root)?;
        discard_file(&repo, &rel, &gate)?;
        status_of(&repo)
    })
    .await
    .map_err(|e| format!("git task: {e}"))??;
    emit_status(&app, &status)?;
    Ok(status)
}

#[tauri::command]
pub async fn git_log(app: AppHandle) -> Result<Vec<CommitEntry>, GitError> {
    let path = repo_dir(&app)?;
    tokio::task::spawn_blocking(move || log_entries(&open_repo(&path)?))
        .await
        .map_err(|e| format!("git task: {e}"))?
}

#[tauri::command]
pub async fn git_commit_files(app: AppHandle, hash: String) -> Result<Vec<FileChange>, GitError> {
    let path = repo_dir(&app)?;
    tokio::task::spawn_blocking(move || commit_files(&open_repo(&path)?, &hash))
        .await
        .map_err(|e| format!("git task: {e}"))?
}

#[tauri::command]
pub async fn git_restore_file(
    app: AppHandle,
    hash: String,
    path: String,
) -> Result<GitStatus, GitError> {
    let root = repo_dir(&app)?;
    let rel = safe_rel_path(&path)?;
    let gate = app.state::<crate::AppState>().repo_gate.clone();
    let status = tokio::task::spawn_blocking(move || {
        let repo = open_repo(&root)?;
        restore_file(&repo, &hash, &rel, &gate)
    })
    .await
    .map_err(|e| format!("git task: {e}"))??;
    emit_status(&app, &status)?;
    Ok(status)
}

#[tauri::command]
pub async fn git_revert_commit(app: AppHandle, hash: String) -> Result<GitStatus, GitError> {
    let root = repo_dir(&app)?;
    let gate = app.state::<crate::AppState>().repo_gate.clone();
    let status = tokio::task::spawn_blocking(move || {
        let repo = open_repo(&root)?;
        revert_commit(&repo, &hash, &gate)
    })
    .await
    .map_err(|e| format!("git task: {e}"))??;
    emit_status(&app, &status)?;
    Ok(status)
}

#[tauri::command]
pub async fn git_file_log(app: AppHandle, path: String) -> Result<Vec<CommitEntry>, GitError> {
    let root = repo_dir(&app)?;
    let rel = safe_rel_path(&path)?;
    tokio::task::spawn_blocking(move || file_log(&open_repo(&root)?, &rel))
        .await
        .map_err(|e| format!("git task: {e}"))?
}

#[tauri::command]
pub async fn git_file_diff(
    app: AppHandle,
    hash: String,
    path: String,
) -> Result<FileDiff, GitError> {
    let root = repo_dir(&app)?;
    let rel = safe_rel_path(&path)?;
    tokio::task::spawn_blocking(move || file_diff(&open_repo(&root)?, &hash, &rel))
        .await
        .map_err(|e| format!("git task: {e}"))?
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
    let gate = app.state::<crate::AppState>().repo_gate.clone();
    let (status, changes) = tokio::task::spawn_blocking(move || {
        let _gate = gate.write().map_err(|e| e.to_string())?;
        resolve_conflict(&root, &path, resolution)
    })
    .await
    .map_err(|e| format!("git task: {e}"))??;
    if !changes.is_empty() {
        let _ = app.emit("pull-changed", &changes);
    }
    emit_status(&app, &status)
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
    use crate::test_util::assert_waits_for_gate;

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
    fn last_push_roundtrips_through_repo_config() {
        let dir = tmp("last-push");
        let repo = Repository::init(&dir).unwrap();
        assert_eq!(last_push_secs(&repo), None);
        record_push(&repo);
        assert!(last_push_secs(&repo).unwrap() > 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn clone_repo_copies_a_bare_remote() {
        let root = tmp("clone-repo");
        let remote_path = root.join("remote.git");
        Repository::init_bare(&remote_path).unwrap();
        let seed = Repository::clone(remote_path.to_str().unwrap(), root.join("seed")).unwrap();
        commit_file(&seed, "a.md", "hello");
        push_current(&seed);
        drop(seed);

        let target = root.join("clone");
        clone_repo(remote_path.to_str().unwrap(), &target, None).unwrap();

        assert!(target.join("a.md").exists());
        assert_eq!(log_entries(&open_repo(&target).unwrap()).unwrap().len(), 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn clone_repo_rejects_a_non_empty_folder() {
        let root = tmp("clone-taken");
        let remote_path = root.join("remote.git");
        Repository::init_bare(&remote_path).unwrap();
        let target = root.join("taken");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("keep.txt"), "mine").unwrap();

        let err = clone_repo(remote_path.to_str().unwrap(), &target, None).unwrap_err();

        assert!(err.to_string().contains("not an empty"), "unexpected: {err}");
        assert_eq!(std::fs::read_to_string(target.join("keep.txt")).unwrap(), "mine");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn move_clone_moves_the_clone_and_keeps_the_history() {
        let root = tmp("move-clone");
        let from = root.join("notes");
        let repo = Repository::init(&from).unwrap();
        commit_file(&repo, "a.md", "1");
        commit_file(&repo, "b.md", "2");
        drop(repo);

        let parent = root.join("elsewhere");
        std::fs::create_dir_all(&parent).unwrap();
        let to = parent.join("notes");
        move_clone(&from, &to).unwrap();

        assert!(!from.exists());
        assert!(to.join(".git").exists());
        assert_eq!(log_entries(&open_repo(&to).unwrap()).unwrap().len(), 2);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn move_clone_refuses_an_existing_target() {
        let root = tmp("move-taken");
        let from = root.join("notes");
        let repo = Repository::init(&from).unwrap();
        commit_file(&repo, "a.md", "1");
        drop(repo);

        let to = root.join("taken");
        std::fs::create_dir_all(&to).unwrap();
        std::fs::write(to.join("keep.txt"), "mine").unwrap();

        let err = move_clone(&from, &to).unwrap_err();

        assert!(err.to_string().contains("already exists"), "unexpected: {err}");
        assert!(from.join("a.md").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn stage_copy_swaps_in_a_verified_copy() {
        let root = tmp("stage-copy");
        let from = root.join("notes");
        let repo = Repository::init(&from).unwrap();
        commit_file(&repo, "a.md", "1");
        drop(repo);

        let to = root.join("elsewhere").join("notes");
        std::fs::create_dir_all(to.parent().unwrap()).unwrap();
        stage_copy(&from, &to).unwrap();

        assert!(to.join("a.md").exists());
        // Removing the source is the caller's job, after the swap.
        assert!(from.join("a.md").exists());
        let leftovers: Vec<String> = std::fs::read_dir(to.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.starts_with('.'))
            .collect();
        assert!(leftovers.is_empty(), "left behind: {leftovers:?}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn move_clone_keeps_uncommitted_changes() {
        let root = tmp("move-dirty");
        let from = root.join("notes");
        let repo = Repository::init(&from).unwrap();
        commit_file(&repo, "a.md", "committed");
        std::fs::write(from.join("a.md"), "dirty").unwrap();
        drop(repo);

        let to = root.join("moved").join("notes");
        std::fs::create_dir_all(to.parent().unwrap()).unwrap();
        move_clone(&from, &to).unwrap();

        assert_eq!(std::fs::read_to_string(to.join("a.md")).unwrap(), "dirty");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn copy_verification_rejects_a_truncated_copy() {
        let root = tmp("verify-copy");
        let from = root.join("notes");
        let repo = Repository::init(&from).unwrap();
        commit_file(&repo, "a.md", "1");
        commit_file(&repo, "b.md", "2");
        drop(repo);

        let to = root.join("copy");
        copy_tree(&from, &to).unwrap();
        verify_copy(&from, &to).unwrap();

        std::fs::remove_file(to.join("b.md")).unwrap();
        let err = verify_copy(&from, &to).unwrap_err();
        assert!(err.to_string().contains("missing files"), "unexpected: {err}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn import_refuses_a_folder_that_already_holds_a_clone() {
        let root = tmp("import-taken");
        Repository::init(root.join("notes")).unwrap();

        let err = fresh_clone_target(&root, "notes").unwrap_err();
        assert!(err.to_string().contains("already cloned"), "unexpected: {err}");
        assert_eq!(fresh_clone_target(&root, "other").unwrap(), root.join("other"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn iso8601_matches_known_dates() {
        assert_eq!(iso8601(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso8601(1_700_000_000), "2023-11-14T22:13:20Z");
    }

    #[test]
    fn clone_dir_prefers_documents_then_home() {
        let root = tmp("clone-dir");
        let docs = root.join("Documenti");
        let home = root.join("home");
        std::fs::create_dir_all(&docs).unwrap();
        std::fs::create_dir_all(&home).unwrap();
        let fallback = root.join("hidden");
        assert_eq!(
            pick_clone_dir(Some(docs.clone()), Some(home.clone()), fallback.clone()),
            docs
        );
        // A configured but missing documents folder must not be used.
        assert_eq!(
            pick_clone_dir(Some(root.join("missing")), Some(home.clone()), fallback.clone()),
            home
        );
        assert_eq!(pick_clone_dir(None, None, fallback.clone()), fallback);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn clone_target_requires_an_existing_absolute_folder() {
        let parent = tmp("clone-target");
        assert!(clone_target(Path::new("relative"), "notes").is_err());
        assert!(clone_target(&parent.join("missing"), "notes").is_err());
        assert_eq!(clone_target(&parent, "notes").unwrap(), parent.join("notes"));
        let _ = std::fs::remove_dir_all(&parent);
    }

    #[test]
    fn copy_tree_copies_files_and_nested_folders() {
        let src = tmp("copy-src");
        let dst = tmp("copy-dst");
        std::fs::write(src.join("a.md"), "a").unwrap();
        std::fs::create_dir_all(src.join("sub")).unwrap();
        std::fs::write(src.join("sub/b.md"), "b").unwrap();
        copy_tree(&src, &dst).unwrap();
        assert!(dst.join("a.md").exists());
        assert!(dst.join("sub/b.md").exists());
        let _ = std::fs::remove_dir_all(&src);
        let _ = std::fs::remove_dir_all(&dst);
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

        let (status, _) = pull_repo(&a, None, &std::sync::RwLock::new(())).unwrap();
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
        let (status, _) = pull_repo(&a, None, &std::sync::RwLock::new(())).unwrap();
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
        let (status, _) = pull_repo(&a, None, &std::sync::RwLock::new(())).unwrap();
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
    fn resolve_conflict_reports_the_resolved_path() {
        let root = tmp("resolve-report");
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
        let (status, _) = pull_repo(&a, None, &std::sync::RwLock::new(())).unwrap();
        assert_eq!(status.conflicts.len(), 1);

        let (status, changes) =
            resolve_conflict(&a_path, "note.md", Resolution::AcceptOurs).unwrap();
        assert!(!status.conflicts.iter().any(|c| c.path == "note.md"));
        assert!(changes
            .iter()
            .any(|change| change.path == "note.md" && change.status == "M"));
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
        let (status, _) = pull_repo(&a, None, &std::sync::RwLock::new(())).unwrap();
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
    fn log_entries_lists_newest_first() {
        let root = tmp("log");
        let repo = Repository::init(&root).unwrap();
        commit_file(&repo, "a.txt", "1");
        commit_file(&repo, "b.txt", "2");

        let entries = log_entries(&repo).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].hash, repo.head().unwrap().target().unwrap().to_string());
        assert_eq!(entries[0].short_hash.len(), 7);
        assert_eq!(entries[0].files_changed, 1);
        assert!(entries[0].timestamp > 0);
        assert_eq!(entries[1].files_changed, 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn file_log_only_lists_touching_commits() {
        let root = tmp("filelog");
        let repo = Repository::init(&root).unwrap();
        commit_file(&repo, "note.md", "one");
        let first = repo.head().unwrap().target().unwrap().to_string();
        commit_file(&repo, "other.txt", "two");

        let entries = file_log(&repo, Path::new("note.md")).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].hash, first);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn file_diff_returns_parent_and_commit_versions() {
        let root = tmp("filediff");
        let repo = Repository::init(&root).unwrap();
        commit_file(&repo, "note.md", "one");
        let first = repo.head().unwrap().target().unwrap().to_string();
        commit_file(&repo, "note.md", "two");
        let head = repo.head().unwrap().target().unwrap().to_string();

        let diff = file_diff(&repo, &head, Path::new("note.md")).unwrap();
        assert_eq!(diff.original, "one");
        assert_eq!(diff.modified, "two");

        let root_diff = file_diff(&repo, &first, Path::new("note.md")).unwrap();
        assert_eq!(root_diff.original, "");
        assert_eq!(root_diff.modified, "one");

        let missing = file_diff(&repo, &head, Path::new("nope.txt")).unwrap();
        assert_eq!(missing.original, "");
        assert_eq!(missing.modified, "");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn commit_files_reports_delta_codes() {
        let root = tmp("commit-files");
        let repo = Repository::init(&root).unwrap();
        commit_file(&repo, "keep.txt", "keep");
        commit_file(&repo, "gone.txt", "gone");
        std::fs::write(root.join("keep.txt"), "edited").unwrap();
        std::fs::remove_file(root.join("gone.txt")).unwrap();
        std::fs::write(root.join("new.txt"), "new").unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new("keep.txt")).unwrap();
        index.remove_path(Path::new("gone.txt")).unwrap();
        index.add_path(Path::new("new.txt")).unwrap();
        index.write().unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let sig = signature().unwrap();
        let parent = repo.head().unwrap().peel_to_commit().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "test", &tree, &[&parent])
            .unwrap();
        let hash = repo.head().unwrap().target().unwrap().to_string();

        let files: Vec<(String, String)> = commit_files(&repo, &hash)
            .unwrap()
            .into_iter()
            .map(|f| (f.path, f.status))
            .collect();
        assert!(files.contains(&("keep.txt".into(), "M".into())));
        assert!(files.contains(&("gone.txt".into(), "D".into())));
        assert!(files.contains(&("new.txt".into(), "A".into())));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn restore_file_brings_back_old_content() {
        let root = tmp("restore");
        let repo = Repository::init(&root).unwrap();
        let gate = std::sync::RwLock::new(());
        commit_file(&repo, "note.md", "old");
        let first = repo.head().unwrap().target().unwrap().to_string();
        commit_file(&repo, "note.md", "new");
        assert_eq!(std::fs::read_to_string(root.join("note.md")).unwrap(), "new");

        let status = restore_file(&repo, &first, Path::new("note.md"), &gate).unwrap();
        assert_eq!(std::fs::read_to_string(root.join("note.md")).unwrap(), "old");
        assert!(!status.synced);
        assert!(status.changes.iter().any(|c| c.path == "note.md"));

        assert!(restore_file(&repo, &first, Path::new("missing.md"), &gate).is_err());
        // The guard must not outlive the call, error path included.
        assert!(gate.try_write().is_ok());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn revert_commit_inverts_changes() {
        let root = tmp("revert");
        let repo = Repository::init(&root).unwrap();
        let gate = std::sync::RwLock::new(());
        commit_file(&repo, "a.txt", "one");
        let first = repo.head().unwrap().target().unwrap().to_string();
        commit_file(&repo, "a.txt", "two");
        let second = repo.head().unwrap().target().unwrap().to_string();

        let status = revert_commit(&repo, &second, &gate).unwrap();
        assert_eq!(std::fs::read_to_string(root.join("a.txt")).unwrap(), "one");
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        assert_ne!(head.id().to_string(), second);
        assert_eq!(
            head.message(),
            Some(format!("gitpad: revert {}", &second[..7]).as_str())
        );
        assert!(!status.branch.is_empty());

        assert!(revert_commit(&repo, &first, &gate).is_err()); // first commit
        assert!(gate.try_write().is_ok());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn revert_brings_back_a_deleted_folder() {
        let root = tmp("restore-folder");
        Repository::init(&root).unwrap();
        std::fs::create_dir_all(root.join("notes/sub")).unwrap();
        std::fs::write(root.join("notes/a.md"), "a").unwrap();
        std::fs::write(root.join("notes/sub/b.md"), "b").unwrap();
        std::fs::write(root.join("notes/.gitkeep"), "").unwrap();
        commit_all(&root, "seed").unwrap();

        // Exactly what fs_delete does for a folder.
        std::fs::remove_dir_all(root.join("notes")).unwrap();
        commit_all(&root, "gitpad: delete notes").unwrap();
        assert!(!root.join("notes").exists());
        let deletion = {
            let repo = Repository::open(&root).unwrap();
            repo.head().unwrap().target().unwrap().to_string()
        };

        let repo = Repository::open(&root).unwrap();
        let gate = std::sync::RwLock::new(());
        let status = revert_commit(&repo, &deletion, &gate).unwrap();
        assert_eq!(
            std::fs::read_to_string(root.join("notes/a.md")).unwrap(),
            "a"
        );
        assert_eq!(
            std::fs::read_to_string(root.join("notes/sub/b.md")).unwrap(),
            "b"
        );
        assert!(root.join("notes/.gitkeep").is_file());
        assert!(status.changes.is_empty());
        assert!(gate.try_write().is_ok());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn discard_file_holds_and_releases_the_gate() {
        let root = tmp("discard-gate");
        let repo = Repository::init(&root).unwrap();
        let gate = std::sync::RwLock::new(());
        commit_file(&repo, "note.md", "old");
        std::fs::write(root.join("note.md"), "edited").unwrap();

        discard_file(&repo, Path::new("note.md"), &gate).unwrap();
        assert_eq!(std::fs::read_to_string(root.join("note.md")).unwrap(), "old");
        assert!(gate.try_write().is_ok());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn discard_file_waits_for_a_held_gate() {
        let root = tmp("discard-wait");
        {
            let repo = Repository::init(&root).unwrap();
            commit_file(&repo, "note.md", "old");
        }
        std::fs::write(root.join("note.md"), "edited").unwrap();
        let root_task = root.clone();

        assert_waits_for_gate(move |gate| {
            let repo = Repository::open(&root_task).unwrap();
            discard_file(&repo, Path::new("note.md"), &gate).is_ok()
        });
        assert_eq!(std::fs::read_to_string(root.join("note.md")).unwrap(), "old");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn discard_refuses_a_conflicted_path() {
        let root = tmp("discard-conflict");
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
        let (status, _) = pull_repo(&a, None, &std::sync::RwLock::new(())).unwrap();
        assert_eq!(status.conflicts.len(), 1);

        let gate = std::sync::RwLock::new(());
        match discard_file(&a, Path::new("note.md"), &gate) {
            Err(GitError::Other(message)) => assert!(message.contains("merge conflict"), "{message}"),
            other => panic!("unexpected discard result: {other:?}"),
        }
        // The conflict must survive a refused discard.
        assert!(a.index().unwrap().has_conflicts());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn discard_restores_a_staged_deletion() {
        let root = tmp("discard-staged-delete");
        let repo = Repository::init(&root).unwrap();
        commit_file(&repo, "note.md", "old");
        {
            let mut index = repo.index().unwrap();
            index.remove_path(Path::new("note.md")).unwrap();
            index.write().unwrap();
        }
        std::fs::remove_file(root.join("note.md")).unwrap();

        let gate = std::sync::RwLock::new(());
        discard_file(&repo, Path::new("note.md"), &gate).unwrap();
        assert_eq!(std::fs::read_to_string(root.join("note.md")).unwrap(), "old");
        assert!(status_of(&repo).unwrap().changes.is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn discard_ignores_a_missing_untracked_file() {
        let root = tmp("discard-missing");
        let repo = Repository::init(&root).unwrap();
        let gate = std::sync::RwLock::new(());

        discard_file(&repo, Path::new("gone.md"), &gate).unwrap();
        assert!(gate.try_write().is_ok());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn restore_file_waits_for_a_held_gate() {
        let root = tmp("restore-wait");
        let first = {
            let repo = Repository::init(&root).unwrap();
            commit_file(&repo, "note.md", "old");
            let first = repo.head().unwrap().target().unwrap().to_string();
            commit_file(&repo, "note.md", "new");
            first
        };
        let root_task = root.clone();

        assert_waits_for_gate(move |gate| {
            let repo = Repository::open(&root_task).unwrap();
            restore_file(&repo, &first, Path::new("note.md"), &gate).is_ok()
        });
        assert_eq!(std::fs::read_to_string(root.join("note.md")).unwrap(), "old");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn revert_commit_waits_for_a_held_gate() {
        let root = tmp("revert-wait");
        let second = {
            let repo = Repository::init(&root).unwrap();
            commit_file(&repo, "a.txt", "one");
            commit_file(&repo, "a.txt", "two");
            repo.head().unwrap().target().unwrap().to_string()
        };
        let root_task = root.clone();

        assert_waits_for_gate(move |gate| {
            let repo = Repository::open(&root_task).unwrap();
            revert_commit(&repo, &second, &gate).is_ok()
        });
        assert_eq!(std::fs::read_to_string(root.join("a.txt")).unwrap(), "one");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn push_only_releases_the_gate_after_committing() {
        let root = tmp("push-only-gate");
        let remote_path = root.join("remote.git");
        Repository::init_bare(&remote_path).unwrap();
        let a_path = root.join("a");
        let a = Repository::clone(remote_path.to_str().unwrap(), &a_path).unwrap();
        commit_file(&a, "a.txt", "1");
        push_current(&a);
        commit_file(&a, "b.txt", "2");
        drop(a);

        let gate = std::sync::RwLock::new(());
        let status = push_only_blocking(&a_path, None, None, GateWait::Block(&gate)).unwrap();
        assert_eq!(status.ahead, 0);
        assert!(gate.try_write().is_ok());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn push_only_releases_the_gate_after_a_push_error() {
        let root = tmp("push-only-gate-error");
        let remote_path = root.join("remote.git");
        Repository::init_bare(&remote_path).unwrap();
        let a_path = root.join("a");
        let a = Repository::clone(remote_path.to_str().unwrap(), &a_path).unwrap();
        commit_file(&a, "a.txt", "1");
        push_current(&a);
        commit_file(&a, "b.txt", "2");
        drop(a);
        // The remote disappears: the commit succeeds, the push does not.
        std::fs::remove_dir_all(&remote_path).unwrap();

        let gate = std::sync::RwLock::new(());
        assert!(push_only_blocking(&a_path, None, None, GateWait::Block(&gate)).is_err());
        assert!(gate.try_write().is_ok());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn push_only_waits_for_a_held_gate() {
        let root = tmp("push-only-wait");
        let remote_path = root.join("remote.git");
        Repository::init_bare(&remote_path).unwrap();
        let a_path = root.join("a");
        let a = Repository::clone(remote_path.to_str().unwrap(), &a_path).unwrap();
        commit_file(&a, "a.txt", "1");
        push_current(&a);
        commit_file(&a, "b.txt", "2");
        drop(a);

        let gate = std::sync::Arc::new(std::sync::RwLock::new(()));
        let held = gate.write().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let task_gate = gate.clone();
        let task_path = a_path.clone();
        let task = std::thread::spawn(move || {
            let ok =
                push_only_blocking(&task_path, None, None, GateWait::Block(task_gate.as_ref()))
                    .is_ok();
            let _ = tx.send(ok);
        });

        assert!(rx
            .recv_timeout(std::time::Duration::from_millis(200))
            .is_err());
        drop(held);
        assert!(rx.recv_timeout(std::time::Duration::from_secs(10)).unwrap());
        task.join().unwrap();
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn push_only_skips_commit_when_the_gate_never_frees() {
        let root = tmp("push-only-until-timeout");
        let remote_path = root.join("remote.git");
        Repository::init_bare(&remote_path).unwrap();
        let a_path = root.join("a");
        let a = Repository::clone(remote_path.to_str().unwrap(), &a_path).unwrap();
        commit_file(&a, "a.txt", "one");
        push_current(&a);
        let before = a.head().unwrap().target().unwrap();
        drop(a);
        std::fs::write(a_path.join("a.txt"), "edited").unwrap();

        let gate = std::sync::Arc::new(std::sync::RwLock::new(()));
        let held = gate.write().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let task_gate = gate.clone();
        let task_path = a_path.clone();
        let started = std::time::Instant::now();
        let task = std::thread::spawn(move || {
            let result = push_only_blocking(
                &task_path,
                None,
                None,
                GateWait::Until(task_gate.as_ref(), std::time::Duration::from_millis(100)),
            );
            let _ = tx.send(result);
        });

        let result = rx
            .recv_timeout(std::time::Duration::from_millis(500))
            .expect("Until must give up instead of waiting for the gate");
        assert!(started.elapsed() < std::time::Duration::from_millis(500));
        assert!(result.is_ok());
        task.join().unwrap();
        assert_eq!(std::fs::read_to_string(a_path.join("a.txt")).unwrap(), "edited");
        let repo = Repository::open(&a_path).unwrap();
        assert_eq!(repo.head().unwrap().target().unwrap(), before);
        drop(held);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn push_only_until_commits_and_pushes_when_the_gate_is_free() {
        let root = tmp("push-only-until-free");
        let remote_path = root.join("remote.git");
        Repository::init_bare(&remote_path).unwrap();
        let a_path = root.join("a");
        let a = Repository::clone(remote_path.to_str().unwrap(), &a_path).unwrap();
        commit_file(&a, "a.txt", "one");
        push_current(&a);
        drop(a);
        std::fs::write(a_path.join("a.txt"), "edited").unwrap();

        let gate = std::sync::RwLock::new(());
        let status = push_only_blocking(
            &a_path,
            None,
            None,
            GateWait::Until(&gate, std::time::Duration::from_millis(100)),
        )
        .unwrap();
        assert_eq!(status.ahead, 0);
        assert!(status.changes.is_empty());
        let repo = Repository::open(&a_path).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        assert!(head.message().unwrap().starts_with("gitpad: sync"));
        assert!(gate.try_write().is_ok());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn push_only_until_recovers_a_poisoned_gate() {
        let root = tmp("push-only-until-poisoned");
        let remote_path = root.join("remote.git");
        Repository::init_bare(&remote_path).unwrap();
        let a_path = root.join("a");
        let a = Repository::clone(remote_path.to_str().unwrap(), &a_path).unwrap();
        commit_file(&a, "a.txt", "one");
        push_current(&a);
        drop(a);
        std::fs::write(a_path.join("a.txt"), "edited").unwrap();

        let gate = std::sync::Arc::new(std::sync::RwLock::new(()));
        let task_gate = gate.clone();
        let _ = std::thread::spawn(move || {
            let _guard = task_gate.write().unwrap();
            panic!("poison the gate");
        })
        .join();
        assert!(gate.is_poisoned());

        let status = push_only_blocking(
            &a_path,
            None,
            None,
            GateWait::Until(gate.as_ref(), std::time::Duration::from_millis(100)),
        )
        .unwrap();
        assert_eq!(status.ahead, 0);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn try_write_until_releases_the_guard() {
        let gate = std::sync::RwLock::new(());
        let guard = try_write_until(&gate, std::time::Duration::from_millis(100))
            .expect("a free gate must be acquired");
        assert!(gate.try_write().is_err());
        drop(guard);
        assert!(gate.try_write().is_ok());
    }

    #[test]
    fn status_of_reports_branch_and_changes() {
        let root = tmp("status-changes");
        let repo = Repository::init(&root).unwrap();
        commit_file(&repo, "keep.md", "keep");
        commit_file(&repo, "gone.md", "gone");
        std::fs::write(root.join("keep.md"), "edited").unwrap();
        std::fs::write(root.join("new.md"), "new").unwrap();
        std::fs::remove_file(root.join("gone.md")).unwrap();

        let status = status_of(&repo).unwrap();
        let changes: Vec<(String, String)> = status
            .changes
            .iter()
            .map(|c| (c.path.clone(), c.status.clone()))
            .collect();
        assert!(changes.contains(&("keep.md".into(), "M".into())));
        assert!(changes.contains(&("new.md".into(), "A".into())));
        assert!(changes.contains(&("gone.md".into(), "D".into())));
        assert_eq!(status.modified_files.len(), 3);
        assert!(!status.branch.is_empty());
        assert!(!status.synced);
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

    #[test]
    fn fast_forward_pull_reports_changed_paths() {
        let root = tmp("ff-changes");
        let remote_path = root.join("remote.git");
        Repository::init_bare(&remote_path).unwrap();

        let a_path = root.join("a");
        let a = Repository::clone(remote_path.to_str().unwrap(), &a_path).unwrap();
        commit_file(&a, "a.txt", "1");
        push_current(&a);

        let b = Repository::clone(remote_path.to_str().unwrap(), root.join("b")).unwrap();
        commit_file(&b, "a.txt", "2");
        push_current(&b);

        let (status, changes) = pull_repo(&a, None, &std::sync::RwLock::new(())).unwrap();
        assert!(status.conflicts.is_empty());
        assert!(changes
            .iter()
            .any(|change| change.path == "a.txt" && change.status == "M"));
        assert_eq!(std::fs::read_to_string(a_path.join("a.txt")).unwrap(), "2");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn pull_reports_deleted_paths() {
        let root = tmp("ff-deleted");
        let remote_path = root.join("remote.git");
        Repository::init_bare(&remote_path).unwrap();

        let a_path = root.join("a");
        let a = Repository::clone(remote_path.to_str().unwrap(), &a_path).unwrap();
        commit_file(&a, "a.txt", "1");
        push_current(&a);

        let b_path = root.join("b");
        let b = Repository::clone(remote_path.to_str().unwrap(), &b_path).unwrap();
        std::fs::remove_file(b_path.join("a.txt")).unwrap();
        commit_all(&b_path, "gitpad: delete a.txt").unwrap();
        push_current(&b);

        let (status, changes) = pull_repo(&a, None, &std::sync::RwLock::new(())).unwrap();
        assert!(status.conflicts.is_empty());
        assert!(changes
            .iter()
            .any(|change| change.path == "a.txt" && change.status == "D"));
        assert!(!a_path.join("a.txt").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn push_success_returns_pull_changes_and_no_error() {
        let root = tmp("push-ok");
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
        drop(a);
        let (status, changes, push_error) = push_blocking(
            &a_path,
            None,
            None,
            std::sync::Arc::new(std::sync::RwLock::new(())),
        )
        .unwrap();

        assert!(push_error.is_none());
        assert!(status.conflicts.is_empty());
        assert_eq!(status.ahead, 0);
        assert!(changes
            .iter()
            .any(|change| change.path == "b.txt" && change.status == "A"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn push_failure_still_returns_pull_changes() {
        // Root ignores file permissions: the read-only remote would accept.
        if std::process::Command::new("id")
            .arg("-u")
            .output()
            .is_ok_and(|out| out.stdout.first() == Some(&b'0'))
        {
            eprintln!("skipping: root can write to a read-only remote");
            return;
        }
        let root = tmp("push-rejected");
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

        // A read-only remote rejects the push; libgit2 does not run local hooks.
        let chmod = |mode: &str| {
            let ok = std::process::Command::new("chmod")
                .arg("-R")
                .arg(mode)
                .arg(&remote_path)
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            assert!(ok, "chmod {mode} failed");
        };
        chmod("a-w");

        drop(a);
        let result = push_blocking(
            &a_path,
            None,
            None,
            std::sync::Arc::new(std::sync::RwLock::new(())),
        );
        chmod("u+w");
        let (status, changes, push_error) = result.unwrap();

        let error = push_error.expect("the read-only remote must reject the push");
        assert!(error.to_string().contains("git push"));
        assert!(status.ahead > 0);
        assert!(changes
            .iter()
            .any(|change| change.path == "b.txt" && change.status == "A"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn conflicted_merge_reports_every_rewritten_path() {
        let root = tmp("conflict-changes");
        let remote_path = root.join("remote.git");
        Repository::init_bare(&remote_path).unwrap();

        let a_path = root.join("a");
        let a = Repository::clone(remote_path.to_str().unwrap(), &a_path).unwrap();
        commit_file(&a, "a.md", "a1");
        commit_file(&a, "b.md", "b1");
        push_current(&a);

        let b = Repository::clone(remote_path.to_str().unwrap(), root.join("b")).unwrap();
        commit_file(&b, "a.md", "a2");
        commit_file(&b, "b.md", "theirs");
        push_current(&b);

        commit_file(&a, "b.md", "ours");
        let (status, changes) = pull_repo(&a, None, &std::sync::RwLock::new(())).unwrap();

        assert_eq!(status.conflicts.len(), 1);
        assert!(changes.iter().any(|change| change.path == "a.md"));
        assert!(changes.iter().any(|change| change.path == "b.md"));
        assert_eq!(std::fs::read_to_string(a_path.join("a.md")).unwrap(), "a2");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn remote_rename_arrives_as_delete_and_add() {
        let root = tmp("rename-changes");
        let remote_path = root.join("remote.git");
        Repository::init_bare(&remote_path).unwrap();

        let a_path = root.join("a");
        let a = Repository::clone(remote_path.to_str().unwrap(), &a_path).unwrap();
        commit_file(&a, "old.txt", "x");
        push_current(&a);

        let b_path = root.join("b");
        let b = Repository::clone(remote_path.to_str().unwrap(), &b_path).unwrap();
        std::fs::rename(b_path.join("old.txt"), b_path.join("new.txt")).unwrap();
        commit_all(&b_path, "gitpad: rename").unwrap();
        push_current(&b);

        let (status, changes) = pull_repo(&a, None, &std::sync::RwLock::new(())).unwrap();

        assert!(status.conflicts.is_empty());
        assert!(changes
            .iter()
            .any(|change| change.path == "old.txt" && change.status == "D"));
        assert!(changes
            .iter()
            .any(|change| change.path == "new.txt" && change.status == "A"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn use_local_repo_initializes_a_plain_folder() {
        let root = tmp("local-plain");
        std::fs::write(root.join("note.md"), "hello").unwrap();

        let (path, initialized, remote) = use_local_repo(&root).unwrap();

        assert!(initialized);
        assert!(!remote);
        assert!(path.join(".git").exists());
        // The folder content survives: init never rewrites the worktree.
        assert_eq!(std::fs::read_to_string(path.join("note.md")).unwrap(), "hello");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn use_local_repo_reopens_an_existing_repository() {
        let root = tmp("local-existing");
        Repository::init(&root).unwrap();

        let (_, initialized, remote) = use_local_repo(&root).unwrap();

        assert!(!initialized);
        assert!(!remote);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn push_without_origin_commits_locally() {
        let root = tmp("local-push");
        Repository::init(&root).unwrap();
        std::fs::write(root.join("a.md"), "one").unwrap();

        let (status, _, error) = push_blocking(
            &root,
            Some("gitpad: update a.md".into()),
            None,
            std::sync::Arc::new(std::sync::RwLock::new(())),
        )
        .unwrap();

        // No remote is not an error: the commit is the whole sync.
        assert!(error.is_none());
        assert!(status.changes.is_empty());
        let reopened = Repository::open(&root).unwrap();
        let head = reopened.head().unwrap();
        let commit = head.peel_to_commit().unwrap();
        assert_eq!(commit.message(), Some("gitpad: update a.md"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn pull_without_origin_does_not_fetch() {
        let root = tmp("local-pull");
        let repo = Repository::init(&root).unwrap();
        commit_file(&repo, "a.md", "one");

        let (status, changes) = pull_repo(&repo, None, &std::sync::RwLock::new(())).unwrap();

        assert!(status.conflicts.is_empty());
        assert!(changes.is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }
}
