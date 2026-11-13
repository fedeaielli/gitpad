//! Update check against the project's public GitHub releases.

use crate::AppState;
use serde::Deserialize;
use tauri::State;

const API: &str = "https://api.github.com";
const REPO: &str = "fedeaielli/gitpad";
/// The version this build carries.
const CURRENT: &str = env!("CARGO_PKG_VERSION");

#[derive(serde::Serialize)]
pub struct UpdateInfo {
    current: String,
    latest: String,
    url: String,
    newer: bool,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    html_url: String,
}

/// The latest published release. No token: the repository is public.
#[tauri::command]
pub async fn update_check(state: State<'_, AppState>) -> Result<UpdateInfo, String> {
    let res = state
        .http
        .get(format!("{API}/repos/{REPO}/releases/latest"))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("update check: {e}"))?;
    if !res.status().is_success() {
        return Err(format!("update check: {}", res.status()));
    }
    let release: Release = res
        .json()
        .await
        .map_err(|e| format!("update check: {e}"))?;
    let latest = release.tag_name.trim_start_matches('v').to_string();
    Ok(UpdateInfo {
        newer: is_newer(CURRENT, &latest),
        current: CURRENT.to_string(),
        latest,
        url: release.html_url,
    })
}

/// Notes for this build's version, so About never shows a newer release's.
#[tauri::command]
pub fn app_changelog() -> String {
    version_section(include_str!("../../CHANGELOG.md"), CURRENT).to_string()
}

fn is_newer(current: &str, latest: &str) -> bool {
    match (semver::Version::parse(current), semver::Version::parse(latest)) {
        (Ok(current), Ok(latest)) => latest > current,
        // A tag we cannot read is not an update.
        _ => false,
    }
}

/// Section for `version` without its heading, or the newest section when the
/// changelog has no entry yet for this version.
fn version_section<'a>(changelog: &'a str, version: &str) -> &'a str {
    let mut starts = Vec::new();
    let mut offset = 0;
    for line in changelog.split_inclusive('\n') {
        if line.starts_with("## ") {
            starts.push(offset);
        }
        offset += line.len();
    }
    let Some(&first) = starts.first() else {
        return "";
    };
    let pick = starts
        .iter()
        .copied()
        .find(|&start| changelog[start + 3..].split_whitespace().next() == Some(version))
        .unwrap_or(first);
    let end = starts
        .iter()
        .copied()
        .find(|&start| start > pick)
        .unwrap_or(changelog.len());
    // The About tab shows the version itself, so the heading is dropped.
    changelog[pick..end]
        .split_once('\n')
        .map_or("", |(_, body)| body)
        .trim()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "# Changelog\n\n## 1.5.0 — 2026-12-01\n\nNewer notes.\n\n### Added\n- A thing.\n\n## 1.4.0 — 2026-11-13\n\nOlder notes.\n";

    #[test]
    fn picks_the_section_for_this_version() {
        assert_eq!(version_section(SAMPLE, "1.4.0"), "Older notes.");
    }

    #[test]
    fn falls_back_to_the_newest_section() {
        assert_eq!(
            version_section(SAMPLE, "9.9.9"),
            "Newer notes.\n\n### Added\n- A thing."
        );
    }

    #[test]
    fn without_sections_there_is_nothing_to_show() {
        assert_eq!(version_section("# Changelog\n", "1.4.0"), "");
    }

    #[test]
    fn compares_release_tags() {
        assert!(is_newer("1.4.0", "1.5.0"));
        assert!(is_newer("1.4.0", "1.4.1"));
        assert!(!is_newer("1.4.0", "1.4.0"));
        assert!(!is_newer("1.5.0", "1.4.0"));
        assert!(!is_newer("1.4.0", "nightly"));
    }
}
