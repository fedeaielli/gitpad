use crate::auth;
use git2::{
    build::{CheckoutBuilder, RepoBuilder}, Commit, Diff, FetchOptions, IndexAddOption, Oid,
    PushOptions, Repository, ResetType, Signature, Sort, StatusOptions,
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

fn restore_file(repo: &Repository, hash: &str, rel: &Path) -> Result<GitStatus, GitError> {
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

fn revert_commit(repo: &Repository, hash: &str) -> Result<GitStatus, GitError> {
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
    push_refs(&repo, token)?;
    status_of(&repo)
}

/// Commit and push without pulling first (used by the push-only button).
pub fn push_only_blocking(
    path: &Path,
    message: Option<String>,
    token: Option<String>,
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
    commit_pending(&repo, message.as_deref())?;
    push_refs(&repo, token)?;
    status_of(&repo)
}

fn push_refs(repo: &Repository, token: Option<String>) -> Result<(), GitError> {
    let branch = current_branch(repo)?;
    let refspec = format!("refs/heads/{branch}:refs/heads/{branch}");
    let mut remote = repo.find_remote("origin").map_err(git_err)?;
    let mut opts = PushOptions::new();
    opts.remote_callbacks(callbacks(token));
    remote
        .push(&[refspec.as_str()], Some(&mut opts))
        .map_err(|e| format!("git push: {}", e.message()))?;
    Ok(())
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
pub async fn git_push(
    app: AppHandle,
    message: Option<String>,
    pull: Option<bool>,
) -> Result<GitStatus, GitError> {
    let path = repo_dir(&app)?;
    let status = tokio::task::spawn_blocking(move || {
        // Keyring reads are blocking: keep them off the async runtime.
        let token = auth::token().ok();
        if pull == Some(false) {
            push_only_blocking(&path, message, token)
        } else {
            push_blocking(&path, message, token)
        }
    })
        .await
        .map_err(|e| format!("git task: {e}"))??;
    emit_status(&app, &status)?;
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

fn discard_file(repo: &Repository, rel: &Path) -> Result<(), GitError> {
    let tracked = repo
        .index()
        .map_err(git_err)?
        .get_path(rel, 0)
        .is_some();
    if tracked {
        let mut checkout = CheckoutBuilder::new();
        checkout.path(rel).force();
        repo.checkout_head(Some(&mut checkout)).map_err(git_err)
    } else {
        let workdir = repo
            .workdir()
            .ok_or_else(|| GitError::IoError("bare repository".into()))?;
        std::fs::remove_file(workdir.join(rel))
            .map_err(|e| GitError::IoError(format!("remove {}: {e}", rel.display())))
    }
}

#[tauri::command]
pub async fn git_discard_file(app: AppHandle, path: String) -> Result<GitStatus, GitError> {
    let root = repo_dir(&app)?;
    let rel = safe_rel_path(&path)?;
    let status = tokio::task::spawn_blocking(move || {
        let repo = open_repo(&root)?;
        discard_file(&repo, &rel)?;
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
    let status = tokio::task::spawn_blocking(move || {
        let repo = open_repo(&root)?;
        restore_file(&repo, &hash, &rel)
    })
    .await
    .map_err(|e| format!("git task: {e}"))??;
    emit_status(&app, &status)?;
    Ok(status)
}

#[tauri::command]
pub async fn git_revert_commit(app: AppHandle, hash: String) -> Result<GitStatus, GitError> {
    let root = repo_dir(&app)?;
    let status = tokio::task::spawn_blocking(move || {
        let repo = open_repo(&root)?;
        revert_commit(&repo, &hash)
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
        commit_file(&repo, "note.md", "old");
        let first = repo.head().unwrap().target().unwrap().to_string();
        commit_file(&repo, "note.md", "new");
        assert_eq!(std::fs::read_to_string(root.join("note.md")).unwrap(), "new");

        let status = restore_file(&repo, &first, Path::new("note.md")).unwrap();
        assert_eq!(std::fs::read_to_string(root.join("note.md")).unwrap(), "old");
        assert!(!status.synced);
        assert!(status.changes.iter().any(|c| c.path == "note.md"));

        assert!(restore_file(&repo, &first, Path::new("missing.md")).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn revert_commit_inverts_changes() {
        let root = tmp("revert");
        let repo = Repository::init(&root).unwrap();
        commit_file(&repo, "a.txt", "one");
        let first = repo.head().unwrap().target().unwrap().to_string();
        commit_file(&repo, "a.txt", "two");
        let second = repo.head().unwrap().target().unwrap().to_string();

        let status = revert_commit(&repo, &second).unwrap();
        assert_eq!(std::fs::read_to_string(root.join("a.txt")).unwrap(), "one");
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        assert_ne!(head.id().to_string(), second);
        assert_eq!(
            head.message(),
            Some(format!("gitpad: revert {}", &second[..7]).as_str())
        );
        assert!(!status.branch.is_empty());

        assert!(revert_commit(&repo, &first).is_err()); // first commit
        let _ = std::fs::remove_dir_all(&root);
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
}
