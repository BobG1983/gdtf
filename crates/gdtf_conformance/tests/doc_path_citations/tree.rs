//! Repo layout helpers for the cited-path guard.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

// The two trees whose prose points readers at a source file.
pub(crate) const SCANNED_ROOTS: &[&str] = &["docs", ".claude"];

// Gitignored files the filesystem fallback must not read. `run-state.md` carries
// ticket text quoting stale paths while a build is running.
const UNTRACKED: &[&str] = &[".claude/run-state.md", ".claude/settings.local.json"];

/// The workspace root, or `None` when no `Cargo.lock` or `[workspace]` manifest sits
/// above this crate.
pub(crate) fn repo_root() -> Option<PathBuf> {
    let root = cobalt_ron_assets::workspace_root()?;
    Some(root.canonicalize().unwrap_or(root))
}

/// Every file under the scanned roots, as repo-relative paths, sorted.
pub(crate) fn scanned_files(root: &Path) -> Vec<String> {
    let mut files = git_tracked(root).unwrap_or_else(|| read_roots(root));
    files.sort();
    files
}

// Tracked files under the scanned roots, so untracked scratch files are ignored.
// None when git cannot answer, which sends the caller to the filesystem instead.
fn git_tracked(root: &Path) -> Option<Vec<String>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .arg("ls-files")
        .arg("--")
        .args(SCANNED_ROOTS)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8(out.stdout).ok()?;
    Some(text.lines().map(str::to_owned).collect())
}

// Fallback for when git is unavailable: walk each scanned root directly.
fn read_roots(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    for scanned in SCANNED_ROOTS {
        walk(root, Path::new(scanned), &mut found);
    }
    found
}

fn walk(root: &Path, relative: &Path, found: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(root.join(relative)) else {
        return;
    };
    for entry in entries.flatten() {
        let child = relative.join(entry.file_name());
        if entry.path().is_dir() {
            walk(root, &child, found);
        } else {
            let path = child.to_string_lossy().replace('\\', "/");
            if !UNTRACKED.contains(&path.as_str()) {
                found.push(path);
            }
        }
    }
}
